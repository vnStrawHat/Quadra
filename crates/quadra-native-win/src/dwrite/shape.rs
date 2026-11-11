//! Shaping: a word becomes positioned glyphs. UTF-8 to UTF-16, script runs, fallback fonts
//! for what the face lacks, then the glyphs and their placements.

use std::rc::Rc;

use quadra_text::{FaceId, FontKey, ShapedGlyph};
use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_GLYPH_OFFSET, DWRITE_SCRIPT_ANALYSIS, DWRITE_SHAPING_GLYPH_PROPERTIES,
    DWRITE_SHAPING_TEXT_PROPERTIES, IDWriteNumberSubstitution,
};
use windows::core::{HRESULT, PCWSTR};

use super::DirectWrite;
use super::analysis::FontRun;

/// The buffers shaping reuses from call to call.
#[derive(Default)]
pub(super) struct Scratch {
    units: Vec<u16>,
    /// For each UTF-16 unit, the byte its character starts at. Empty for ASCII: the same.
    unit_to_byte: Vec<u32>,
    /// For each unit, the first glyph of its cluster.
    clusters: Vec<u16>,
    text_props: Vec<DWRITE_SHAPING_TEXT_PROPERTIES>,
    glyphs: Vec<u16>,
    glyph_props: Vec<DWRITE_SHAPING_GLYPH_PROPERTIES>,
    advances: Vec<f32>,
    offsets: Vec<DWRITE_GLYPH_OFFSET>,
    /// For each glyph, the first unit of its cluster.
    first_unit: Vec<u32>,
}

/// Whether a text is all printable ASCII, and if so whether it has a letter.
fn printable_ascii(text: &str) -> Option<bool> {
    text.bytes().all(|b| (0x20..=0x7E).contains(&b)).then(|| text.bytes().any(|b| b.is_ascii_alphabetic()))
}

impl Scratch {
    /// Appends the glyphs of the run just shaped (`count` glyphs for `len` units from unit
    /// `start` of the word) at `pen`, and returns the pen after them.
    fn place(
        &mut self,
        count: usize,
        len: usize,
        face: FaceId,
        start: usize,
        pen: f32,
        out: &mut Vec<ShapedGlyph>,
    ) -> f32 {
        self.first_unit.clear();
        self.first_unit.resize(count, u32::MAX);
        for unit in (0..len).rev() {
            if let Some(first) = self.first_unit.get_mut(self.clusters[unit] as usize) {
                *first = unit as u32;
            }
        }
        let (mut pen, mut current) = (pen, 0);
        for g in 0..count {
            if self.first_unit[g] != u32::MAX {
                current = self.first_unit[g] as usize;
            }
            let unit = start + current;
            out.push(ShapedGlyph {
                face,
                id: self.glyphs[g],
                x: pen + self.offsets[g].advanceOffset,
                y: self.offsets[g].ascenderOffset,
                advance: self.advances[g],
                cluster: self.unit_to_byte.get(unit).copied().unwrap_or(unit as u32),
            });
            pen += self.advances[g];
        }
        pen
    }
}

impl DirectWrite {
    /// Printable ASCII skips script analysis and fallback: its analysis is known (one run,
    /// Latin when it has a letter) and every text font has it.
    pub(super) fn shape_word(&mut self, text: &str, face: FaceId, size: f32) -> Vec<ShapedGlyph> {
        if text.is_empty() || face.0 as usize >= self.faces.len() {
            return Vec::new();
        }
        let mut s = std::mem::take(&mut self.scratch);
        let mut units = std::mem::take(&mut s.units);
        units.clear();
        s.unit_to_byte.clear();
        let mut out = Vec::with_capacity(text.len());
        if let Some(letters) = printable_ascii(text) {
            units.extend(text.bytes().map(u16::from));
            let analysis = self.ascii_analysis()[letters as usize];
            if let Some(count) = self.shape_run(&mut s, &units, &analysis, face, size) {
                s.place(count, units.len(), face, 0, 0.0, &mut out);
            }
        } else {
            for (byte, ch) in text.char_indices() {
                for unit in ch.encode_utf16(&mut [0; 2]) {
                    units.push(*unit);
                    s.unit_to_byte.push(byte as u32);
                }
            }
            let key = self.keys[face.0 as usize].clone();
            let mut pen = 0.0;
            for run in self.analyze(&units) {
                let range = run.start as usize..(run.start + run.len) as usize;
                let fonts = if self.covers(face, &units[range]) {
                    vec![FontRun { start: run.start, len: run.len, family: None }]
                } else {
                    self.map_fonts(&units, run.start, run.len, &key)
                };
                for font in fonts {
                    let family =
                        font.family.map(|family| FontKey { family: Rc::from(family), ..key.clone() });
                    let with =
                        family.and_then(|key| quadra_text::TextBackend::face(self, &key)).unwrap_or(face);
                    let (start, len) = (font.start as usize, font.len as usize);
                    if let Some(count) =
                        self.shape_run(&mut s, &units[start..start + len], &run.analysis, with, size)
                    {
                        pen = s.place(count, len, with, start, pen, &mut out);
                    }
                }
            }
        }
        s.units = units;
        self.scratch = s;
        out
    }

    fn ascii_analysis(&mut self) -> [DWRITE_SCRIPT_ANALYSIS; 2] {
        if self.ascii.is_none() {
            let of =
                |probe: u8| self.analyze(&[probe as u16]).first().map(|r| r.analysis).unwrap_or_default();
            self.ascii = Some([of(b'0'), of(b'a')]);
        }
        self.ascii.unwrap_or_default()
    }

    /// Whether a face has a glyph for every character of a run.
    fn covers(&self, face: FaceId, units: &[u16]) -> bool {
        let points: Vec<u32> =
            char::decode_utf16(units.iter().copied()).map(|c| c.map_or(0xFFFD, u32::from)).collect();
        let mut glyphs = vec![0u16; points.len()];
        let face = &self.faces[face.0 as usize].face;
        // SAFETY: both buffers hold `points.len()` items, as the call is told.
        let done = unsafe { face.GetGlyphIndices(points.as_ptr(), points.len() as u32, glyphs.as_mut_ptr()) };
        done.is_ok() && glyphs.iter().all(|&g| g != 0)
    }

    /// Shapes a run in one script with one face into the scratch buffers, and returns the
    /// number of glyphs.
    fn shape_run(
        &self,
        s: &mut Scratch,
        units: &[u16],
        analysis: &DWRITE_SCRIPT_ANALYSIS,
        face: FaceId,
        size: f32,
    ) -> Option<usize> {
        let (len, face) = (units.len(), &self.faces[face.0 as usize].face);
        if len == 0 {
            return None;
        }
        let text = PCWSTR(units.as_ptr());
        let no_numbers = None::<&IDWriteNumberSubstitution>;
        let too_small = HRESULT::from_win32(ERROR_INSUFFICIENT_BUFFER.0);
        s.clusters.resize(len, 0);
        s.text_props.resize(len, Default::default());
        let (mut max, mut count) = (len * 3 / 2 + 16, 0u32);
        loop {
            s.glyphs.resize(max, 0);
            s.glyph_props.resize(max, Default::default());
            // SAFETY: the text is `len` units; the cluster and text buffers hold `len` items
            // and the glyph buffers `max`, as the call is told.
            let done = unsafe {
                self.analyzer.GetGlyphs(
                    text,
                    len as u32,
                    face,
                    false,
                    false,
                    analysis,
                    &self.locale,
                    no_numbers,
                    None,
                    None,
                    0,
                    max as u32,
                    s.clusters.as_mut_ptr(),
                    s.text_props.as_mut_ptr(),
                    s.glyphs.as_mut_ptr(),
                    s.glyph_props.as_mut_ptr(),
                    &mut count,
                )
            };
            match done {
                Ok(()) => break,
                Err(e) if e.code() == too_small && max < 1 << 16 => max *= 2,
                Err(_) => return None,
            }
        }
        let count = count as usize;
        s.advances.resize(count, 0.0);
        s.offsets.resize(count, Default::default());
        // SAFETY: the buffers are those `GetGlyphs` filled, and the two out buffers hold
        // `count` items.
        unsafe {
            self.analyzer.GetGlyphPlacements(
                text,
                s.clusters.as_ptr(),
                s.text_props.as_mut_ptr(),
                len as u32,
                s.glyphs.as_ptr(),
                s.glyph_props.as_ptr(),
                count as u32,
                face,
                size,
                false,
                false,
                analysis,
                &self.locale,
                None,
                None,
                0,
                s.advances.as_mut_ptr(),
                s.offsets.as_mut_ptr(),
            )
        }
        .ok()?;
        Some(count)
    }
}

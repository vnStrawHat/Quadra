//! Shaping a word (specs/08): a line of Core Text. Its runs come in the order they are
//! drawn, left to right, and each names the font it is drawn in: the face asked for, or the
//! one fallback chose for what that face lacks.

use std::ptr::NonNull;

use objc2_core_foundation::{
    CFAttributedString, CFDictionary, CFIndex, CFRange, CFString, CFType, CGPoint, CGSize,
};
use objc2_core_text::{CTFont, CTLine, CTRun, kCTFontAttributeName};
use quadra_text::{FaceId, ShapedGlyph, next_grapheme};

use super::CoreText;

/// For each UTF-16 unit of a text, the byte its grapheme begins at: Core Text counts in
/// units, and a mark is of the cluster of its base.
fn clusters(text: &str) -> Vec<u32> {
    let mut out = Vec::with_capacity(text.len());
    let mut at = 0;
    while at < text.len() {
        let next = next_grapheme(text, at).max(at + 1);
        let units: usize = text[at..next].chars().map(char::len_utf16).sum();
        out.extend(std::iter::repeat_n(at as u32, units));
        at = next;
    }
    out
}

impl CoreText {
    pub(super) fn shape_word(&mut self, text: &str, face: FaceId, size: f32) -> Vec<ShapedGlyph> {
        let mut out = Vec::new();
        if text.is_empty() {
            return out;
        }
        let Some(font) = self.at(face, size) else { return out };
        let (cluster_of, string) = (clusters(text), CFString::from_str(text));
        let font_value: &CFType = &font;
        // The whole of a run: a range of no length is all of it.
        let all = CFRange { location: 0, length: 0 };
        // SAFETY: the line is made of a valid string and font. Each run's glyphs, places,
        // advances and string indices are copied into buffers as long as its count of
        // glyphs. A run's attributes are Core Text's names to values, its font among them.
        unsafe {
            let own = font.post_script_name().to_string();
            let attributes =
                CFDictionary::<CFString, CFType>::from_slices(&[kCTFontAttributeName], &[font_value]);
            let attributed = CFAttributedString::new(None, Some(&string), Some(attributes.as_opaque()));
            let Some(attributed) = attributed else { return out };
            let line = CTLine::with_attributed_string(&attributed);
            let runs = line.glyph_runs();
            for run in runs.cast_unchecked::<CTRun>().iter() {
                let count = run.glyph_count().max(0) as usize;
                if count == 0 {
                    continue;
                }
                let run_attributes = run.attributes();
                let used = run_attributes.cast_unchecked::<CFString, CFType>().get(kCTFontAttributeName);
                let run_face = match used.as_ref().and_then(|it| it.downcast_ref::<CTFont>()) {
                    Some(used) => match used.post_script_name().to_string() {
                        name if name == own => face,
                        name => self.fallback(used, &name),
                    },
                    None => face,
                };
                let mut glyphs = vec![0u16; count];
                let mut places = vec![CGPoint { x: 0.0, y: 0.0 }; count];
                let mut advances = vec![CGSize { width: 0.0, height: 0.0 }; count];
                let mut indices: Vec<CFIndex> = vec![0; count];
                run.glyphs(all, NonNull::from(glyphs.as_mut_slice()).cast());
                run.positions(all, NonNull::from(places.as_mut_slice()).cast());
                run.advances(all, NonNull::from(advances.as_mut_slice()).cast());
                run.string_indices(all, NonNull::from(indices.as_mut_slice()).cast());
                for k in 0..count {
                    let unit = indices[k].max(0) as usize;
                    out.push(ShapedGlyph {
                        face: run_face,
                        id: glyphs[k],
                        x: places[k].x as f32,
                        y: places[k].y as f32,
                        advance: advances[k].width as f32,
                        cluster: cluster_of.get(unit).copied().unwrap_or(text.len() as u32),
                    });
                }
            }
        }
        out
    }
}

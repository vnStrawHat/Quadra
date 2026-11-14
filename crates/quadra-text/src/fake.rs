//! A deterministic text backend for headless tests: no OS, identical on every machine.
//! Every grapheme advances 0.6 em; the ascent is 0.8 em and the descent 0.2 em.

use std::rc::Rc;

use crate::bidi::{has_rtl, is_rtl};
use crate::grapheme::next_grapheme;
use crate::types::{FaceId, FaceMetrics, FontKey, GlyphBitmap, ShapedGlyph, TextBackend};

pub struct FakeBackend {
    families: Vec<Rc<str>>,
    faces: Vec<FontKey>,
}

impl Default for FakeBackend {
    fn default() -> FakeBackend {
        FakeBackend { families: vec![Rc::from("Fake Sans")], faces: Vec::new() }
    }
}

impl FakeBackend {
    pub fn new() -> FakeBackend {
        FakeBackend::default()
    }
}

impl TextBackend for FakeBackend {
    fn default_family(&self) -> String {
        "Fake Sans".to_string()
    }

    fn families(&self) -> Vec<String> {
        self.families.iter().map(|f| f.to_string()).collect()
    }

    fn load_fonts(&mut self, files: &[&[u8]]) -> Result<Vec<String>, String> {
        let mut names = Vec::new();
        for _ in files {
            let name = format!("Fake Bundled {}", self.families.len());
            self.families.push(Rc::from(name.as_str()));
            names.push(name);
        }
        Ok(names)
    }

    fn face(&mut self, key: &FontKey) -> Option<FaceId> {
        if !self.families.contains(&key.family) {
            return None;
        }
        let known = self.faces.iter().position(|f| f == key);
        let index = known.unwrap_or_else(|| {
            self.faces.push(key.clone());
            self.faces.len() - 1
        });
        Some(FaceId(index as u32))
    }

    fn metrics(&self, _: FaceId) -> FaceMetrics {
        FaceMetrics {
            ascent: 0.8,
            descent: 0.2,
            line_gap: 0.0,
            underline_position: 0.1,
            underline_thickness: 0.05,
        }
    }

    fn shape(&mut self, text: &str, face: FaceId, size: f32) -> Vec<ShapedGlyph> {
        let advance = size * 0.6;
        let mut out = Vec::new();
        let (mut i, mut x) = (0, 0.0);
        while i < text.len() {
            let first = text[i..].chars().next().unwrap_or(' ');
            out.push(ShapedGlyph { face, id: first as u32 as u16, x, y: 0.0, advance, cluster: i as u32 });
            x += advance;
            i = next_grapheme(text, i);
        }
        if has_rtl(text) {
            // A word with a right-to-left letter is all right to left, in visual order.
            out.reverse();
            for (k, g) in out.iter_mut().enumerate() {
                g.x = k as f32 * advance;
            }
        }
        out
    }

    /// Right-to-left letters at level 1; others at the text's level, or 2 in right-to-left
    /// text; digits after a right-to-left letter at 2; a space or a mark between two of one
    /// direction at that direction's, otherwise at the text's.
    fn bidi_levels(&mut self, text: &str, rtl: bool) -> Option<Vec<u8>> {
        let base = rtl as u8;
        let left = if rtl { 2 } else { 0 };
        let (mut levels, mut last_strong) = (Vec::new(), if rtl { 1 } else { 0 });
        for c in text.chars() {
            let level = if c.is_alphabetic() {
                let level = if is_rtl(c) { 1 } else { left };
                last_strong = level;
                level
            } else if c.is_ascii_digit() {
                if last_strong == 1 || rtl { 2 } else { 0 }
            } else {
                u8::MAX // a neutral: resolved below
            };
            levels.push((c.len_utf8(), level));
        }
        for k in 0..levels.len() {
            if levels[k].1 == u8::MAX {
                let before = levels[..k].iter().rev().map(|l| l.1).find(|&l| l != u8::MAX);
                let after = levels[k..].iter().map(|l| l.1).find(|&l| l != u8::MAX);
                let side = |l: u8| if l == 0 { 0 } else { 1 };
                levels[k].1 = match (before, after) {
                    (Some(b), Some(a)) if side(b) == side(a) => b.min(a),
                    _ => base,
                };
            }
        }
        Some(levels.iter().flat_map(|&(len, level)| std::iter::repeat_n(level, len)).collect())
    }

    /// A solid box of half an em by 0.7 em standing on the baseline; nothing for a space.
    fn rasterize(&mut self, _: FaceId, glyph: u16, size: f32, _phase: u8) -> Option<GlyphBitmap> {
        if glyph == b' ' as u16 || glyph == b'\t' as u16 {
            return Some(GlyphBitmap::default());
        }
        let (width, height) = ((size * 0.5).round() as u32, (size * 0.7).round() as u32);
        let pixels = vec![255; (width * height) as usize];
        Some(GlyphBitmap { left: 0, top: -(height as i32), width, height, pixels, ..GlyphBitmap::default() })
    }
}

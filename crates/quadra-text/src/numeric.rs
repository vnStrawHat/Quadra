//! Numbers are the text that changes every frame (tables, meters, clocks), so each new one
//! would be shaped. When a face shapes them glyph by glyph (no kerning or ligature between any
//! two of their characters, checked once per face), a number is composed from glyphs shaped
//! once per size instead (specs/08, "Shaping cache").

use std::collections::HashMap;

use quadra_prim::FastBuild;

use crate::system::Word;
use crate::types::{FaceId, ShapedGlyph, TextBackend};

/// How many numbers a face shapes the usual way before it is checked for composing them.
pub(crate) const WARM_UP: u32 = 64;

/// The characters numbers are written with.
const CHARS: &[u8; 17] = b"0123456789.,-+%:/";

fn index(byte: u8) -> Option<usize> {
    CHARS.iter().position(|&c| c == byte)
}

#[derive(Default)]
pub(crate) struct Numeric {
    /// Per face: the numbers asked for so far, and, once there were enough of them to be
    /// worth the check, whether its numbers may be composed.
    faces: HashMap<FaceId, (u32, Option<bool>), FastBuild>,
    /// Per face and size: the glyph of each character, shaped alone.
    singles: HashMap<(FaceId, u32), [ShapedGlyph; 17], FastBuild>,
}

fn single(backend: &mut dyn TextBackend, at: usize, face: FaceId, size: f32) -> Option<ShapedGlyph> {
    let shaped = backend.shape(std::str::from_utf8(&CHARS[at..at + 1]).ok()?, face, size);
    (shaped.len() == 1).then(|| shaped[0])
}

/// Every pair must shape to the two glyphs of its characters, the second one advanced by the
/// width of the first: 289 pairs, once per face.
fn composes(backend: &mut dyn TextBackend, face: FaceId) -> bool {
    let size = 16.0;
    let singles: Vec<_> = (0..CHARS.len()).filter_map(|i| single(backend, i, face, size)).collect();
    if singles.len() != CHARS.len() {
        return false;
    }
    (0..CHARS.len()).all(|i| {
        (0..CHARS.len()).all(|j| {
            let (pair, a, b) = ([CHARS[i], CHARS[j]], singles[i], singles[j]);
            let shaped = backend.shape(std::str::from_utf8(&pair).unwrap_or_default(), face, size);
            let same =
                |got: &ShapedGlyph, want: &ShapedGlyph| got.id == want.id && got.advance == want.advance;
            shaped.len() == 2
                && same(&shaped[0], &a)
                && same(&shaped[1], &b)
                && (shaped[1].x - (a.advance + b.x)).abs() < 0.001
        })
    })
}

impl Numeric {
    /// The word of a number, composed; `None` when the token is not one, or the face shapes
    /// numbers as more than their characters.
    pub fn word(
        &mut self,
        backend: &mut dyn TextBackend,
        token: &str,
        face: FaceId,
        size: f32,
    ) -> Option<Word> {
        if token.is_empty() || !token.bytes().all(|b| index(b).is_some()) {
            return None;
        }
        // The check shapes 300 pairs: a window that shows a few numbers never pays for it.
        let (asked, composes_them) = self.faces.entry(face).or_insert((0, None));
        *asked += 1;
        if *asked <= WARM_UP || !*composes_them.get_or_insert_with(|| composes(backend, face)) {
            return None;
        }
        let singles = match self.singles.get(&(face, size.to_bits())) {
            Some(singles) => singles,
            None => {
                let mut singles = [ShapedGlyph::default(); 17];
                for (at, glyph) in singles.iter_mut().enumerate() {
                    *glyph = single(backend, at, face, size)?;
                }
                self.singles.entry((face, size.to_bits())).or_insert(singles)
            }
        };
        let mut word = Word { glyphs: Vec::with_capacity(token.len()), width: 0.0 };
        for (at, byte) in token.bytes().enumerate() {
            let glyph = singles[index(byte)?];
            word.glyphs.push(ShapedGlyph { x: word.width + glyph.x, cluster: at as u32, ..glyph });
            word.width += glyph.advance;
        }
        Some(word)
    }
}

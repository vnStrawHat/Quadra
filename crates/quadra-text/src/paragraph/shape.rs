//! Shaping a paragraph: its text cut into words and spaces, each shaped with the face and
//! the size of the run it is in, on one endless line.

use super::{Kind, Paragraph, PositionedGlyph, Token};
use crate::grapheme::next_grapheme;
use crate::system::TextSystem;

fn token_end(bytes: &[u8], i: usize) -> usize {
    let separator = |b: u8| matches!(b, b' ' | b'\t' | b'\n');
    if separator(bytes[i]) {
        return i + 1;
    }
    bytes[i..].iter().position(|&b| separator(b)).map_or(bytes.len(), |n| i + n)
}

impl Paragraph {
    pub(super) fn shape(&mut self, ts: &mut TextSystem) {
        (self.ascent, self.descent) = ts.line_metrics(&self.style);
        self.shaped.clear();
        for (end, says) in &self.runs {
            let style = says.over(&self.style);
            self.shaped.push((*end, ts.face_for(&style), style.size));
        }
        self.shaped.push((usize::MAX, ts.face_for(&self.style), self.style.size));
        self.glyphs.clear();
        self.tokens.clear();
        self.wrapped_at = None;
        let bytes = self.text.as_bytes();
        // Where each byte's first glyph starts: clusters need not come in order.
        let mut first_x = vec![f32::NAN; bytes.len()];
        let (mut i, mut x, mut run) = (0, 0.0, 0);
        while i < bytes.len() {
            let end = token_end(bytes, i);
            let kind = match bytes[i] {
                b' ' | b'\t' => Kind::Space,
                b'\n' => Kind::Newline,
                _ => Kind::Word,
            };
            let from = self.glyphs.len();
            let (mut width, mut at) = (0.0, i);
            // A word that goes over the end of a run is shaped in pieces, a piece a run.
            while at < end && kind != Kind::Newline {
                while self.shaped[run].0 <= at {
                    run += 1;
                }
                let (run_end, face, size) = self.shaped[run];
                let piece_end = end.min(run_end);
                let word = ts.word(&self.text[at..piece_end], face, size);
                for g in &word.glyphs {
                    let (cluster, x) = (at + g.cluster as usize, x + width + g.x);
                    if first_x[cluster].is_nan() {
                        first_x[cluster] = x;
                    }
                    let (face, id, y, advance) = (g.face, g.id, g.y, g.advance);
                    self.glyphs.push(PositionedGlyph { face, id, x, y, advance, cluster: cluster as u32 });
                }
                (width, at) = (width + word.width, piece_end);
            }
            // A token starts at the pen even when no glyph marks its first byte (a newline).
            if first_x[i].is_nan() {
                first_x[i] = x;
            }
            self.tokens.push(Token { start: i, end, width, glyphs: from..self.glyphs.len(), kind });
            x += width;
            i = end;
        }
        // A boundary inside a ligature has no glyph of its own: it takes the stop before it.
        self.stops.clear();
        let (mut at, mut last) = (0, 0.0);
        while at < bytes.len() {
            if !first_x[at].is_nan() {
                last = first_x[at];
            }
            self.stops.push((at, last));
            at = next_grapheme(&self.text, at);
        }
        self.stops.push((bytes.len(), x));
    }
}

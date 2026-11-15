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
    /// The faces of the runs and the metrics of the lines: what the style and the runs say,
    /// whatever the text. Then the text is shaped.
    pub(super) fn restyle(&mut self, ts: &mut TextSystem) {
        (self.ascent, self.descent) = ts.line_metrics(&self.style);
        self.shaped.clear();
        for (end, says) in &self.runs {
            let style = says.over(&self.style);
            self.shaped.push((*end, ts.face_for(&style), style.size, says.advance));
        }
        self.shaped.push((usize::MAX, ts.face_for(&self.style), self.style.size, None));
        self.shape(ts);
    }

    pub(super) fn shape(&mut self, ts: &mut TextSystem) {
        self.glyphs.clear();
        self.tokens.clear();
        self.wrapped_at = None;
        let bytes = self.text.as_bytes();
        // Where each byte's first glyph starts: clusters need not come in order.
        let mut first_x = std::mem::take(&mut ts.firsts);
        first_x.clear();
        first_x.resize(bytes.len(), f32::NAN);
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
            // A token starts at its pen, whatever glyph is first in it (a right-to-left word).
            first_x[i] = x;
            // A word that goes over the end of a run is shaped in pieces, a piece a run.
            while at < end && kind != Kind::Newline {
                while self.shaped[run].0 <= at {
                    run += 1;
                }
                let (run_end, face, size, advance) = self.shaped[run];
                let piece_end = end.min(run_end);
                if let Some(advance) = advance {
                    // A box: the pen goes past it, and nothing is shaped.
                    if first_x[at].is_nan() {
                        first_x[at] = x + width;
                    }
                    (width, at) = (width + advance, piece_end);
                    continue;
                }
                // A number is composed in a buffer kept for it: it is no word to keep.
                let (token, mut composed) = (&self.text[at..piece_end], std::mem::take(&mut ts.composed));
                composed.clear();
                let word;
                let (glyphs, word_width) = match ts.number(token, face, size, &mut composed) {
                    Some(width) => (&composed[..], width),
                    None => {
                        word = ts.word(token, face, size);
                        (&word.glyphs[..], word.width)
                    }
                };
                for g in glyphs {
                    let (cluster, x) = (at + g.cluster as usize, x + width + g.x);
                    if first_x[cluster].is_nan() {
                        first_x[cluster] = x;
                    }
                    let (face, id, y, advance) = (g.face, g.id, g.y, g.advance);
                    self.glyphs.push(PositionedGlyph { face, id, x, y, advance, cluster: cluster as u32 });
                }
                (width, at) = (width + word_width, piece_end);
                ts.composed = composed;
            }
            self.tokens.push(Token { start: i, end, x, width, glyphs: from..self.glyphs.len(), kind });
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
        ts.firsts = first_x;
        self.bidi = ts.bidi_levels(&self.text);
    }
}

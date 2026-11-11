//! Paragraphs: text shaped once and broken into lines at a width (specs/08, "Paragraphs").
//!
//! A text node owns its paragraph. Shaping happens when the text or a shaping style changes;
//! breaking at a new width walks the shaped tokens and shapes nothing; colour and decoration
//! touch neither.

use std::ops::Range;

use quadra_prim::Size;

use crate::grapheme::next_grapheme;
use crate::system::TextSystem;
use crate::types::{FaceId, TextStyle};

/// The slack absorbs float error in layout sums, which would otherwise break a word that was
/// measured to fit.
const SLACK: f32 = 0.01;

/// A glyph of a paragraph. `x` runs along the whole text as if it were one endless line; a
/// line gives its own start to subtract. `cluster` is a byte offset in the text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositionedGlyph {
    pub face: FaceId,
    pub id: u16,
    pub x: f32,
    /// Above the baseline.
    pub y: f32,
    pub advance: f32,
    pub cluster: u32,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Word,
    /// One space or tab: it hangs past the edge instead of breaking.
    Space,
    Newline,
}

struct Token {
    start: usize,
    end: usize,
    width: f32,
    glyphs: Range<usize>,
    kind: Kind,
}

/// One line of a wrapped paragraph.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    /// The byte range in the text.
    pub start: usize,
    pub end: usize,
    /// Without trailing spaces.
    pub width: f32,
    /// The top of the line box.
    pub y: f32,
    glyphs: Range<usize>,
    origin: f32,
}

pub struct Paragraph {
    text: String,
    style: TextStyle,
    glyphs: Vec<PositionedGlyph>,
    /// Caret stops: every grapheme boundary and its x, the end of the text included.
    stops: Vec<(usize, f32)>,
    tokens: Vec<Token>,
    lines: Vec<Line>,
    size: Size,
    wrapped_at: Option<f32>,
    /// Some line ends because the next word did not fit. Without that, the same lines come
    /// out at any width from the paragraph's own up.
    width_breaks: bool,
    ascent: f32,
    descent: f32,
}

fn token_end(bytes: &[u8], i: usize) -> usize {
    let separator = |b: u8| matches!(b, b' ' | b'\t' | b'\n');
    if separator(bytes[i]) {
        return i + 1;
    }
    bytes[i..].iter().position(|&b| separator(b)).map_or(bytes.len(), |n| i + n)
}

impl Paragraph {
    pub fn new(ts: &mut TextSystem, text: impl Into<String>, style: TextStyle) -> Paragraph {
        let mut p = Paragraph {
            text: text.into(),
            style,
            glyphs: Vec::new(),
            stops: Vec::new(),
            tokens: Vec::new(),
            lines: Vec::new(),
            size: Size::ZERO,
            wrapped_at: None,
            width_breaks: false,
            ascent: 0.0,
            descent: 0.0,
        };
        p.shape(ts);
        p
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn style(&self) -> &TextStyle {
        &self.style
    }

    /// Replaces the text. Words seen before come from the cache; false when it is equal.
    pub fn set_text(&mut self, ts: &mut TextSystem, text: &str) -> bool {
        if self.text == text {
            return false;
        }
        self.text.clear();
        self.text.push_str(text);
        self.shape(ts);
        true
    }

    /// Replaces the shaping style; false when it is equal.
    pub fn set_style(&mut self, ts: &mut TextSystem, style: &TextStyle) -> bool {
        if self.style == *style {
            return false;
        }
        self.style = style.clone();
        self.shape(ts);
        true
    }

    fn shape(&mut self, ts: &mut TextSystem) {
        let (face, size) = (ts.face_for(&self.style), self.style.size);
        (self.ascent, self.descent) = ts.line_metrics(&self.style);
        self.glyphs.clear();
        self.tokens.clear();
        self.wrapped_at = None;
        let bytes = self.text.as_bytes();
        // Where each byte's first glyph starts: clusters need not come in order.
        let mut first_x = vec![f32::NAN; bytes.len()];
        let (mut i, mut x) = (0, 0.0);
        while i < bytes.len() {
            let end = token_end(bytes, i);
            let kind = match bytes[i] {
                b' ' | b'\t' => Kind::Space,
                b'\n' => Kind::Newline,
                _ => Kind::Word,
            };
            let from = self.glyphs.len();
            let mut width = 0.0;
            if kind != Kind::Newline {
                let word = ts.word(&self.text[i..end], face, size);
                for g in &word.glyphs {
                    let cluster = i + g.cluster as usize;
                    if first_x[cluster].is_nan() {
                        first_x[cluster] = x + g.x;
                    }
                    let (face, id, y, advance) = (g.face, g.id, g.y, g.advance);
                    self.glyphs.push(PositionedGlyph {
                        face,
                        id,
                        x: x + g.x,
                        y,
                        advance,
                        cluster: cluster as u32,
                    });
                }
                width = word.width;
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

    /// The x of the caret stop at or before a byte offset, along the endless line.
    pub fn x_at(&self, index: usize) -> f32 {
        let after = self.stops.partition_point(|&(at, _)| at <= index);
        self.stops[after.saturating_sub(1)].1
    }

    /// The byte offset of the caret stop nearest an x along the endless line: where a click
    /// puts the caret.
    pub fn index_at(&self, x: f32) -> usize {
        let nearest = self.stops.iter().min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()));
        nearest.map_or(0, |&(at, _)| at)
    }

    /// Breaks the paragraph into lines no wider than `max_width` (infinite: only at newlines)
    /// and returns its size: the widest line and the height of all of them.
    pub fn wrap(&mut self, ts: &mut TextSystem, max_width: f32) -> Size {
        // The same lines come out of every width from the widest line up to the width they
        // were broken at: every line still fits, and none can take its next word. A layout
        // asks at the space it has, then at the width it was told: both are in that range.
        // Lines that no width broke are the same at any width they fit in.
        let fits = max_width + SLACK >= self.size.width;
        if self.wrapped_at.is_some_and(|at| fits && (max_width <= at || !self.width_breaks)) {
            return self.size;
        }
        ts.stats.paragraphs_broken += 1;
        let limit = max_width + SLACK;
        let mut ranges = Vec::new();
        let (mut line_start, mut x, mut has_content, mut hard) = (0, 0.0, false, 1);
        for token in &self.tokens {
            match token.kind {
                Kind::Newline => {
                    ranges.push(line_start..token.start);
                    (line_start, x, has_content, hard) = (token.end, 0.0, false, hard + 1);
                }
                Kind::Space => x += token.width,
                Kind::Word => {
                    if has_content && x + token.width > limit {
                        ranges.push(line_start..token.start);
                        (line_start, x) = (token.start, 0.0);
                    }
                    if token.width > limit && limit > 0.0 {
                        // A word wider than the line breaks between graphemes.
                        x = 0.0;
                        for g in &self.glyphs[token.glyphs.clone()] {
                            let cluster = g.cluster as usize;
                            if x + g.advance > limit && cluster > line_start {
                                ranges.push(line_start..cluster);
                                (line_start, x) = (cluster, 0.0);
                            }
                            x += g.advance;
                        }
                    } else {
                        x += token.width;
                    }
                    has_content = true;
                }
            }
        }
        ranges.push(line_start..self.text.len());

        let line_px = self.style.line_px();
        let bytes = self.text.as_bytes();
        self.lines.clear();
        let mut widest: f32 = 0.0;
        for (k, range) in ranges.iter().enumerate() {
            let origin = self.x_at(range.start);
            let visible =
                bytes[range.clone()].iter().rposition(|b| !matches!(b, b' ' | b'\t')).map_or(0, |n| n + 1);
            let width = self.x_at(range.start + visible) - origin;
            let from = self.glyphs.partition_point(|g| (g.cluster as usize) < range.start);
            let to = self.glyphs.partition_point(|g| (g.cluster as usize) < range.end);
            widest = widest.max(width);
            let y = k as f32 * line_px;
            self.lines.push(Line { start: range.start, end: range.end, width, y, glyphs: from..to, origin });
        }
        self.size = Size::new(widest, ranges.len() as f32 * line_px);
        self.width_breaks = ranges.len() > hard;
        self.wrapped_at = Some(max_width);
        self.size
    }

    /// The size from the last [`Paragraph::wrap`].
    pub fn size(&self) -> Size {
        self.size
    }

    /// The lines from the last [`Paragraph::wrap`].
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// The glyphs of a line, and the x to subtract from theirs to start the line at zero.
    pub fn line_glyphs(&self, line: &Line) -> (&[PositionedGlyph], f32) {
        (&self.glyphs[line.glyphs.clone()], line.origin)
    }

    /// The ascent and descent of the lines, in pixels.
    pub fn metrics(&self) -> (f32, f32) {
        (self.ascent, self.descent)
    }
}

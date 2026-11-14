//! Paragraphs: text shaped once and broken into lines at a width (specs/08, "Paragraphs").
//!
//! A text node owns its paragraph. Shaping happens when the text or a shaping style changes;
//! breaking at a new width walks the shaped tokens and shapes nothing; colour and decoration
//! touch neither.

use std::ops::Range;

use quadra_prim::Size;

use crate::system::TextSystem;
use crate::types::{FaceId, Shaping, TextStyle};

mod shape;
mod visual;

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
    /// Its pen along the endless line.
    x: f32,
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
    /// The top of the line box, its height, and its baseline from its top: the style's line,
    /// or higher where a box on it reaches further.
    pub y: f32,
    pub height: f32,
    pub baseline: f32,
    glyphs: Range<usize>,
    /// Where the line starts along the endless line the text was shaped on.
    pub(crate) origin: f32,
}

pub struct Paragraph {
    text: String,
    style: TextStyle,
    /// Rich text: where each run ends, and what it says otherwise than the style. Empty:
    /// all of the text is as the style says.
    runs: Vec<(usize, Shaping)>,
    /// Where each run ends, the face and the size it was shaped with, and how wide it is
    /// when it is a box; the last one never ends.
    shaped: Vec<(usize, FaceId, f32, Option<f32>)>,
    glyphs: Vec<PositionedGlyph>,
    /// Whether the text reads right to left and the level of each byte, when it has a
    /// right-to-left character; then `visual` is the glyphs with each line in visual order.
    bidi: Option<(bool, Vec<u8>)>,
    visual: Vec<PositionedGlyph>,
    /// Caret stops: every grapheme boundary and its x, the end of the text included.
    pub(crate) stops: Vec<(usize, f32)>,
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

impl Paragraph {
    pub fn new(ts: &mut TextSystem, text: impl Into<String>, style: TextStyle) -> Paragraph {
        Paragraph::rich(ts, text, style, Vec::new())
    }

    /// A paragraph of runs: `runs` says where each ends, in order, and what it has otherwise
    /// than `style`. What comes after the last one is as the style says.
    pub fn rich(
        ts: &mut TextSystem,
        text: impl Into<String>,
        style: TextStyle,
        runs: Vec<(usize, Shaping)>,
    ) -> Paragraph {
        let mut p = Paragraph {
            text: text.into(),
            style,
            runs,
            shaped: Vec::new(),
            glyphs: Vec::new(),
            bidi: None,
            visual: Vec::new(),
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

    /// The run a byte offset is in, counted from the first, and the size of its text.
    pub fn run_at(&self, index: usize) -> (usize, f32) {
        let run = self.shaped.partition_point(|&(end, ..)| end <= index);
        (run, self.shaped.get(run).map_or(self.style.size, |&(_, _, size, _)| size))
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
        // The text's baseline in a line, and what reaches further over it or under it: a box,
        // or a run larger than the text, whose line would be as much larger.
        let base = (line_px - self.ascent - self.descent) / 2.0 + self.ascent;
        let (mut boxes, mut from) = (Vec::new(), 0);
        for (end, shaping) in &self.runs {
            let larger = shaping.size.map_or(1.0, |size| size / self.style.size);
            if shaping.advance.is_some() {
                boxes.push((from..from + 1, shaping.reach));
            } else if larger > 1.0 {
                boxes.push((from..*end, (base * larger, (line_px - base) * larger)));
            }
            from = *end;
        }
        let (mut widest, mut y): (f32, f32) = (0.0, 0.0);
        for range in &ranges {
            let origin = self.x_at(range.start);
            let visible =
                bytes[range.clone()].iter().rposition(|b| !matches!(b, b' ' | b'\t')).map_or(0, |n| n + 1);
            let width = self.x_at(range.start + visible) - origin;
            let from = self.glyphs.partition_point(|g| (g.cluster as usize) < range.start);
            let to = self.glyphs.partition_point(|g| (g.cluster as usize) < range.end);
            widest = widest.max(width);
            let (mut above, mut below) = (base, line_px - base);
            let on = |at: &std::ops::Range<usize>| {
                at.start < range.end.max(range.start + 1) && at.end > range.start
            };
            for &(_, (up, down)) in boxes.iter().filter(|(at, _)| on(at)) {
                (above, below) = (above.max(up.ceil()), below.max(down.ceil()));
            }
            let (start, end, height) = (range.start, range.end, above + below);
            self.lines.push(Line { start, end, width, y, height, baseline: above, glyphs: from..to, origin });
            y += height;
        }
        self.size = Size::new(widest, y);
        self.width_breaks = ranges.len() > hard;
        self.wrapped_at = Some(max_width);
        self.place_visually();
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
        let glyphs = if self.bidi.is_some() { &self.visual } else { &self.glyphs };
        (&glyphs[line.glyphs.clone()], line.origin)
    }

    /// The line at a height from the paragraph's top: the first above it, the last below.
    pub fn line_at_y(&self, y: f32) -> usize {
        let after = self.lines.partition_point(|line| line.y + line.height <= y);
        after.min(self.lines.len().saturating_sub(1))
    }

    /// The ascent and descent of the lines, in pixels.
    pub fn metrics(&self) -> (f32, f32) {
        (self.ascent, self.descent)
    }
}

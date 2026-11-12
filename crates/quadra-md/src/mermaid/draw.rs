//! What a diagram that was laid out paints: a list of things in the space of the diagram
//! (logical pixels from its top-left corner). The layout makes it once; painting walks it.
//! Colours are roles, so another theme needs no other layout.

use std::rc::Rc;

use quadra_prim::{Point, Rect};
use quadra_text::{TextStyle, TextSystem, Word};

/// What a thing is painted with: each is a role of the theme, or one made faint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    None,
    /// The fill of a node.
    Panel,
    /// Behind the label of a link.
    Canvas,
    /// The borders of nodes and frames, and lifelines.
    Line,
    Text,
    /// The titles of frames.
    Faint,
    /// Links and their heads.
    Accent,
    /// The fill of the frame of a subgraph, and of a frame of a sequence.
    Frame,
    /// Text on the accent: the numbers of messages.
    OnAccent,
    /// The fill of a note.
    Note,
}

/// What a link ends with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Head {
    None,
    /// A filled triangle.
    Arrow,
    /// Two strokes: a message that is not waited for.
    Open,
    Circle,
    Cross,
}

pub enum Op {
    /// A rectangle with a fill, a border and rounded corners.
    Box { rect: Rect, radius: f32, fill: Ink, border: Ink },
    /// A line through points, dashed or not.
    Line { points: Vec<Point>, width: f32, dash: bool, ink: Ink },
    /// A filled polygon with a border.
    Fill { points: Vec<Point>, fill: Ink, border: Ink },
    /// A line of text: `at` is the left end of its baseline; its width and the height of
    /// its line are for the bounds of the drawing.
    Text { at: Point, word: Rc<Word>, size: f32, ink: Ink, line: f32 },
    /// The head of a link: its tip, and a point behind it on the line.
    Head { tip: Point, from: Point, head: Head, ink: Ink },
}

#[derive(Default)]
pub struct Drawing {
    pub ops: Vec<Op>,
    pub width: f32,
    pub height: f32,
}

/// Measures and shapes text in the font of a diagram: `base` is the style of a node's text.
pub struct Measure<'a> {
    pub ts: &'a mut TextSystem,
    pub base: TextStyle,
}

/// Lines of text, shaped: what is between the breaks of a label.
#[derive(Clone, Default)]
pub struct TextBox {
    pub lines: Vec<Rc<Word>>,
    pub width: f32,
    pub height: f32,
    pub size: f32,
}

/// Where the baseline is in a line of text, from its top, as a part of the line.
const BASELINE: f32 = 0.72;

impl Measure<'_> {
    /// How far a line of text of a size is from the next.
    pub fn line(size: f32) -> f32 {
        (size * 1.4 + 0.5).floor()
    }

    pub fn word(&mut self, text: &str, size: f32, weight: u16) -> Rc<Word> {
        let face = self.ts.face_for(&TextStyle { size, weight, ..self.base.clone() });
        self.ts.word(text, face, size)
    }

    pub fn text(&mut self, text: &str, size: f32, weight: u16) -> TextBox {
        let lines: Vec<_> =
            text.split('\n').filter(|_| !text.is_empty()).map(|line| self.word(line, size, weight)).collect();
        let width = lines.iter().fold(0.0f32, |width, line| width.max(line.width));
        TextBox { width, height: Measure::line(size) * lines.len() as f32, size, lines }
    }
}

impl Drawing {
    /// The lines of a text, each in the middle, the middle of all of them at `center`.
    pub fn text(&mut self, text: &TextBox, center: Point, ink: Ink) {
        let line = Measure::line(text.size);
        let top = center.y - text.height / 2.0;
        for (k, word) in text.lines.iter().enumerate() {
            let at = Point::new(center.x - word.width / 2.0, top + line * (k as f32 + BASELINE));
            self.ops.push(Op::Text { at, word: word.clone(), size: text.size, ink, line });
        }
    }

    /// One line with its left end at `x`, its middle at the height `y`.
    pub fn text_left(&mut self, word: Rc<Word>, size: f32, x: f32, y: f32, ink: Ink) {
        let line = Measure::line(size);
        let at = Point::new(x, y - line / 2.0 + line * BASELINE);
        self.ops.push(Op::Text { at, word, size, ink, line });
    }

    pub fn line(&mut self, points: Vec<Point>, width: f32, dash: bool, ink: Ink) {
        self.ops.push(Op::Line { points, width, dash, ink });
    }

    pub fn head(&mut self, tip: Point, from: Point, head: Head, ink: Ink) {
        if head != Head::None {
            self.ops.push(Op::Head { tip, from, head, ink });
        }
    }

    /// Moves the drawing so that what it holds starts a margin from its corner, and sets
    /// its size.
    pub fn normalize(&mut self, margin: f32) {
        let (mut low, mut high) = (Point::new(f32::MAX, f32::MAX), Point::new(f32::MIN, f32::MIN));
        let mut hold = |rect: Rect| {
            low = Point::new(low.x.min(rect.x), low.y.min(rect.y));
            high = Point::new(high.x.max(rect.right()), high.y.max(rect.bottom()));
        };
        // A point of a line has its width and the head of a link about it.
        let about = |p: &Point| Rect::new(p.x - 6.0, p.y - 6.0, 12.0, 12.0);
        for op in &self.ops {
            match op {
                Op::Box { rect, .. } => hold(*rect),
                Op::Text { at, word, line, .. } => {
                    hold(Rect::new(at.x, at.y - line * BASELINE, word.width, *line))
                }
                Op::Line { points, .. } | Op::Fill { points, .. } => {
                    points.iter().map(about).for_each(&mut hold)
                }
                Op::Head { tip, from, .. } => [tip, from].into_iter().map(about).for_each(&mut hold),
            }
        }
        if high.x < low.x || !(high.x - low.x).is_finite() || !(high.y - low.y).is_finite() {
            return;
        }
        let (dx, dy) = (margin - low.x, margin - low.y);
        let moved = |p: &mut Point| *p = Point::new(p.x + dx, p.y + dy);
        for op in &mut self.ops {
            match op {
                Op::Box { rect, .. } => *rect = rect.translate(dx, dy),
                Op::Text { at, .. } => moved(at),
                Op::Line { points, .. } | Op::Fill { points, .. } => points.iter_mut().for_each(moved),
                Op::Head { tip, from, .. } => [tip, from].into_iter().for_each(moved),
            }
        }
        (self.width, self.height) = (high.x - low.x + 2.0 * margin, high.y - low.y + 2.0 * margin);
    }
}

/// The text of a label: `<br>` breaks its line, quotes around it are dropped, and the
/// names of four characters are those characters. Read once: what a name gives is not read
/// again.
pub fn decode(label: &str) -> String {
    const SWAPS: [(&str, &str); 8] = [
        ("<br/>", "\n"),
        ("<br />", "\n"),
        ("<br>", "\n"),
        ("\\n", "\n"),
        ("&quot;", "\""),
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
    ];
    let label = label.trim();
    let quoted = label.strip_prefix('"').and_then(|label| label.strip_suffix('"'));
    let (mut rest, mut out) = (quoted.unwrap_or(label), String::new());
    while let Some(c) = rest.chars().next() {
        let swap = SWAPS.iter().find(|(from, _)| rest.starts_with(from));
        let (from, to) = swap.map_or((c.len_utf8(), None), |(from, to)| (from.len(), Some(*to)));
        match to {
            Some(to) => out.push_str(to),
            None => out.push(c),
        }
        rest = &rest[from..];
    }
    out
}

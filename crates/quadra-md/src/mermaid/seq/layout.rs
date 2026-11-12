//! Where the things of a sequence diagram are: participants in columns far enough apart
//! for the messages and notes between them, what happens one under the other, frames about
//! what they hold; and what each is drawn as.

use quadra_prim::{Point, Rect};

use super::{Actor, Item, Seq, Side};
use crate::mermaid::curve::{cut_end, dist, head_len};
use crate::mermaid::draw::{Drawing, Head, Ink, Measure, Op, TextBox};

/// A frame that is open: where it starts, where its parts start with what they say, and
/// how far to the sides what it holds goes.
struct Frame {
    word: &'static str,
    text: String,
    top: f32,
    branches: Vec<(f32, String)>,
    sides: Option<(f32, f32)>,
    /// How many frames are in each other inside it.
    inner: usize,
}

impl Frame {
    fn cover(&mut self, left: f32, right: f32) {
        let (low, high) = self.sides.unwrap_or((left, right));
        self.sides = Some((low.min(left), high.max(right)));
    }
}

/// Makes the gaps between the middles of `a` and `b` wider, each by as much, until they add
/// up to `need`.
fn spread(gap: &mut [f32], a: usize, b: usize, need: f32) {
    let have: f32 = gap[a..b].iter().sum();
    if have < need {
        gap[a..b].iter_mut().for_each(|gap| *gap += (need - have) / (b - a) as f32);
    }
}

/// The middle of each participant: apart enough for every label between them.
fn columns(s: &Seq, widths: &[f32], texts: &[TextBox]) -> Vec<f32> {
    let last = s.actors.len() - 1;
    // From the middle of each to the middle of the next.
    let mut gap: Vec<f32> = widths.windows(2).map(|pair| (pair[0] + pair[1]) / 2.0 + 36.0).collect();
    // The room left of the first participant.
    let mut left = 0.0f32;
    for (item, text) in s.items.iter().zip(texts) {
        let w = text.width;
        match *item {
            Item::Message { from, to, .. } if from != to => {
                spread(&mut gap, from.min(to), from.max(to), w + 40.0)
            }
            // What is at the right of a participant: a message to itself, a note.
            Item::Message { from, .. } if from < last => spread(&mut gap, from, from + 1, w + 50.0),
            Item::Note { from, side: Side::Right, .. } if from < last => {
                spread(&mut gap, from, from + 1, w + 40.0)
            }
            Item::Note { from, side: Side::Left, .. } if from > 0 => {
                spread(&mut gap, from - 1, from, w + 40.0)
            }
            Item::Note { side: Side::Left, .. } => left = left.max(w + 40.0),
            Item::Note { from, to, side: Side::Over, .. } if from != to => {
                spread(&mut gap, from, to, w - 16.0)
            }
            _ => {}
        }
    }
    let after = gap.iter().scan(left, |at, gap| {
        *at += gap;
        Some(*at)
    });
    std::iter::once(left).chain(after).collect()
}

/// Lays a sequence diagram out: what to paint.
pub fn layout(s: &Seq, m: &mut Measure) -> Drawing {
    let (size, mut d) = (m.base.size, Drawing::default());
    if s.actors.is_empty() {
        return d;
    }
    let small = Measure::line(size * 0.85);
    let heads: Vec<TextBox> = s.actors.iter().map(|actor| m.text(&actor.label, size, 500)).collect();
    let widths: Vec<f32> = heads.iter().map(|head| (head.width + 24.0).max(80.0)).collect();
    let text = |item: &Item, m: &mut Measure| match item {
        Item::Message { text, .. } => m.text(text, size * 0.92, 400),
        Item::Note { text, .. } => m.text(text, size, 400),
        _ => TextBox::default(),
    };
    let texts: Vec<TextBox> = s.items.iter().map(|item| text(item, m)).collect();
    let c = columns(s, &widths, &texts);
    let head_h = if s.actors.iter().any(|actor| actor.figure) { 58.0 } else { 38.0 };
    // Frames and lifelines are under the rest.
    let mut back = Drawing::default();
    let (mut y, mut number) = (head_h + 24.0, 0);
    let mut open: Vec<Frame> = Vec::new();
    for (item, text) in s.items.iter().zip(&texts) {
        // How far to the sides the item goes, for the frame it is in.
        let mut sides = None;
        match item {
            Item::Message { from, to, dashed, head, .. } => {
                let (a, b) = (c[*from], c[*to]);
                let label = text.height.max(Measure::line(size * 0.92));
                number += 1;
                let numbered = s.autonumber.then_some(number);
                if from == to {
                    // A message to itself: its label beside a turn out and back.
                    d.text(text, Point::new(a + 36.0 + text.width / 2.0, y + label / 2.0), Ink::Text);
                    y += label + 4.0;
                    let turn = [(a, y), (a + 30.0, y), (a + 30.0, y + 22.0), (a, y + 22.0)];
                    d.message(&turn.map(|(x, y)| Point::new(x, y)), *dashed, *head, numbered, m);
                    sides = Some((a, a + 40.0 + text.width));
                    y += 22.0 + 20.0;
                } else {
                    d.text(text, Point::new((a + b) / 2.0, y + label / 2.0), Ink::Text);
                    y += label + 6.0;
                    d.message(&[Point::new(a, y), Point::new(b, y)], *dashed, *head, numbered, m);
                    sides = Some((a.min(b), a.max(b)));
                    y += 22.0;
                }
            }
            Item::Note { from, to, side, .. } => {
                let over = if from != to { c[*to] - c[*from] + 40.0 } else { 0.0 };
                let (w, h) = ((text.width + 20.0).max(over), text.height + 14.0);
                let left = match side {
                    Side::Left => c[*from] - 12.0 - w,
                    Side::Right => c[*from] + 12.0,
                    Side::Over => (c[*from] + c[*to]) / 2.0 - w / 2.0,
                };
                let rect = Rect::new(left, y, w, h);
                // A note is a tint, and the lifelines it is over do not show through it.
                d.ops.push(Op::Box { rect, radius: 3.0, fill: Ink::Canvas, border: Ink::None });
                d.ops.push(Op::Box { rect, radius: 3.0, fill: Ink::Note, border: Ink::Line });
                d.text(text, rect.center(), Ink::Text);
                sides = Some((left, left + w));
                y += h + 16.0;
            }
            Item::Open { word, text } => {
                open.push(Frame {
                    word,
                    text: text.clone(),
                    top: y,
                    branches: Vec::new(),
                    sides: None,
                    inner: 0,
                });
                y += small + 18.0;
            }
            Item::Branch { text } => {
                open.last_mut().into_iter().for_each(|frame| frame.branches.push((y, text.clone())));
                y += small + 14.0;
            }
            Item::Close => {
                if let Some(frame) = open.pop() {
                    let (left, right) = back.frame(&frame, y, m);
                    sides = Some((left, right));
                    open.last_mut()
                        .into_iter()
                        .for_each(|outer| outer.inner = outer.inner.max(frame.inner + 1));
                }
                y += 18.0;
            }
        }
        if let (Some((left, right)), Some(frame)) = (sides, open.last_mut()) {
            frame.cover(left, right);
        }
    }
    let bottom = y + 4.0;
    for (i, actor) in s.actors.iter().enumerate() {
        back.line(vec![Point::new(c[i], head_h), Point::new(c[i], bottom)], 1.0, true, Ink::Line);
        d.participant(actor, &heads[i], Rect::new(c[i] - widths[i] / 2.0, 0.0, widths[i], head_h));
        d.participant(actor, &heads[i], Rect::new(c[i] - widths[i] / 2.0, bottom, widths[i], head_h));
    }
    back.ops.append(&mut d.ops);
    back.normalize(8.0);
    back
}

impl Drawing {
    /// The line of a message with its head, and its number at its start.
    fn message(
        &mut self,
        points: &[Point],
        dashed: bool,
        head: Head,
        number: Option<usize>,
        m: &mut Measure,
    ) {
        let (start, tip) = (points[0], points[points.len() - 1]);
        let line = cut_end(points, head_len(head, 1.5));
        let end = line[line.len() - 1];
        // A head the line is not made shorter for points along its last piece.
        let from = if dist(end, tip) < 0.5 { points[points.len() - 2] } else { end };
        self.line(line, 1.5, dashed, Ink::Accent);
        self.head(tip, from, head, Ink::Accent);
        if let Some(number) = number {
            let rect = Rect::new(start.x - 9.0, start.y - 9.0, 18.0, 18.0);
            self.ops.push(Op::Box { rect, radius: 9.0, fill: Ink::Accent, border: Ink::None });
            self.text(&m.text(&number.to_string(), m.base.size * 0.75, 600), start, Ink::OnAccent);
        }
    }

    /// A participant at an end of its lifeline: a box with its name, or a figure over it.
    fn participant(&mut self, actor: &Actor, name: &TextBox, r: Rect) {
        if !actor.figure {
            self.ops.push(Op::Box { rect: r, radius: 4.0, fill: Ink::Panel, border: Ink::Line });
            return self.text(name, r.center(), Ink::Text);
        }
        let (x, top) = (r.center().x, r.y);
        let at = |dx: f32, dy: f32| Point::new(x + dx, top + dy);
        // The figure ends over the name: its feet are not in the letters.
        let head = Rect::new(x - 7.0, top, 14.0, 14.0);
        self.ops.push(Op::Box { rect: head, radius: 7.0, fill: Ink::None, border: Ink::Text });
        self.line(vec![at(0.0, 14.0), at(0.0, 27.0)], 1.5, false, Ink::Text);
        self.line(vec![at(-10.0, 19.0), at(10.0, 19.0)], 1.5, false, Ink::Text);
        self.line(vec![at(-8.0, 36.0), at(0.0, 27.0), at(8.0, 36.0)], 1.5, false, Ink::Text);
        self.text(name, at(0.0, r.height - name.height / 2.0), Ink::Text);
    }

    /// A frame that ends at `y`, wider by the frames inside it: its word in a tab, what it
    /// says beside it, and a line over each of its parts. Gives its sides.
    fn frame(&mut self, frame: &Frame, y: f32, m: &mut Measure) -> (f32, f32) {
        let size = m.base.size * 0.85;
        let pad = 12.0 + 10.0 * frame.inner as f32;
        let (left, right) = frame.sides.unwrap_or((0.0, 120.0));
        let (left, right) = (left - pad, right + pad);
        let rect = Rect::new(left, frame.top, right - left, y + 6.0 - frame.top);
        self.ops.push(Op::Box { rect, radius: 3.0, fill: Ink::Frame, border: Ink::Line });
        let word = m.text(frame.word, size, 600);
        let tab = Rect::new(left, frame.top, word.width + 16.0, Measure::line(size) + 6.0);
        self.ops.push(Op::Box { rect: tab, radius: 3.0, fill: Ink::Panel, border: Ink::Line });
        self.text(&word, tab.center(), Ink::Text);
        let mut said = |d: &mut Drawing, text: &str, x: f32, y: f32| {
            if !text.is_empty() {
                d.text_left(
                    m.word(&format!("[{}]", text.replace('\n', " ")), size, 400),
                    size,
                    x,
                    y,
                    Ink::Faint,
                );
            }
        };
        said(self, &frame.text, tab.right() + 8.0, tab.center().y);
        for (top, text) in &frame.branches {
            self.line(vec![Point::new(left, *top), Point::new(right, *top)], 1.0, true, Ink::Line);
            said(self, text, left + 10.0, top + Measure::line(size) / 2.0 + 5.0);
        }
        (left, right)
    }
}

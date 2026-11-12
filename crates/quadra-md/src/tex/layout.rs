//! The layout of a formula, in the way of TeX: every node becomes a box (a width, a height
//! above the baseline, a depth below it) of glyphs, rules and strokes. The baseline is at
//! y = 0 and y grows down.

use std::rc::Rc;

use quadra_prim::Point;
use quadra_text::{FaceId, TextStyle, TextSystem, Word};

use super::{Class, Limits, Node};

/// The family formulas are set in: it comes with Windows. Without it, the default one.
pub const FAMILY: &str = "Cambria Math";

/// What a box holds, each thing where the box says.
pub enum Part {
    /// Shaped text at a size; its place is the start of its pen, on its baseline.
    Glyphs { word: Rc<Word>, size: f32 },
    /// A filled rectangle; its place is its top-left corner.
    Rule { width: f32, height: f32 },
    /// A line through points, so thick; the points are from its place.
    Stroke { points: Vec<Point>, width: f32 },
    /// A box in the box; its place is the left end of its baseline.
    Box(Laid),
}

/// A box. Its parts are a tree: putting a box in another moves nothing of what it holds.
#[derive(Default)]
pub struct Laid {
    pub width: f32,
    /// Above the baseline.
    pub height: f32,
    /// Below the baseline.
    pub depth: f32,
    /// How far the ink leans past the width: what is raised after it moves right by it.
    pub(super) lean: f32,
    /// The bottom of the ink of glyphs, below the baseline (less than 0: above it).
    pub(super) low: Option<f32>,
    pub(super) parts: Vec<(f32, f32, Part)>,
}

impl Laid {
    pub(super) fn sized(width: f32, height: f32, depth: f32) -> Laid {
        Laid { width, height, depth, ..Laid::default() }
    }

    /// Puts `child` with the left end of its baseline at (x, y). The size is the caller's.
    pub(super) fn put(&mut self, child: Laid, x: f32, y: f32) {
        if !child.parts.is_empty() {
            self.parts.push((x, y, Part::Box(child)));
        }
    }

    /// Makes the box as high and as deep as `child` with its baseline at `y` needs.
    pub(super) fn grow(&mut self, child: &Laid, y: f32) {
        self.height = self.height.max(child.height - y);
        self.depth = self.depth.max(child.depth + y);
    }

    pub(super) fn rule(&mut self, x: f32, top: f32, width: f32, height: f32) {
        self.parts.push((x, top, Part::Rule { width, height }));
    }

    pub(super) fn stroke(&mut self, points: Vec<Point>, width: f32) {
        self.parts.push((0.0, 0.0, Part::Stroke { points, width }));
    }

    /// Calls `f` with every part that is no box, and where it is from the left end of the
    /// baseline of this one.
    pub fn each(&self, x: f32, y: f32, f: &mut impl FnMut(f32, f32, &Part)) {
        for (px, py, part) in &self.parts {
            match part {
                Part::Box(inner) => inner.each(x + px, y + py, f),
                part => f(x + px, y + py, part),
            }
        }
    }
}

/// The styles: display, text, script, script of a script; how large each sets its glyphs.
const SCALE: [f32; 4] = [1.0, 1.0, 0.7, 0.5];

/// The space between atoms by their classes, in thin, medium and thick spaces (1 to 3);
/// less than 0: in display and in text only.
const SPACING: [[i8; 8]; 8] = [
    [0, 1, -2, -3, 0, 0, 0, -1],
    [1, 1, 0, -3, 0, 0, 0, -1],
    [-2, -2, 0, 0, -2, 0, 0, -2],
    [-3, -3, 0, 0, -3, 0, 0, -3],
    [0, 0, 0, 0, 0, 0, 0, 0],
    [0, 1, -2, -3, 0, 0, 0, -1],
    [-1, -1, 0, -1, -1, -1, -1, -1],
    [-1, 1, -2, -3, -1, 0, -1, -1],
];

/// Lays formulas out at a size of text.
pub struct Lay<'a> {
    pub(super) ts: &'a mut TextSystem,
    size: f32,
    face: FaceId,
    axes: [Option<f32>; 4],
}

impl<'a> Lay<'a> {
    pub fn new(ts: &'a mut TextSystem, size: f32) -> Lay<'a> {
        let face = ts.face_for(&TextStyle { family: Rc::from(FAMILY), size, ..TextStyle::default() });
        Lay { ts, size, face, axes: [None; 4] }
    }

    pub(super) fn em(&self, style: usize) -> f32 {
        self.size * SCALE[style.min(3)]
    }

    fn mu(&self, style: usize) -> f32 {
        self.em(style) / 18.0
    }

    /// How thick the bar of a fraction and the top of a root are.
    pub(super) fn rule_width(&self, style: usize) -> f32 {
        (self.em(style) * 0.055).max(0.6)
    }

    pub(super) fn script(style: usize) -> usize {
        if style < 2 { 2 } else { 3 }
    }

    pub(super) fn glyph(&mut self, text: &str, style: usize) -> Laid {
        self.glyph_at(text, self.em(style))
    }

    /// Text shaped at a size, as high and as deep as its ink: the ascent of a face is far
    /// more than a formula has room for.
    pub(super) fn glyph_at(&mut self, text: &str, size: f32) -> Laid {
        if text.is_empty() {
            return Laid::default();
        }
        let word = self.ts.word(text, self.face, size);
        let mut laid = Laid { width: word.width, ..Laid::default() };
        let (mut right, mut low) = (0.0f32, f32::MIN);
        for g in &word.glyphs {
            let ink = self.ts.ink(g.face, g.id, size);
            if ink.width <= 0.0 {
                continue;
            }
            laid.height = laid.height.max(g.y - ink.y);
            low = low.max(ink.bottom() - g.y);
            right = right.max(g.x + ink.right());
        }
        laid.depth = low.max(0.0);
        laid.low = (low > f32::MIN).then_some(low);
        laid.lean = (right - word.width - 0.5).max(0.0);
        laid.parts.push((0.0, 0.0, Part::Glyphs { word, size }));
        laid
    }

    /// How high over the baseline the axis is: where the bar of a fraction is, and the
    /// middle of an operator. It is the middle of a minus.
    pub(super) fn axis(&mut self, style: usize) -> f32 {
        let style = style.min(3);
        if let Some(axis) = self.axes[style] {
            return axis;
        }
        let minus = self.glyph("\u{2212}", style);
        let axis = match minus.low {
            Some(low) if minus.height > 0.0 => (minus.height - low) / 2.0,
            _ => self.em(style) * 0.25,
        };
        self.axes[style] = Some(axis);
        axis
    }

    /// Nodes side by side in a style, with the spaces their classes ask for.
    pub fn list(&mut self, nodes: &[Node], style: usize) -> Laid {
        self.list_after(nodes, style, None)
    }

    /// The same, as if the list came after an atom of a class (a cell after an `&`).
    pub(super) fn list_after(&mut self, nodes: &[Node], style: usize, after: Option<Class>) -> Laid {
        let mut classes: Vec<Class> = nodes.iter().map(Node::class).collect();
        // A binary operator with nothing on a side to work on is ordinary: -x, a = -b.
        for i in 0..classes.len() {
            let before = if i > 0 { classes[i - 1] } else { after.unwrap_or(Class::Open) };
            let next = classes.get(i + 1).copied().unwrap_or(Class::Close);
            let alone = matches!(before, Class::Bin | Class::Op | Class::Rel | Class::Open | Class::Punct)
                || matches!(next, Class::Rel | Class::Close | Class::Punct);
            if classes[i] == Class::Bin && alone {
                classes[i] = Class::Ord;
            }
        }
        let (mut laid, mut style, mut before) = (Laid::default(), style, after.map(|class| class as usize));
        for (node, class) in nodes.iter().zip(classes) {
            match node {
                Node::Style(level) => style = usize::from(*level),
                Node::Space(amount) => laid.width += amount * self.mu(style),
                node => {
                    let child = self.node(node, style);
                    let gap = before.map_or(0, |before| SPACING[before][class as usize]);
                    if gap > 0 || (gap < 0 && style < 2) {
                        laid.width += f32::from(gap.abs() + 2) * self.mu(style);
                    }
                    laid.grow(&child, 0.0);
                    (laid.lean, before) = (child.lean, Some(class as usize));
                    let x = laid.width;
                    laid.width += child.width;
                    laid.put(child, x, 0.0);
                }
            }
        }
        laid
    }

    pub(super) fn node(&mut self, node: &Node, style: usize) -> Laid {
        match node {
            Node::Atom { text, .. } | Node::Text(text) => self.glyph(text, style),
            Node::Function { name, .. } => self.glyph(name, style),
            Node::Group(body) => self.list(body, style),
            Node::Frac { over, under, rule, style: fixed } => {
                self.frac(over, under, *rule, fixed.map_or(style, usize::from))
            }
            Node::Sqrt { index, body } => self.sqrt(index.as_deref(), body, style),
            Node::Scripts { base, sup, sub } => {
                let limits = match **base {
                    Node::BigOp { limits, .. } | Node::Function { limits, .. } => limits,
                    _ => Limits::Side,
                };
                let (sup, sub) = (sup.as_deref(), sub.as_deref());
                if limits == Limits::Always || (limits == Limits::Display && style == 0) {
                    let base = self.node(base, style);
                    self.limits(base, sup, sub, style)
                } else {
                    self.scripts(base, sup, sub, style)
                }
            }
            Node::BigOp { glyph, .. } => self.big_op(glyph, style),
            Node::Delim { left, right, body } => {
                let inner = self.list(body, style);
                self.wrap(inner, left, right, style)
            }
            Node::Big { delim, level, .. } => {
                let half = [1.2, 1.8, 2.4, 3.0][usize::from(*level).clamp(1, 4) - 1] * self.em(style) / 2.0;
                self.delimiter(delim, half, style)
            }
            Node::Accent { mark, body } => self.accent(mark, body, style),
            Node::Line { body, under } => self.line(body, *under, style),
            Node::Matrix { env, rows } => self.matrix(env, rows, style),
            Node::Stack { body, over, under, .. } => {
                let base = self.list(body, style);
                self.limits(base, over.as_deref(), under.as_deref(), style)
            }
            Node::Space(_) | Node::Style(_) => Laid::default(),
        }
    }
}

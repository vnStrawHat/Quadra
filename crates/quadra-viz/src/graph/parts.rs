//! What a graph is made of: its nodes, its links and how a link is drawn.

use std::rc::Rc;

use quadra_design::token;
use quadra_prim::{Point, Rect, Size};
use quadra_tree::Paint;
use quadra_view::Div;

/// A view at a point of the graph's plane, as large as its size, named by its label.
pub struct Node {
    pub(crate) id: Rc<str>,
    pub(crate) at: Point,
    pub(crate) size: Size,
    pub(crate) label: String,
    pub(crate) face: Rc<dyn Fn() -> Div>,
    pub(crate) svg: Option<Rc<dyn Fn(Rect) -> String>>,
}

impl Node {
    pub fn new(
        id: impl Into<Rc<str>>,
        at: Point,
        size: Size,
        label: impl Into<String>,
        face: impl Fn() -> Div + 'static,
    ) -> Node {
        Node { id: id.into(), at, size, label: label.into(), face: Rc::new(face), svg: None }
    }

    /// What the export writes for it, given its box in the plane: SVG of what its face shows,
    /// in place of a box with its label.
    pub fn svg(mut self, write: impl Fn(Rect) -> String + 'static) -> Self {
        self.svg = Some(Rc::new(write));
        self
    }
}

/// A link from the node `from` to the node `to`, by their ids.
pub struct Link {
    pub(crate) from: Rc<str>,
    pub(crate) to: Rc<str>,
    pub(crate) through: Vec<Point>,
}

impl Link {
    pub fn new(from: impl Into<Rc<str>>, to: impl Into<Rc<str>>) -> Link {
        Link { from: from.into(), to: to.into(), through: Vec::new() }
    }

    /// Points of the plane the link passes, in order: past the columns it skips.
    pub fn through(mut self, points: impl IntoIterator<Item = Point>) -> Self {
        self.through = points.into_iter().collect();
        self
    }
}

/// How a link is drawn: its colour, its width, its dashes (the length of a dash and of a
/// gap; 0 is solid), and a head at its end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkStyle {
    pub color: Paint,
    pub width: f32,
    pub dash: f32,
    pub arrow: bool,
}

impl Default for LinkStyle {
    fn default() -> Self {
        LinkStyle { color: token::TEXT_3.into(), width: 1.5, dash: 0.0, arrow: true }
    }
}

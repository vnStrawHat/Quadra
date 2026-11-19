//! What a rich text is made of (specs/05, "Rules"): its spans, the boxes in its lines, and
//! where its links are.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_prim::{Color, Point, Rect};
use quadra_text::{Shaping, TextStyle};

use crate::theme::Paint;
use crate::widget::{MeasureCx, PaintCx};

/// A span of a rich text: where it ends in the text, and what it says of itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RichSpan {
    pub end: usize,
    pub shaping: Shaping,
    /// Its colour, where it is not that of the text around: a colour, or a role.
    pub color: Option<Paint>,
    /// A fill behind it, as high as its line.
    pub background: Option<Paint>,
    /// A line through it.
    pub strike: bool,
    /// What a click on it gives: it is a link, and underlined.
    pub link: Option<Rc<str>>,
}

/// What is in a line of rich text and is no text: a formula. Its span is one character,
/// U+FFFC, and as wide as the box says.
pub trait InlineBox {
    /// Its width, its height above the baseline and its depth below it, in a text of a
    /// style. A line it is on is made as high as it reaches.
    fn measure(&mut self, cx: &mut MeasureCx, style: &TextStyle) -> (f32, f32, f32);

    /// Paints it with the left end of its baseline at `origin`, in the colour of its span.
    fn paint(&mut self, cx: &mut PaintCx, origin: Point, color: Color);

    /// What it is to a reader: said in its place in the text.
    fn says(&self) -> &str;
}

/// Where the links of a rich text are, from the node's corner, as last painted: a box for
/// each line a link is on, and what the link gives. The view that handles the pointer holds
/// them too.
pub type LinkBoxes = Rc<RefCell<Vec<(Rect, Rc<str>)>>>;

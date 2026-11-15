//! The look of a pane tree (specs/behaviour/16, "Pane tree"): seams of a line in a strip of
//! 6 px, and the veil over the part of a pane a dropped tab would take.

use std::rc::Rc;

use quadra_headless::work::{self, Axis, Pane, Panes, Parts};
use quadra_layout::{Direction, Display, Justify};
use quadra_platform::Cursor;
use quadra_prim::{Edges, Rect};
use quadra_reactive::{Memo, Signal};
use quadra_view::{Div, View, div};

use crate::token;

/// The width of the strip around the seam of a split, which takes the pointer; the line in
/// its middle is one pixel.
const GRIP: f32 = 6.0;

/// A seam: a line in the middle of its strip, or beside a folded leaf a quieter line alone.
fn seam(strip: Div, axis: Axis, fixed: bool) -> Div {
    let stacked = axis == Axis::Column;
    let line = if stacked { div().height(1.0) } else { div().width(1.0) };
    let strip = strip.justify(Justify::Center);
    let strip = strip.direction(if stacked { Direction::Column } else { Direction::Row });
    if fixed {
        return strip.child(line.bg(token::LINE));
    }
    let strip = strip.child(line.bg(token::LINE_STRONG));
    strip.cursor(if stacked { Cursor::ResizeVertical } else { Cursor::ResizeHorizontal })
}

/// What marks where a drop would land, in a box of this size: a grey veil with a line in the
/// colour of the focus around it, 4 px in.
fn veil(shown: Memo<Option<Rect>>) -> Div {
    let rect = move || shown.get().unwrap_or_default().inset(Edges::all(4.0));
    let veil = div().absolute().left(move || rect().x).top(move || rect().y);
    let veil = veil.width(move || rect().width).height(move || rect().height);
    let veil = veil.bg(token::TEXT_3.alpha(0.5)).border(1.0).border_color(token::BORDER_FOCUSED);
    veil.radius(token::RADIUS_SM)
        .display(move || if shown.get().is_some() { Display::Flex } else { Display::None })
}

/// Shows a pane tree: `leaf` builds what a group shows, once, and the seams between the
/// sides of the splits are dragged to give a side more of the room (which writes the tree).
/// A collapsed leaf keeps 32 px, a tab strip, unless told (`.header(px)`).
pub fn panes<V, F>(root: Signal<Pane>, leaf: F) -> Panes<F>
where
    V: View + 'static,
    F: Fn(&str) -> V + 'static,
{
    let parts = Parts { grip: GRIP, seam: Rc::new(seam), veil: Rc::new(veil) };
    work::panes(root, leaf, parts).header(32.0)
}

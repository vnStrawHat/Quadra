//! The split (specs/behaviour/09, "Layout"): two panes and the seam between them, which is
//! dragged to give one pane more of the room. The share of the first pane is a signal of the
//! caller's, from 0 to 1.

use std::cell::Cell;
use std::rc::Rc;

use quadra_layout::{Direction, Justify, Length, Overflow};
use quadra_platform::Cursor;
use quadra_prim::Size;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, PointerEvent, PointerKind};
use quadra_ui::{AnyView, MountCx, View, div};

use crate::token;

/// The width of the strip that takes the pointer around the seam of one pixel.
const GRIP: f32 = 6.0;

pub struct Split {
    first: AnyView,
    second: AnyView,
    share: Signal<f32>,
    stacked: bool,
    least: f32,
}

/// Two panes side by side: `share` is how much of the width the first one has. Dragging the
/// seam writes it; a double click on the seam makes the panes even.
pub fn split(first: impl View + 'static, second: impl View + 'static, share: Signal<f32>) -> Split {
    Split { first: AnyView::new(first), second: AnyView::new(second), share, stacked: false, least: 80.0 }
}

impl Split {
    /// The first pane over the second: `share` is of the height.
    pub fn stacked(mut self) -> Self {
        self.stacked = true;
        self
    }

    /// The least a pane is dragged to, in pixels: 80 unless told.
    pub fn least(mut self, px: f32) -> Self {
        self.least = px;
        self
    }
}

impl View for Split {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (share, stacked, least) = (self.share, self.stacked, self.least);
        let along = move |size: Size| if stacked { size.height } else { size.width };
        let part = move || Length::Pct(share.get() * 100.0);
        let pane = || div().overflow_x(Overflow::Hidden).overflow_y(Overflow::Hidden);
        let (first, second) = (pane().shrink(0.0).child(self.first), pane().grow(1.0).child(self.second));
        let (first, line) = match stacked {
            true => (first.height(part), div().height(1.0)),
            false => (first.width(part), div().width(1.0)),
        };

        // The seam: the press on it takes the pointer, and the share follows the pointer by
        // as much of the whole as it moves. The whole is known once the split is laid out.
        let whole: Rc<Cell<Option<Signal<Size>>>> = Rc::default();
        let held: Rc<Cell<Option<(f32, f32)>>> = Rc::default();
        let at = move |e: &PointerEvent| if stacked { e.at.y } else { e.at.x };
        let (press, drag, total) = (held.clone(), held.clone(), whole.clone());
        let grip = div().shrink(0.0).justify(Justify::Center).child(line.bg(token::LINE_STRONG));
        let grip = match stacked {
            true => grip.height(GRIP).direction(Direction::Column).cursor(Cursor::ResizeVertical),
            false => grip.width(GRIP).cursor(Cursor::ResizeHorizontal),
        };
        let grip = grip.on_pointer(PointerKind::Down, move |e| {
            press.set(Some((at(e), share.get_untracked())));
            e.capture_pointer();
            e.stop();
        });
        let grip = grip.on_pointer(PointerKind::Move, move |e| {
            let (Some((from, was)), Some(whole)) = (drag.get(), total.get()) else { return };
            let whole = along(whole.get_untracked());
            if whole > 2.0 * least {
                let to = was + (at(e) - from) / whole;
                share.set(to.clamp(least / whole, 1.0 - least / whole));
            }
        });
        let grip = grip.on_pointer(PointerKind::Up, move |_| held.set(None));
        let grip = grip.on_click(move |e| {
            if e.clicks == 2 {
                share.set(0.5);
            }
        });

        let frame = div().grow(1.0).direction(if stacked { Direction::Column } else { Direction::Row });
        let id = frame.child(first).child(grip).child(second).mount(cx);
        whole.set(Some(cx.tree.size_signal(id)));
        id
    }
}

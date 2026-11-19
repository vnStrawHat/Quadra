//! The split (specs/17, "The components"): two panes and the seam between them, which is
//! dragged to give one pane more of the room, never leaving a pane under the least it may
//! have; a double click on the seam makes the panes even. The share of the first pane is a
//! signal of the caller's, from 0 to 1. What does not fit in a pane is cut. The look gives the
//! seam.

use std::cell::Cell;
use std::rc::Rc;

use quadra_layout::{Direction, Display, Length, Overflow};
use quadra_prim::Size;
use quadra_reactive::Signal;
use quadra_tree::{NodeId, PointerEvent, PointerKind};
use quadra_view::{AnyView, Div, MountCx, View, div};

pub struct Split {
    first: AnyView,
    second: AnyView,
    share: Signal<f32>,
    seam: Div,
    stacked: bool,
    least: f32,
    alone: Option<Rc<dyn Fn() -> Option<usize>>>,
}

/// Two panes side by side with `seam` between them: `share` is how much of the width the
/// first one has.
pub fn split(
    first: impl View + 'static,
    second: impl View + 'static,
    share: Signal<f32>,
    seam: Div,
) -> Split {
    let (first, second) = (AnyView::new(first), AnyView::new(second));
    Split { first, second, share, seam, stacked: false, least: 80.0, alone: None }
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

    /// While `f` gives `Some(0)` or `Some(1)`, that pane has the whole and the other and the
    /// seam are not displayed: kept, with what they hold.
    pub fn alone(mut self, f: impl Fn() -> Option<usize> + 'static) -> Self {
        self.alone = Some(Rc::new(f));
        self
    }
}

impl View for Split {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (share, stacked, least) = (self.share, self.stacked, self.least);
        let along = move |size: Size| if stacked { size.height } else { size.width };
        // The pane that has the whole, if one has.
        let alone = self.alone.clone();
        let solo: Rc<dyn Fn() -> Option<usize>> = Rc::new(move || alone.as_ref().and_then(|f| f()));
        let shown = |pane: usize| {
            let solo = solo.clone();
            move || match solo() {
                Some(only) if only != pane => Display::None,
                _ => Display::Flex,
            }
        };
        let part = {
            let solo = solo.clone();
            move || match solo() {
                Some(_) => Length::Pct(100.0),
                None => Length::Pct(share.get() * 100.0),
            }
        };
        let pane = || div().overflow_x(Overflow::Hidden).overflow_y(Overflow::Hidden);
        let first = pane().shrink(0.0).display(shown(0)).child(self.first);
        let second = pane().grow(1.0).display(shown(1)).child(self.second);
        let first = if stacked { first.height(part) } else { first.width(part) };

        // The press on the seam takes the pointer, and the share follows the pointer by as
        // much of the whole as it moves. The whole is known once the split is laid out.
        let whole: Rc<Cell<Option<Signal<Size>>>> = Rc::default();
        let held: Rc<Cell<Option<(f32, f32)>>> = Rc::default();
        let at = move |e: &PointerEvent| if stacked { e.at.y } else { e.at.x };
        let (press, drag, total) = (held.clone(), held.clone(), whole.clone());
        let seam = self.seam.on_pointer(PointerKind::Down, move |e| {
            press.set(Some((at(e), share.get_untracked())));
            e.capture_pointer();
            e.stop();
        });
        let seam = seam.on_pointer(PointerKind::Move, move |e| {
            let (Some((from, was)), Some(whole)) = (drag.get(), total.get()) else { return };
            let whole = along(whole.get_untracked());
            if whole > 2.0 * least {
                let to = was + (at(e) - from) / whole;
                share.set(to.clamp(least / whole, 1.0 - least / whole));
            }
        });
        let seam = seam.on_pointer(PointerKind::Up, move |_| held.set(None));
        let seam = seam.on_click(move |e| {
            if e.clicks == 2 {
                share.set(0.5);
            }
        });

        let seam = seam.display(move || if solo().is_some() { Display::None } else { Display::Flex });
        let frame = div().grow(1.0).direction(if stacked { Direction::Column } else { Direction::Row });
        let id = frame.child(first).child(seam).child(second).mount(cx);
        whole.set(Some(cx.tree.size_signal(id)));
        id
    }
}

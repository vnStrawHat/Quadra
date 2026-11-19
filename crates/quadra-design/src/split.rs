//! The split (specs/behaviour/09, "Layout"): the headless crate's (`quadra_headless::split`) with the kit's
//! seam, a line of one pixel in a grip of 6 with the cursor of a resize.

use quadra_headless::split as base;
use quadra_layout::{Direction, Justify};
use quadra_platform::Cursor;
use quadra_reactive::Signal;
use quadra_tree::NodeId;
use quadra_view::{AnyView, MountCx, View, div};

use crate::token;

/// The width of the strip that takes the pointer around the seam of one pixel.
const GRIP: f32 = 6.0;

pub struct Split {
    first: AnyView,
    second: AnyView,
    share: Signal<f32>,
    stacked: bool,
    least: f32,
    alone: Option<Box<dyn Fn() -> Option<usize>>>,
}

/// Two panes side by side: `share` is how much of the width the first one has. Dragging the
/// seam writes it; a double click on the seam makes the panes even.
pub fn split(first: impl View + 'static, second: impl View + 'static, share: Signal<f32>) -> Split {
    let (first, second) = (AnyView::new(first), AnyView::new(second));
    Split { first, second, share, stacked: false, least: 80.0, alone: None }
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

    /// While `f` gives `Some(0)` or `Some(1)`, that pane has the whole, the other and the seam
    /// hidden and kept.
    pub fn alone(mut self, f: impl Fn() -> Option<usize> + 'static) -> Self {
        self.alone = Some(Box::new(f));
        self
    }
}

impl View for Split {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let line = if self.stacked { div().height(1.0) } else { div().width(1.0) };
        let seam = div().shrink(0.0).justify(Justify::Center).child(line.bg(token::LINE_STRONG));
        let seam = match self.stacked {
            true => seam.height(GRIP).direction(Direction::Column).cursor(Cursor::ResizeVertical),
            false => seam.width(GRIP).cursor(Cursor::ResizeHorizontal),
        };
        let mut split = base::split(self.first, self.second, self.share, seam).least(self.least);
        if let Some(alone) = self.alone {
            split = split.alone(alone);
        }
        if self.stacked { split.stacked() } else { split }.mount(cx)
    }
}

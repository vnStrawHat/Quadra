//! The thumb of a scrollbar (specs/06, "Scrolling"): a pill in the text colour that holds at
//! its node. The node is the grip, wider than the pill; what moves it and when it shows is
//! the scroll view's to say.

use quadra_prim::{Color, Corners, Rect};
use quadra_render::Quad;

use crate::inherit::Inherited;
use crate::widget::{PaintCx, Widget};

/// The width of the pill, how far it is from the edge, and how much of the text colour it has.
const THICK: f32 = 6.0;
const INSET: f32 = 4.0;
const STRENGTH: f32 = 0.3;

#[derive(Default)]
pub struct ScrollThumb {
    color: Color,
    /// From 0, not there, to 1.
    shows: f32,
}

impl ScrollThumb {
    /// Sets how much of the thumb shows, and says whether that is a change.
    pub fn show(&mut self, shows: f32) -> bool {
        let changed = self.shows != shows;
        self.shows = shows;
        changed
    }
}

impl Widget for ScrollThumb {
    fn paint(&mut self, cx: &mut PaintCx) {
        let bounds = Rect::new(cx.rect.right() - INSET - THICK, cx.rect.y, THICK, cx.rect.height);
        let background = self.color.alpha(self.color.a * STRENGTH * self.shows);
        cx.quad(Quad { bounds, background, radii: Corners::all(THICK / 2.0), ..Quad::default() });
    }

    fn inherit(&mut self, inherited: &Inherited) {
        self.color = inherited.color;
    }
}

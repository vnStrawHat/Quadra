//! The system's three buttons in a window with the app's own frame (specs/23, "The
//! buttons"): AppKit puts them at the middle of its own title bar, which is lower than the
//! bar most apps draw, so they stand above the middle of the app's. They are put at its
//! middle. How tall the app's bar is, is asked as the system asks what a point is: it is as
//! far down as the app says the point under the close button is its caption.

use std::rc::Weak;

use objc2_app_kit::{NSWindowButton, NSWindowStyleMask};
use objc2_foundation::NSPoint;
use quadra_platform::{HitRegion, WindowId};
use quadra_prim::Point;

use super::Shared;

/// No bar of an app's is taller, in points: where the search for its end stops.
const TALLEST: f32 = 128.0;

impl Shared {
    /// How tall the caption is at `x`, to a point: none when the top of the window is not
    /// the app's caption there. `known` is what it was, which two questions confirm.
    fn caption_height(&self, id: WindowId, x: f32, known: f32) -> f32 {
        let caption = |y: f32| self.hit_test(id, Point::new(x, y)) == HitRegion::Caption;
        if known > 0.0 && caption(known - 1.0) && !caption(known + 1.0) {
            return known;
        }
        if !caption(1.0) {
            return 0.0;
        }
        // The caption is one stretch from the top: halved until its end is within a point.
        let (mut inside, mut outside) = (1.0, TALLEST);
        while outside - inside > 1.0 {
            let middle = (inside + outside) / 2.0;
            if caption(middle) { inside = middle } else { outside = middle }
        }
        outside.round()
    }

    /// Puts the buttons of a window at the middle of the app's bar. After every frame: the
    /// bar may have a new height, and AppKit puts the buttons back when it lays its own bar
    /// out again (a new size, the full screen left).
    pub(crate) fn place_buttons(&self, id: WindowId) {
        let state = self.windows.borrow().iter().filter_map(Weak::upgrade).find(|it| it.id == id);
        let Some(state) = state.filter(|it| it.custom_frame) else { return };
        let Some(window) = state.window.load() else { return };
        // Filling the screen the window shows no buttons, and the bar is the system's to place.
        if window.styleMask().contains(NSWindowStyleMask::FullScreen) {
            return;
        }
        let Some(close) = window.standardWindowButton(NSWindowButton::CloseButton) else { return };
        let frame = close.frame();
        let x = (frame.origin.x + frame.size.width / 2.0) as f32;
        let bar = self.caption_height(id, x, state.bar.get());
        state.bar.set(bar);
        if bar == 0.0 {
            return;
        }
        // The view that holds the system's bar is made as tall as the app's, its top where
        // it was: a button outside it would not be pressed.
        // SAFETY: questions of views that are alive, about the views that hold them.
        let holder = unsafe { close.superview().and_then(|bar| bar.superview()) };
        let Some(holder) = holder else { return };
        let mut held = holder.frame();
        if held.size.height != f64::from(bar) {
            held.origin.y += held.size.height - f64::from(bar);
            held.size.height = f64::from(bar);
            holder.setFrame(held);
        }
        let all =
            [NSWindowButton::CloseButton, NSWindowButton::MiniaturizeButton, NSWindowButton::ZoomButton];
        for button in all.into_iter().filter_map(|which| window.standardWindowButton(which)) {
            let at = button.frame();
            let y = ((f64::from(bar) - at.size.height) / 2.0).round();
            if at.origin.y != y {
                button.setFrameOrigin(NSPoint::new(at.origin.x, y));
            }
        }
    }
}

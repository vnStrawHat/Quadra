//! Where a window is (specs/15, "Window state"): its frame as AppKit keeps it, in points
//! of the desktop from the bottom left of the first display, read and put back only where
//! a display is.

use objc2::MainThreadMarker;
use objc2_app_kit::{NSScreen, NSWindow};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use quadra_platform::Placement;

/// As much of a window as must be on a display for the user to take it by, each way.
const REACH: f64 = 64.0;

/// Whether a frame is on one of the displays, by what they show of it.
fn reachable(frame: NSRect, displays: &[NSRect]) -> bool {
    let shared = |a: f64, a_len: f64, b: f64, b_len: f64| (a + a_len).min(b + b_len) - a.max(b);
    displays.iter().any(|shown| {
        shared(frame.origin.x, frame.size.width, shown.origin.x, shown.size.width) >= REACH
            && shared(frame.origin.y, frame.size.height, shown.origin.y, shown.size.height) >= REACH
    })
}

/// Puts the window where it was, if a display is there now: whether it did.
/// ponytail: a zoomed window comes back by its frame alone; the frame it had before the
/// zoom is not kept, and a window that filled the screen comes back as one of that size.
pub(super) fn put(mtm: MainThreadMarker, window: &NSWindow, place: Placement) -> bool {
    let frame = NSRect::new(
        NSPoint::new(place.x.into(), place.y.into()),
        NSSize::new(place.width.into(), place.height.into()),
    );
    let displays: Vec<NSRect> = NSScreen::screens(mtm).iter().map(|it| it.visibleFrame()).collect();
    let there = reachable(frame, &displays);
    if there {
        window.setFrame_display(frame, false);
    }
    there
}

pub(super) fn of(window: &NSWindow) -> Placement {
    let frame = window.frame();
    Placement {
        x: frame.origin.x as i32,
        y: frame.origin.y as i32,
        width: frame.size.width as i32,
        height: frame.size.height as i32,
        maximized: window.isZoomed(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_place_is_taken_only_where_a_display_shows_enough_of_it() {
        let rect = |x: f64, y: f64, w: f64, h: f64| NSRect::new(NSPoint::new(x, y), NSSize::new(w, h));
        // Two displays side by side, the second higher up.
        let displays = [rect(0.0, 0.0, 1440.0, 875.0), rect(1440.0, 200.0, 1920.0, 1055.0)];
        assert!(reachable(rect(100.0, 100.0, 800.0, 600.0), &displays));
        assert!(reachable(rect(3000.0, 1000.0, 800.0, 600.0), &displays), "a corner on the second");
        assert!(!reachable(rect(3400.0, 300.0, 800.0, 600.0), &displays), "past the right end");
        assert!(!reachable(rect(100.0, -580.0, 800.0, 600.0), &displays), "a sliver at the bottom");
        assert!(!reachable(rect(100.0, 100.0, 800.0, 600.0), &[]));
    }
}

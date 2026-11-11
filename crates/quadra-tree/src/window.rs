//! What a tree knows of the window it is in, and asks of it (specs/10, "Windows with their
//! own frame"): which of its nodes the window is dragged by, whether the window is
//! maximized, and the minimize, maximize and close a view asks for.

use quadra_platform::HitRegion;
use quadra_prim::Point;
use quadra_reactive::{Signal, signal};

use crate::input::Handler;
use crate::node::NodeId;
use crate::shared::Handle;
use crate::tree::Tree;

/// What a view asks of its window. The app does it after the handler that asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowRequest {
    Minimize,
    /// Maximize, or put back as it was.
    ToggleMaximize,
    Close,
}

impl Handle {
    /// Asks something of the tree's window.
    pub fn ask_window(&self, request: WindowRequest) {
        self.0.asked.borrow_mut().push(request);
        self.0.wake.set(true);
    }
}

impl Tree {
    /// What was asked of the window since this was last called.
    pub fn take_window_requests(&mut self) -> Vec<WindowRequest> {
        self.shared.asked.take()
    }

    /// Whether the window is maximized, as a signal: made where it is first asked for, which
    /// is where the window's view is mounted.
    pub fn maximized(&mut self) -> Signal<bool> {
        *self.maximized.get_or_insert_with(|| signal(false))
    }

    /// Tells the tree whether its window is maximized: what reads it runs again.
    pub fn set_maximized(&mut self, on: bool) {
        if let Some(maximized) = self.maximized {
            maximized.set(on);
        }
    }

    /// Says what a node is to the OS in a window with its own frame: the bar the window is
    /// dragged by, its maximize button.
    pub fn set_window_region(&mut self, id: NodeId, region: HitRegion) {
        if let Some(node) = self.get_mut(id) {
            node.region = region;
        }
    }

    /// What a point of the window is to the OS: the region of the deepest node under it that
    /// has one, unless a node that takes the pointer (a handler for it, the focus) is deeper:
    /// a button in the title bar stays a button.
    pub fn window_region_at(&self, at: Point) -> HitRegion {
        let Some(hit) = self.hit_test(at) else { return HitRegion::Client };
        for id in self.path(hit) {
            let Some(node) = self.get(id) else { continue };
            let takes = node.focusable || node.handlers.iter().any(|h| matches!(h, Handler::Pointer { .. }));
            if node.region != HitRegion::Client || takes {
                return node.region;
            }
        }
        HitRegion::Client
    }
}

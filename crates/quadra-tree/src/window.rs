//! What a tree knows of the window it is in, and asks of it (specs/10, "Windows with their
//! own frame"): which of its nodes the window is dragged by, whether the window is
//! maximized, active or minimized, the minimize, maximize and close a view asks for, and whether the window may
//! close (specs/15, "Closing").

use quadra_platform::HitRegion;
use quadra_prim::{Point, Size};
use quadra_reactive::{Signal, signal};

use crate::input::Handler;
use crate::node::NodeId;
use crate::shared::Handle;
use crate::tree::Tree;

/// What a tree knows of its window, as signals: what reads one runs again when it changes.
#[derive(Clone, Copy)]
pub struct WindowState {
    pub maximized: Signal<bool>,
    /// It has the keyboard.
    pub active: Signal<bool>,
    pub minimized: Signal<bool>,
}

/// What a view asks of its window. The app does it after the handler that asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WindowRequest {
    Minimize,
    /// Maximize, or put back as it was.
    ToggleMaximize,
    /// Close as the user asks to: when what is in the window says it may.
    Close,
    /// Close, asking nothing: what a view does once the user said what to do.
    CloseNow,
    /// What the OS shows as its title: its frame, the taskbar.
    Title(String),
    /// Shown again if minimized, and brought to the front.
    Front,
}

impl Tree {
    /// Draws the whole UI `zoom` times larger: the next frame lays out a window so many times
    /// smaller at a scale so many times larger. What comes from the window in its logical
    /// pixels is divided by it (`zoom()`), as the app does.
    pub fn set_zoom(&mut self, zoom: f32) {
        if zoom != self.zoom {
            self.zoom = zoom;
            let (size, scale) = self.window;
            self.fit(size, scale); // this frame lays out at it, if it has not yet
        }
    }

    /// The size and scale of a window of `size` at `scale`, zoomed.
    pub(crate) fn fit(&mut self, size: Size, scale: f32) {
        self.window = (size, scale);
        let (size, scale) = (Size::new(size.width / self.zoom, size.height / self.zoom), scale * self.zoom);
        if size != self.size {
            self.relayout = true;
        }
        if scale != self.scale {
            // Snapping and glyph sizes depend on the scale: everything is painted again. A
            // listener is told the root again, which carries the scale to device pixels.
            (self.repaint, self.repaint_all) = (true, true);
            if let Some(root) = self.root {
                self.semantics_changed(root);
            }
        }
        (self.size, self.scale) = (size, scale);
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }
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

    /// Runs `f` when the user asks the window to close: it says whether the window may. One
    /// that says no asks the user itself, and closes the window later with
    /// [`WindowRequest::CloseNow`].
    pub fn on_close(&mut self, id: NodeId, f: impl FnMut() -> bool + 'static) {
        self.add_handler(id, Handler::Close(Box::new(f)));
    }

    /// Asks what is in the tree whether the window may close, until one says no.
    pub fn may_close(&mut self) -> bool {
        for node in self.nodes.iter_mut().filter(|node| node.alive) {
            for handler in &mut node.handlers {
                if let Handler::Close(may) = handler
                    && !may()
                {
                    return false;
                }
            }
        }
        true
    }

    /// What the tree knows of its window, as signals: made where it is first asked for, which
    /// is where the window's view is mounted.
    pub fn window_state(&mut self) -> WindowState {
        let active = self.active;
        *self.state.get_or_insert_with(|| WindowState {
            maximized: signal(false),
            active: signal(active),
            minimized: signal(false),
        })
    }

    /// Tells the tree whether its window is maximized: what reads it runs again.
    pub fn set_maximized(&mut self, on: bool) {
        if let Some(state) = self.state
            && state.maximized.get_untracked() != on
        {
            state.maximized.set(on);
        }
    }

    /// Tells the tree whether its window is minimized: what reads it runs again.
    pub fn set_minimized(&mut self, on: bool) {
        if let Some(state) = self.state
            && state.minimized.get_untracked() != on
        {
            state.minimized.set(on);
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

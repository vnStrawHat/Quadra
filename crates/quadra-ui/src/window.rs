//! The window a view is in (specs/10, "Windows with their own frame"): what a title bar
//! drawn by the app asks of it.

use quadra_reactive::{Signal, expect};
use quadra_tree::{Handle, WindowRequest};

/// The window of the view that asked for it.
#[derive(Clone)]
pub struct Window {
    handle: Handle,
    maximized: Signal<bool>,
}

/// The window the calling view is in. Call it while the view is built.
#[track_caller]
pub fn window() -> Window {
    expect::<Window>()
}

impl Window {
    pub(crate) fn new(handle: Handle, maximized: Signal<bool>) -> Window {
        Window { handle, maximized }
    }

    pub fn minimize(&self) {
        self.handle.ask_window(WindowRequest::Minimize);
    }

    /// Maximizes the window, or puts it back as it was.
    pub fn toggle_maximize(&self) {
        self.handle.ask_window(WindowRequest::ToggleMaximize);
    }

    /// Closes the window, as its close button does.
    pub fn close(&self) {
        self.handle.ask_window(WindowRequest::Close);
    }

    /// Whether the window is maximized: a read that what it is in runs again for.
    pub fn is_maximized(&self) -> bool {
        self.maximized.get()
    }
}

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

    /// Closes the window, as its close button does: if what is in it says it may
    /// (`on_close`).
    pub fn close(&self) {
        self.handle.ask_window(WindowRequest::Close);
    }

    /// Closes the window, asking nothing: after the user said to, in a dialog an `on_close`
    /// opened.
    pub fn close_now(&self) {
        self.handle.ask_window(WindowRequest::CloseNow);
    }

    /// Whether the window is maximized: a read that what it is in runs again for.
    pub fn is_maximized(&self) -> bool {
        self.maximized.get()
    }
}

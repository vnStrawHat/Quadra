//! The window a view is in (specs/10, "Windows with their own frame"): what a title bar
//! drawn by the app asks of it.

use quadra_reactive::{Memo, expect, use_context};
use quadra_tree::{Handle, WindowRequest, WindowState};

/// The window of the view that asked for it.
#[derive(Clone)]
pub struct Window {
    handle: Handle,
    state: WindowState,
}

/// The text scale of the app's windows (specs/12, "Text size"), as the app gives it to views.
#[derive(Clone, Copy)]
pub struct TextScale(pub Memo<f32>);

/// `px` at a text scale of 1, and as many times more as text is larger: the height of a box
/// that holds a line of text (`.height(by_text(28.0))`). Call it while the view is built.
pub fn by_text(px: f32) -> impl Fn() -> f32 + Clone + 'static {
    let scale = use_context::<TextScale>();
    move || px * scale.map_or(1.0, |scale| scale.0.get())
}

/// The window the calling view is in. Call it while the view is built.
#[track_caller]
pub fn window() -> Window {
    expect::<Window>()
}

impl Window {
    pub(crate) fn new(handle: Handle, state: WindowState) -> Window {
        Window { handle, state }
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

    /// How many frames the window has made (`Handle::frames`).
    pub fn frames(&self) -> u64 {
        self.handle.frames()
    }

    /// Whether the window is maximized: a read that what it is in runs again for.
    pub fn is_maximized(&self) -> bool {
        self.state.maximized.get()
    }

    /// Whether the window has the keyboard: a read that what it is in runs again for.
    pub fn is_active(&self) -> bool {
        self.state.active.get()
    }

    /// Whether the window is minimized: a read that what it is in runs again for.
    pub fn is_minimized(&self) -> bool {
        self.state.minimized.get()
    }
}

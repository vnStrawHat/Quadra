//! A headless window: it keeps what it is told, for the test to read.

use quadra_a11y::TreeUpdate;
use quadra_prim::{Rect, Size};

use super::{HeadlessPlatform, HeadlessWindow};
use crate::window::{Cursor, Placement, PlatformWindow, WindowControls, WindowHandle, WindowId};

impl PlatformWindow for HeadlessWindow {
    fn id(&self) -> WindowId {
        self.0.id
    }

    fn handle(&self) -> WindowHandle {
        WindowHandle::Headless
    }

    fn size(&self) -> Size {
        self.0.size.get()
    }

    fn scale(&self) -> f32 {
        self.0.scale.get()
    }

    fn set_title(&self, title: &str) {
        *self.0.title.borrow_mut() = title.to_string();
    }

    fn request_frame(&self) {
        self.0.frame.set(true);
    }

    fn set_cursor(&self, cursor: Cursor) {
        self.0.cursor.set(cursor);
    }

    fn set_text_input(&self, caret: Option<Rect>) {
        self.0.text_input.set(caret);
    }

    fn update_semantics(&self, update: &TreeUpdate) {
        self.0.semantics.borrow_mut().push(update.clone());
    }

    fn minimize(&self) {
        self.0.minimized.set(true);
    }

    fn toggle_maximize(&self) {
        self.0.maximized.set(!self.0.maximized.get());
    }

    fn to_front(&self) {
        self.0.minimized.set(false);
    }

    fn is_maximized(&self) -> bool {
        self.0.maximized.get()
    }

    fn controls(&self) -> WindowControls {
        self.0.controls
    }

    fn placement(&self) -> Placement {
        let ((x, y), size, scale) = (self.0.at.get(), self.0.size.get(), self.0.scale.get());
        let (width, height) = ((size.width * scale) as i32, (size.height * scale) as i32);
        Placement { x, y, width, height, maximized: self.0.maximized.get() }
    }
}

/// What a test reads of a window, and does to it as the user would.
impl HeadlessPlatform {
    /// Whether a window was minimized, and whether it is maximized.
    pub fn is_minimized(&self, window: WindowId) -> bool {
        self.window(window).is_some_and(|w| w.minimized.get())
    }

    pub fn is_maximized(&self, window: WindowId) -> bool {
        self.window(window).is_some_and(|w| w.maximized.get())
    }

    /// Where a window is: as it was opened, moved and resized.
    pub fn placement(&self, window: WindowId) -> Option<Placement> {
        self.window(window).map(|w| HeadlessWindow(w).placement())
    }

    /// The user moves a window.
    pub fn move_window(&self, window: WindowId, x: i32, y: i32) {
        if let Some(w) = self.window(window) {
            w.at.set((x, y));
        }
    }

    /// The title a window was last given.
    pub fn title(&self, window: WindowId) -> Option<String> {
        self.window(window).map(|w| w.title.borrow().clone())
    }

    /// From now on the platform says of a window's buttons and of the names of keys what
    /// the system it is built for does: for a tool that draws what a user of that system
    /// sees. A test leaves it, so that it says the same on every machine.
    pub fn as_the_system(&self) {
        self.0.system.set(true);
    }
}

//! A headless window: it keeps what it is told, for the test to read.

use quadra_a11y::TreeUpdate;
use quadra_prim::{Rect, Size};

use super::HeadlessWindow;
use crate::window::{Cursor, Placement, PlatformWindow, WindowHandle, WindowId};

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

    fn is_maximized(&self) -> bool {
        self.0.maximized.get()
    }

    fn placement(&self) -> Placement {
        let ((x, y), size, scale) = (self.0.at.get(), self.0.size.get(), self.0.scale.get());
        let (width, height) = ((size.width * scale) as i32, (size.height * scale) as i32);
        Placement { x, y, width, height, maximized: self.0.maximized.get() }
    }
}

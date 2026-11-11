//! Windows: how one is asked for, and what it can be told.

use quadra_prim::Size;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub u32);

/// What a renderer needs to make a surface for a window: one variant per windowing system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowHandle {
    Win32 { hwnd: isize },
    Headless,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowOptions {
    pub title: String,
    /// The content size, in logical pixels.
    pub size: Size,
    pub resizable: bool,
    /// Opens centred on the primary display; otherwise where the OS puts it.
    pub centered: bool,
    /// Above 0: no side is longer than this fraction of the display.
    pub fit: f32,
    /// A window that is not shown: tests, and tools that only draw.
    pub visible: bool,
}

impl WindowOptions {
    pub fn new(title: &str) -> WindowOptions {
        WindowOptions {
            title: title.to_string(),
            size: Size::new(960.0, 600.0),
            resizable: true,
            centered: false,
            fit: 0.0,
            visible: true,
        }
    }

    pub fn size(mut self, width: f32, height: f32) -> WindowOptions {
        self.size = Size::new(width, height);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> WindowOptions {
        self.resizable = resizable;
        self
    }

    pub fn centered(mut self) -> WindowOptions {
        self.centered = true;
        self
    }

    pub fn fit(mut self, fraction: f32) -> WindowOptions {
        self.fit = fraction;
        self
    }

    pub fn hidden(mut self) -> WindowOptions {
        self.visible = false;
        self
    }
}

/// A window size capped by [`WindowOptions::fit`] against a display, both in logical pixels.
pub fn fit_size(size: Size, display: Size, fit: f32) -> Size {
    if fit > 0.0 && display.width > 0.0 && display.height > 0.0 {
        Size::new(size.width.min(display.width * fit), size.height.min(display.height * fit))
    } else {
        size
    }
}

/// An open window. Dropping it closes the window.
pub trait PlatformWindow {
    fn id(&self) -> WindowId;
    fn handle(&self) -> WindowHandle;
    /// The content size, in logical pixels.
    fn size(&self) -> Size;
    /// Device pixels per logical pixel.
    fn scale(&self) -> f32;
    fn set_title(&self, title: &str);
    /// Asks for [`crate::PlatformHandler::frame`] as soon as the window can show a frame.
    fn request_frame(&self);
}

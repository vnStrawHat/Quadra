//! Windows: how one is asked for, and what it can be told.

use quadra_a11y::TreeUpdate;
use quadra_prim::{Rect, Size};

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
    /// The smallest content size the user can drag the window to, in logical pixels; a
    /// window asked to open smaller (by `size`, `fit` or a placement) opens this big.
    pub min_size: Size,
    pub resizable: bool,
    /// Opens centred on the primary display; otherwise where the OS puts it.
    pub centered: bool,
    /// Above 0: no side is longer than this fraction of the display.
    pub fit: f32,
    /// A window that is not shown: tests, and tools that only draw.
    pub visible: bool,
    /// The app draws the title bar: the content covers where the OS would draw its own, and
    /// the app says what each point of it is ([`crate::PlatformHandler::hit_test`]).
    pub custom_frame: bool,
    /// The app keeps where the window was under this name (`window.<key>` in its settings).
    pub remember: Option<String>,
    /// Where the window opens, if that place is on a display; otherwise as the rest says.
    pub placement: Option<Placement>,
}

/// Where a window is, as the OS keeps it: its frame when it is not maximized, in device
/// pixels of the desktop, and whether it is maximized. What the app stores and gives back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
}

impl WindowOptions {
    pub fn new(title: &str) -> WindowOptions {
        WindowOptions {
            title: title.to_string(),
            size: Size::new(960.0, 600.0),
            min_size: Size::ZERO,
            resizable: true,
            centered: false,
            fit: 0.0,
            visible: true,
            custom_frame: false,
            remember: None,
            placement: None,
        }
    }

    pub fn size(mut self, width: f32, height: f32) -> WindowOptions {
        self.size = Size::new(width, height);
        self
    }

    pub fn min_size(mut self, width: f32, height: f32) -> WindowOptions {
        self.min_size = Size::new(width, height);
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

    pub fn custom_frame(mut self) -> WindowOptions {
        self.custom_frame = true;
        self
    }

    /// The window opens where it was when it last closed, and as big (specs/15, "Window
    /// state"); `key` tells the windows of an app apart.
    pub fn remember(mut self, key: &str) -> WindowOptions {
        self.remember = Some(key.to_string());
        self
    }
}

/// What a point of a window with its own frame is to the OS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HitRegion {
    /// The app's: pointer events come to it.
    #[default]
    Client,
    /// The window is dragged by it, and a double click maximizes it.
    Caption,
    /// The app's maximize button: the OS shows its snap layouts over it.
    Maximize,
}

/// The shape of the pointer over a window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Cursor {
    #[default]
    Arrow,
    /// Over text that can be selected or edited.
    Text,
    /// Over what a click follows: a link.
    Pointer,
    Crosshair,
    Move,
    NotAllowed,
    /// Over an edge that is dragged left and right, up and down, or along a diagonal.
    ResizeHorizontal,
    ResizeVertical,
    ResizeNwse,
    ResizeNesw,
    /// The app is busy; `Progress` when it still answers.
    Wait,
    Progress,
    Help,
}

/// A window size capped by [`WindowOptions::fit`] against a display, both in logical pixels.
pub fn fit_size(size: Size, display: Size, fit: f32) -> Size {
    if fit > 0.0 && display.width > 0.0 && display.height > 0.0 {
        Size::new(size.width.min(display.width * fit), size.height.min(display.height * fit))
    } else {
        size
    }
}

/// A size grown, side by side, to at least `min`.
pub fn at_least(size: Size, min: Size) -> Size {
    Size::new(size.width.max(min.width), size.height.max(min.height))
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
    /// The shape the pointer has while it is over the window's content.
    fn set_cursor(&self, cursor: Cursor);
    /// Says whether the focus is on something that takes text, and where its caret is, in
    /// logical pixels of the window: an input method is on only then, and opens its
    /// candidates beside that caret.
    fn set_text_input(&self, caret: Option<Rect>);
    /// Tells assistive technology what is new in the window's semantic tree. Called only
    /// after [`crate::PlatformEvent::SemanticsWanted`]; the first update is the whole tree.
    fn update_semantics(&self, update: &TreeUpdate);
    fn minimize(&self);
    /// Maximizes the window, or puts it back as it was.
    fn toggle_maximize(&self);
    /// Shows the window again if it is minimized, and brings it to the front with the
    /// keyboard, as far as the OS lets this process.
    fn to_front(&self);
    fn is_maximized(&self) -> bool;
    /// Where the window is now: what [`WindowOptions::placement`] opens it at again.
    fn placement(&self) -> Placement;
}

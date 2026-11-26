//! Quadra: what an app depends on (specs/02, "Crates"). It gathers the pieces an app names
//! and builds an [`App`] on the backends of the operating system.
//!
//! ```no_run
//! use quadra::{WindowOptions, column, signal, text};
//!
//! fn main() -> quadra::Result<()> {
//!     let view = || {
//!         let count = signal(0);
//!         column().padding(16.0).child(text(move || count.get().to_string()))
//!     };
//!     quadra::app()?.window(WindowOptions::new("Counter").size(360.0, 260.0), view)?.run()
//! }
//! ```
//!
//! # Where things are
//!
//! - State: [`signal`], [`memo`], [`effect`], [`provide`] and [`use_context`] (`quadra-reactive`).
//! - Views: [`div`], [`row`], [`column()`], [`text()`], [`text_input`], [`scroll`],
//!   [`uniform_list`], [`overlay`] and the rest of `quadra-view`; styles by state on a [`Div`],
//!   colours by [`Token`] of a [`Theme`].
//! - Keys and commands: [`Action`], [`Keymap`] (`App::keymap`).
//! - Work off the UI thread and over time: [`spawn`], [`background`], [`inbox`], [`timeout`]
//!   (`quadra-async`).
//! - The app's services (specs/15): its folders and [`Settings`] (`App::dirs`), windows that are
//!   remembered ([`WindowOptions::remember`]), [`log`](mod@log), [`time`](mod@time), its language ([`I18n`],
//!   [`t`], [`Locale`]), pages ([`router`]) and closing (`Div::on_close`).
//! - The components and their look are the crate `quadra-design`; what the OS does beyond
//!   windows (dialogs, the tray, one instance) is `quadra-shell`, which an app adds itself.

#[cfg(any(windows, target_os = "macos"))]
use std::rc::Rc;

#[cfg(target_os = "macos")]
use quadra_native_mac::{CoreText, MacPlatform, MetalRenderer};
#[cfg(windows)]
use quadra_native_win::{D3d11Renderer, DirectWrite, WinPlatform};

pub use quadra_app::{
    AppDirs, AppError, Arg, Feature, FrameReport, I18n, Language, Locale, Result, Router, Settings, Stored,
    SystemText, TextSize, Windows, app_keymap, language, log, router, t, t_with, text_size, time,
};
pub use quadra_async::{
    Background, CancelToken, Full, Inbox, Panicked, Sender, Task, Timer, background, background_with,
    debounce, inbox, inbox_with, interval, sleep, spawn, throttle, timeout,
};
pub use quadra_layout::{Align, Direction, Display, Justify, Length, Overflow, Position, Track};
pub use quadra_platform::{
    Appearance, Clipboard, Cursor, DroppedFiles, DroppedText, HitRegion, Key, Mods, MouseButton, Placement,
    WindowId, WindowOptions,
};
pub use quadra_prim::{Color, Corners, Ease, Edges, Point, Rect, Size};
pub use quadra_reactive::{
    Memo, Scope, Signal, batch, effect, expect, memo, on_cleanup, provide, signal, untrack, use_context,
};

pub use quadra_tree::{
    Action, Actions, Along, BorderStyle, Fit, FocusNext, FocusPrevious, GoBack, GoForward, Gradient,
    GradientKind, KeyEvent, Keymap, KeymapError, Keystroke, NodeId, Paint, PointerEvent, PointerKind, Role,
    SemanticAction, Side, State, Theme, Token,
};
pub use quadra_view::{
    AnyView, Div, ImageSource, ImageView, Images, IntoProp, MountCx, NodeRef, Place, Refine, Region, Span,
    TextView, Timeline, View, Window, by_text, column, div, dynamic, each, each_frame, image, inline,
    overlay, rich_text, row, scroll, scroll_strip, scroll_x, show, span, text, text_area, text_input,
    timeline, tween, uniform_list, virtual_list, window,
};

/// An app on the backends of this operating system: Win32 windows, the D3D11 renderer and
/// DirectWrite text.
#[cfg(windows)]
pub type App = quadra_app::App<D3d11Renderer>;

/// Makes the app: the platform, the GPU device and the text backend.
#[cfg(windows)]
pub fn app() -> Result<App> {
    let platform = WinPlatform::new()?;
    // Before the renderer: its worker starts the GPU driver, which holds up whatever loads
    // a library meanwhile.
    let text = DirectWrite::new().map_err(AppError::Text)?;
    let renderer = D3d11Renderer::starting(&platform);
    Ok(App::with(Rc::new(platform), renderer, Box::new(text)))
}

/// An app on the backends of this operating system: AppKit windows, the Metal renderer and
/// Core Text (specs/23).
#[cfg(target_os = "macos")]
pub type App = quadra_app::App<MetalRenderer>;

/// Makes the app: the platform, the GPU device and the text backend. On the main thread
/// alone, as AppKit is.
#[cfg(target_os = "macos")]
pub fn app() -> Result<App> {
    let platform = MacPlatform::new()?;
    let text = CoreText::new().map_err(AppError::Text)?;
    let renderer = MetalRenderer::new()?;
    Ok(App::with(Rc::new(platform), renderer, Box::new(text)))
}

/// An app where Quadra has no native crate (specs/23): the type an app names, so that it
/// builds and its tests run there; [`app`] makes none.
#[cfg(not(any(windows, target_os = "macos")))]
pub type App = quadra_app::App<quadra_render::NullRenderer>;

/// Says that Quadra opens no window on this operating system (specs/23).
#[cfg(not(any(windows, target_os = "macos")))]
pub fn app() -> Result<App> {
    let why = "Quadra has no native backend for this operating system yet";
    Err(AppError::Platform(quadra_platform::PlatformError(why.to_string())))
}

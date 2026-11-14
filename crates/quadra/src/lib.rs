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
//!   [`uniform_list`], [`overlay`] and the rest of `quadra-ui`; styles by state on a [`Div`],
//!   colours by [`Token`] of a [`Theme`].
//! - Keys and commands: [`Action`], [`Keymap`] (`App::keymap`).
//! - Work off the UI thread and over time: [`spawn`], [`background`], [`inbox`], [`timeout`]
//!   (`quadra-async`).
//! - The app's services (specs/15): its folders and [`Settings`] (`App::dirs`), windows that are
//!   remembered ([`WindowOptions::remember`]), [`log`](mod@log), [`time`](mod@time), its language ([`I18n`],
//!   [`t`], [`Locale`]), pages ([`router`]) and closing (`Div::on_close`).
//! - The components and their look are the crate `quadra-kit`; what the OS does beyond
//!   windows (dialogs, the tray, one instance) is `quadra-shell`, which an app adds itself.

use std::rc::Rc;

use quadra_native_win::{D3d11Renderer, DirectWrite, WinPlatform};

pub use quadra_app::{
    AppDirs, AppError, Arg, Feature, FrameReport, I18n, Language, Locale, Result, Router, Settings, Stored,
    language, log, router, t, t_with, time,
};
pub use quadra_async::{
    Background, CancelToken, Full, Inbox, Panicked, Sender, Task, Timer, background, background_with,
    debounce, inbox, inbox_with, interval, sleep, spawn, throttle, timeout,
};
pub use quadra_layout::{Align, Direction, Display, Justify, Length, Overflow, Position, Track};
pub use quadra_platform::{
    Appearance, Cursor, DroppedFiles, DroppedText, HitRegion, Key, Mods, MouseButton, Placement, WindowId,
    WindowOptions,
};
pub use quadra_prim::{Color, Corners, Ease, Edges, Point, Rect, Size};
pub use quadra_reactive::{
    Memo, Scope, Signal, batch, effect, expect, memo, on_cleanup, provide, signal, untrack, use_context,
};

pub use quadra_tree::{
    Action, Along, BorderStyle, Fit, FocusNext, FocusPrevious, GoBack, GoForward, Gradient, GradientKind,
    KeyEvent, Keymap, KeymapError, Keystroke, Paint, PointerEvent, PointerKind, Role, SemanticAction, Side,
    State, Theme, Token,
};
pub use quadra_ui::{
    AnyView, Div, ImageSource, ImageView, Images, IntoProp, MountCx, NodeRef, Place, Refine, Region, Span,
    TextView, Timeline, View, Window, column, div, dynamic, each, each_frame, image, inline, overlay,
    rich_text, row, scroll, scroll_strip, scroll_x, show, span, text, text_area, text_input, timeline, tween,
    uniform_list, virtual_list, window,
};

/// An app on the backends of this operating system: Win32 windows, the D3D11 renderer and
/// DirectWrite text.
pub type App = quadra_app::App<D3d11Renderer>;

/// Makes the app: the platform, the GPU device and the text backend.
pub fn app() -> Result<App> {
    let platform = WinPlatform::new()?;
    // Before the renderer: its worker starts the GPU driver, which holds up whatever loads
    // a library meanwhile.
    let text = DirectWrite::new().map_err(AppError::Text)?;
    let renderer = D3d11Renderer::starting(&platform);
    Ok(App::with(Rc::new(platform), renderer, Box::new(text)))
}

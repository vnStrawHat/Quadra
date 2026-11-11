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

use std::rc::Rc;

use quadra_native_win::{D3d11Renderer, DirectWrite, WinPlatform};

pub use quadra_app::{AppError, FrameReport, Result};
pub use quadra_layout::{Align, Direction, Justify, Length, Overflow};
pub use quadra_platform::{Cursor, Key, Mods, MouseButton, WindowId, WindowOptions};
pub use quadra_prim::{Color, Corners, Edges, Point, Rect, Size};
pub use quadra_reactive::{
    Memo, Scope, Signal, batch, effect, expect, memo, on_cleanup, provide, signal, untrack, use_context,
};
pub use quadra_tree::{
    Action, Along, FocusNext, FocusPrevious, KeyEvent, Keymap, KeymapError, Keystroke, PointerEvent,
    PointerKind, Role, SemanticAction, Side, State,
};
pub use quadra_ui::{
    AnyView, Div, NodeRef, Place, Refine, Region, View, column, div, dynamic, each, each_frame, overlay, row,
    scroll, show, text, text_input, uniform_list, virtual_list,
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

//! Quadra's terminal (specs/18): a session over the terminal engine, the view of its grid,
//! and the input that goes back to the program. The transport is the app's: a pseudo console
//! (`quadra-shell`), a network channel, or `echo()`.

mod actions;
mod feed;
mod keys;
mod look;
mod paint;
mod pointer;
mod session;
mod status;
mod view;
mod widget;

/// The engine, for the types a session's API names.
pub use oneterm_vt as vt;

pub use actions::TermSettings;
pub use feed::{Feeder, Notice};
pub use keys::{TERMINAL_CONTEXT, TermCommand, bind_keys};
pub use look::{TermColors, TermLook};
pub use session::{Link, Session, echo};
pub use status::ProgramState;
pub use view::{Terminal, terminal};
pub use widget::TerminalWidget;

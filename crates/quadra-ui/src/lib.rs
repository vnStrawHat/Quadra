//! Quadra's views (specs/05-views.md): UI is described once, mounted into the retained tree,
//! and from then on driven by signals.
//!
//! ```
//! use quadra_reactive::signal;
//! use quadra_ui::{View, row, text};
//!
//! fn counter() -> impl View {
//!     let count = signal(0);
//!     row().gap(8.0).child(text(move || count.get().to_string()))
//! }
//! # let _ = counter();
//! ```
//!
//! `counter` runs when it is mounted and never again. Writing the signal runs the one binding
//! that read it, which sets the text of one node.

mod div;
mod flow;
mod input;
mod list;
mod node_ref;
mod overlay;
mod prop;
mod scroll;
mod text;
mod tween;
mod view;

pub use div::{Div, Refine, column, div, row};
pub use flow::{Region, dynamic, each, mount_in, show};
pub use input::{TextInput, text_area, text_input};
pub use list::{ListView, uniform_list, virtual_list};
pub use node_ref::NodeRef;
pub use overlay::{OverlayView, Place, overlay};
pub use prop::{IntoProp, Prop};
pub use scroll::scroll;
pub use text::{TextView, text};
pub use tween::tween;
pub use view::{AnyView, MountCx, View, each_frame, mount_root, mount_with};

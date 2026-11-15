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
mod drawing;
mod flow;
mod image;
mod input;
mod list;
mod node_ref;
mod overlay;
mod prop;
mod rich;
mod scroll;
mod scrollbar;
mod text;
mod tween;
mod view;
mod window;

pub use div::{Div, Refine, Timeline, column, div, row, timeline};
pub use drawing::{DrawingView, drawing};
pub use flow::{Region, dynamic, each, mount_in, show};
pub use image::{ImageSource, ImageView, Images, InlineImage, image, image_info, image_pixels, inline_image};
pub use input::{TextInput, text_area, text_input};
pub use list::{ListView, uniform_list, virtual_list};
pub use node_ref::NodeRef;
pub use overlay::{OverlayView, Place, overlay};
pub use prop::{IntoProp, Prop};
pub use rich::{RichTextView, Span, inline, rich_text, span};
pub use scroll::{scroll, scroll_strip, scroll_x};
pub use text::{TextView, text};
pub use tween::tween;
pub use view::{AnyView, MountCx, View, each_frame, mount_root, mount_with};
pub use window::{TextScale, Window, by_text, window};

//! Quadra's retained node tree (specs/04-tree.md): each window owns one tree of nodes that
//! lives across frames. A frame is a sequence of passes, each over the nodes flagged for it.
//!
//! This crate joins the reactive runtime (bindings queue writes), the layout tree (rects kept
//! on nodes), paragraphs (text nodes) and layers (painted output kept and patched).

mod dispatch;
mod fonts;
mod frame;
mod input;
mod layers;
mod node;
mod paint;
mod shared;
mod structure;
mod text;
mod tree;
mod verify;
mod widget;

pub use dispatch::PointerInput;
pub use fonts::{Fonts, GlyphKey, GlyphMeta, GlyphTile, SUBPIXEL_STEPS};
pub use frame::{FrameStats, PassTimes};
pub use input::{PointerEvent, PointerKind};
pub use node::{NodeId, PaintStyle, Style};
pub use shared::Handle;
pub use text::Text;
pub use tree::Tree;
pub use widget::{MeasureCx, PaintCx, Widget};

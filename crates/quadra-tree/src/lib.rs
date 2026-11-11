//! Quadra's retained node tree (specs/04-tree.md): each window owns one tree of nodes that
//! lives across frames. A frame is a sequence of passes, each over the nodes flagged for it.
//!
//! This crate joins the reactive runtime (bindings queue writes), the layout tree (rects kept
//! on nodes), paragraphs (text nodes) and layers (painted output kept and patched).

mod frame;
mod node;
mod paint;
mod shared;
mod text;
mod tree;
mod widget;

pub use frame::FrameStats;
pub use node::{NodeId, PaintStyle, Style};
pub use shared::Handle;
pub use text::Text;
pub use tree::Tree;
pub use widget::{MeasureCx, PaintCx, Widget};

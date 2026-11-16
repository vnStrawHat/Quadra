//! Quadra's layout engine (specs/06-layout.md): flex and grid on a tree that is kept across frames.
//!
//! A parent asks a child a request and the child answers with a size. Nodes keep their
//! answers, so a change costs the changed node and its ancestors up to the first one that
//! answers as before.
//!
//! ```
//! use quadra_layout::{LayoutTree, Length, NoMeasure, RootSize, Style};
//! use quadra_prim::{Rect, Size};
//!
//! let mut tree = LayoutTree::new();
//! let side = tree.insert(Style { width: Length::Px(200.0), ..Style::default() });
//! let body = tree.insert(Style { grow: 1.0, ..Style::default() });
//! let root = tree.insert(Style::default());
//! tree.set_children(root, &[side, body]);
//! tree.layout(root, RootSize::Fill(Size::new(800.0, 600.0)), &mut NoMeasure);
//! assert_eq!(tree.rect(body), Rect::new(200.0, 0.0, 600.0, 600.0));
//! ```

mod cache;
mod flex;
mod flex_items;
mod flex_place;
mod grid;
mod grid_layout;
mod grid_place;
mod grid_tracks;
mod incremental;
mod solve;
mod style;
mod tree;

pub use grid::{GridStyle, Track, TrackSize};
pub use incremental::{RootSize, Stats};
pub use solve::{Holds, Known, Measure, NoMeasure};
pub use style::{Align, AutoMargins, Direction, Display, Insets, Justify, Length, Overflow, Position, Style};
pub use tree::{LayoutId, LayoutTree, RectChange};

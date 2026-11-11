//! The value types every other Quadra crate shares (specs/02-architecture.md, Crates).
//!
//! Lengths are `f32` logical pixels unless a name says otherwise. All types are plain `Copy`
//! values with public fields.

mod affine;
mod color;
mod geom;

pub use affine::Affine;
pub use color::Color;
pub use geom::{Corners, Edges, Point, Rect, Size};

//! Quadra's renderer (specs/07-render.md): instances kept in layers, and the boundary to a
//! GPU backend.
//!
//! The tree repaints only what changed into its layers' display lists. A backend draws every
//! visible layer each frame from buffers that mostly did not change, and uploads only what a
//! layer says changed.

mod atlas;
mod hasher;
mod instance;
mod layer;
mod null;
mod packing;
mod renderer;
mod scene;

pub use atlas::{Atlas, AtlasStats, Tile, TileRef};
pub use hasher::{FastBuild, FastHasher};
pub use instance::{
    Clip, FLAG_BORDER, FLAG_GLYPH, FLAG_GRADIENT, Instance, Kind, Line, LineMode, Quad, Shadow, Sprite,
    push_quad,
};
pub use layer::{Dirty, Layer, LayerConstants, LayerId, Rebuild};
pub use null::NullRenderer;
pub use packing::{AtlasCore, AtlasKind, TileRect, Upload};
pub use renderer::{DeviceSize, Pixels, RenderError, RenderStats, Renderer, SurfaceId, SurfaceTarget};
pub use scene::{Scene, SceneId, Step, World};

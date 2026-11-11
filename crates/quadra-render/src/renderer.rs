//! The boundary to a GPU backend (specs/07, "The `Renderer` trait").

use std::fmt;

use quadra_prim::Color;

use crate::scene::{Scene, SceneId};

/// A size in device pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeviceSize {
    pub width: u32,
    pub height: u32,
}

/// Where a surface draws. Defined here and not by the platform crate: the two do not know each
/// other, and the per-OS native crate converts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceTarget {
    /// A Win32 window, by its handle.
    Win32 { hwnd: isize },
    /// A texture that can be read back: tests and snapshots.
    Offscreen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SurfaceId(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderError {
    /// The GPU device is gone. Call [`Renderer::recover`], then draw again.
    DeviceLost,
    /// The surface does not exist, or cannot do what was asked.
    BadSurface,
    /// Anything else a backend reports, in its own words.
    Backend(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::DeviceLost => write!(f, "the GPU device was lost"),
            RenderError::BadSurface => write!(f, "the surface does not exist or cannot do this"),
            RenderError::Backend(what) => write!(f, "the renderer failed: {what}"),
        }
    }
}

impl std::error::Error for RenderError {}

/// RGBA pixels, 8 bits a channel, rows from the top.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pixels {
    pub size: DeviceSize,
    pub rgba: Vec<u8>,
}

/// What the last `draw` did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderStats {
    /// Frames drawn since the renderer was made.
    pub frames: u64,
    pub draw_calls: u32,
    /// Layers bound to draw from: one per layer, and one more each time a layer continues
    /// after a child.
    pub layers_bound: u32,
    pub instances_drawn: u32,
    /// Instances sent to the GPU: ranges overwritten in place, and whole rebuilt layers.
    pub instances_uploaded: u32,
    pub layers_uploaded: u32,
}

pub trait Renderer {
    fn create_surface(&mut self, target: SurfaceTarget, size: DeviceSize) -> Result<SurfaceId, RenderError>;
    fn resize_surface(&mut self, surface: SurfaceId, size: DeviceSize) -> Result<(), RenderError>;
    fn drop_surface(&mut self, surface: SurfaceId);

    /// Draws the scene: uploads what its layers say changed, frees what it dropped, and runs
    /// its steps. The scene's layers are clean afterwards.
    fn draw(&mut self, surface: SurfaceId, scene: &mut Scene, clear: Color) -> Result<(), RenderError>;
    fn present(&mut self, surface: SurfaceId) -> Result<(), RenderError>;

    /// The pixels of an offscreen surface.
    fn read_pixels(&mut self, surface: SurfaceId) -> Result<Pixels, RenderError>;

    /// Frees everything kept for a scene: its window closed.
    fn forget_scene(&mut self, scene: SceneId);

    /// Makes a new device after [`RenderError::DeviceLost`]. Everything is uploaded again by
    /// the next draws.
    fn recover(&mut self) -> Result<(), RenderError>;

    fn stats(&self) -> RenderStats;
}

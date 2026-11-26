//! The Metal renderer (specs/07; specs/23): it keeps a buffer per layer, uploads what a layer
//! says changed, and draws the scene's steps. The Windows renderer's twin: the same instances,
//! the same shader in Metal's language, the same pixels.

mod atlas;
mod backdrop;
mod frame;
mod gpu;
mod layers;
mod surface;

use quadra_prim::Color;
use quadra_render::{
    AtlasCore, AtlasKind, DeviceSize, Pixels, RenderError, RenderStats, Renderer, Scene, SceneId, Step,
    SurfaceId, SurfaceTarget,
};

use atlas::AtlasTexture;
use backdrop::Backdrop;
use gpu::Gpu;
use layers::LayerBuffers;
use surface::Surface;

pub struct MetalRenderer {
    gpu: Gpu,
    /// Each with what it was made for, to make it again on a new device.
    surfaces: Vec<Option<(Surface, SurfaceTarget)>>,
    layers: LayerBuffers,
    /// The glyph and image atlases, as texture arrays.
    coverage: Option<AtlasTexture>,
    color: Option<AtlasTexture>,
    /// What is behind a box that blurs it, kept for the next one.
    backdrop: Option<Backdrop>,
    steps: Vec<Step>,
    stats: RenderStats,
}

impl MetalRenderer {
    /// A renderer on the system's default device.
    pub fn new() -> Result<MetalRenderer, RenderError> {
        Ok(MetalRenderer {
            gpu: Gpu::new()?,
            surfaces: Vec::new(),
            layers: LayerBuffers::default(),
            coverage: None,
            color: None,
            backdrop: None,
            steps: Vec::new(),
            stats: RenderStats::default(),
        })
    }

    fn surface(&self, id: SurfaceId) -> Result<&Surface, RenderError> {
        let slot = self.surfaces.get(id.0 as usize).and_then(Option::as_ref);
        slot.map(|(surface, _)| surface).ok_or(RenderError::BadSurface)
    }
}

impl Renderer for MetalRenderer {
    fn create_surface(&mut self, target: SurfaceTarget, size: DeviceSize) -> Result<SurfaceId, RenderError> {
        let surface = Surface::new(&self.gpu, target, size)?;
        self.surfaces.push(Some((surface, target)));
        Ok(SurfaceId(self.surfaces.len() as u32 - 1))
    }

    fn resize_surface(&mut self, surface: SurfaceId, size: DeviceSize) -> Result<(), RenderError> {
        let slot = self.surfaces.get_mut(surface.0 as usize).and_then(Option::as_mut);
        let (old, target) = slot.ok_or(RenderError::BadSurface)?;
        *old = Surface::new(&self.gpu, *target, size)?;
        Ok(())
    }

    fn drop_surface(&mut self, surface: SurfaceId) {
        if let Some(slot) = self.surfaces.get_mut(surface.0 as usize) {
            *slot = None;
        }
    }

    fn sync_atlas(&mut self, atlas: &mut AtlasCore) -> Result<(), RenderError> {
        let slot = match atlas.kind() {
            AtlasKind::Coverage => &mut self.coverage,
            AtlasKind::Color => &mut self.color,
        };
        AtlasTexture::sync(slot, &self.gpu, atlas)
    }

    fn draw(&mut self, surface: SurfaceId, scene: &mut Scene, clear: Color) -> Result<(), RenderError> {
        self.frame(surface, scene, clear)
    }

    /// Nothing to show: a surface so far is a texture that is read back.
    fn present(&mut self, surface: SurfaceId) -> Result<(), RenderError> {
        self.surface(surface).map(|_| ())
    }

    fn read_pixels(&mut self, surface: SurfaceId) -> Result<Pixels, RenderError> {
        Ok(self.surface(surface)?.read())
    }

    fn forget_scene(&mut self, scene: SceneId) {
        self.layers.forget(scene);
    }

    /// A new device, with every surface made again on it and nothing else kept: the next
    /// draws upload everything.
    fn recover(&mut self) -> Result<(), RenderError> {
        self.gpu = Gpu::new()?;
        self.layers.clear();
        (self.coverage, self.color, self.backdrop) = (None, None, None);
        for (surface, target) in self.surfaces.iter_mut().flatten() {
            *surface = Surface::new(&self.gpu, *target, surface.size)?;
        }
        Ok(())
    }

    fn stats(&self) -> RenderStats {
        self.stats
    }
}

//! The D3D11 renderer (specs/07): it keeps a buffer per layer, uploads what a layer says
//! changed, and draws the scene's steps.

use quadra_prim::{Color, Rect};
use quadra_render::{
    AtlasCore, AtlasKind, DeviceSize, Pixels, RenderError, RenderStats, Renderer, Scene, SceneId, Step,
    SurfaceId, SurfaceTarget,
};
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct3D11::D3D11_VIEWPORT;

use crate::atlas::AtlasTexture;
use crate::backdrop::Backdrop;
use crate::gpu::{Device, Gpu};
use crate::layers::LayerBuffers;
use crate::pacing::Pacing;
use crate::platform::WinPlatform;
use crate::surface::Surface;
use crate::timing::Timing;
use crate::upgrade::Upgrade;

pub struct D3d11Renderer {
    /// The device, or the one on its way: every call that can fail waits for it first.
    gpu: Gpu,
    surfaces: Vec<Option<Surface>>,
    /// Where the loop finds each window's frame-latency handle.
    pacing: Pacing,
    layers: LayerBuffers,
    /// The glyph and image atlases, as texture arrays.
    coverage: Option<AtlasTexture>,
    color: Option<AtlasTexture>,
    /// What is behind a box that blurs it, kept for the next one.
    backdrop: Option<Backdrop>,
    steps: Vec<Step>,
    stats: RenderStats,
    /// What frames cost the GPU.
    timing: Timing,
    /// The hardware device on its way, while the first frames are drawn on the software one.
    pub(crate) upgrade: Option<Upgrade>,
}

impl D3d11Renderer {
    /// A renderer on the hardware device, or on the software rasterizer: no GPU, the same
    /// pixels on every machine (tests, and the first frames of a window).
    pub fn new(software: bool) -> Result<D3d11Renderer, RenderError> {
        Ok(D3d11Renderer::on(Gpu::made(Device::new(software)?), Pacing::default()))
    }

    pub(crate) fn on(gpu: Gpu, pacing: Pacing) -> D3d11Renderer {
        D3d11Renderer {
            gpu,
            surfaces: Vec::new(),
            pacing,
            layers: LayerBuffers::default(),
            coverage: None,
            color: None,
            backdrop: None,
            steps: Vec::new(),
            stats: RenderStats::default(),
            timing: Timing::default(),
            upgrade: None,
        }
    }

    /// A renderer whose window surfaces pace the frames of that platform's loop: a frame a
    /// window asked for is given when its swap chain can take one (specs/10).
    pub fn for_platform(platform: &WinPlatform, software: bool) -> Result<D3d11Renderer, RenderError> {
        Ok(D3d11Renderer::on(Gpu::made(Device::new(software)?), platform.0.pacing.clone()))
    }

    pub fn is_software(&self) -> bool {
        self.gpu.is_software()
    }

    fn surface(&self, id: SurfaceId) -> Result<&Surface, RenderError> {
        self.surfaces.get(id.0 as usize).and_then(Option::as_ref).ok_or(RenderError::BadSurface)
    }

    /// Makes a surface and tells the loop what to wait on for it.
    fn make_surface(&self, target: SurfaceTarget, size: DeviceSize) -> Result<Surface, RenderError> {
        // Before the hardware device is there, a window is shown by copying (specs/07).
        let surface = Surface::new(&self.gpu, target, size, self.upgrade.is_some())?;
        if let Surface::Window(window) = &surface {
            self.pacing.set(window.hwnd, Some(window.waitable));
        }
        Ok(surface)
    }
}

/// A scissor in whole pixels inside the surface.
fn scissor(rect: Rect, size: DeviceSize) -> RECT {
    let clamp = |v: f32, max: u32| v.round().clamp(0.0, max as f32) as i32;
    RECT {
        left: clamp(rect.x, size.width),
        top: clamp(rect.y, size.height),
        right: clamp(rect.right(), size.width),
        bottom: clamp(rect.bottom(), size.height),
    }
}

impl Renderer for D3d11Renderer {
    fn create_surface(&mut self, target: SurfaceTarget, size: DeviceSize) -> Result<SurfaceId, RenderError> {
        self.gpu.ready()?;
        let surface = self.make_surface(target, size)?;
        self.surfaces.push(Some(surface));
        Ok(SurfaceId(self.surfaces.len() as u32 - 1))
    }

    fn resize_surface(&mut self, surface: SurfaceId, size: DeviceSize) -> Result<(), RenderError> {
        self.gpu.ready()?;
        let slot = self.surfaces.get_mut(surface.0 as usize).and_then(Option::as_mut);
        slot.ok_or(RenderError::BadSurface)?.resize(&self.gpu, size)
    }

    fn drop_surface(&mut self, surface: SurfaceId) {
        if let Some(Surface::Window(window)) =
            self.surfaces.get_mut(surface.0 as usize).and_then(Option::take)
        {
            self.pacing.set(window.hwnd, None);
        }
    }

    fn sync_atlas(&mut self, atlas: &mut AtlasCore) -> Result<(), RenderError> {
        self.gpu.ready()?;
        let slot = match atlas.kind() {
            AtlasKind::Coverage => &mut self.coverage,
            AtlasKind::Color => &mut self.color,
        };
        AtlasTexture::sync(slot, &self.gpu, atlas)
    }

    fn draw(&mut self, surface: SurfaceId, scene: &mut Scene, clear: Color) -> Result<(), RenderError> {
        self.gpu.ready()?;
        if self.wants_frame() {
            return Err(RenderError::DeviceLost); // a better device waits: `recover` takes it
        }
        let (size, target) = {
            let s = self.surface(surface)?;
            if let Surface::Window(window) = s {
                window.set_background(clear);
            }
            (s.size(), s.target()?)
        };
        for layer in scene.take_dropped() {
            self.layers.remove(scene.id(), layer);
        }
        let mut steps = std::mem::take(&mut self.steps);
        let bounds = Rect::new(0.0, 0.0, size.width as f32, size.height as f32);
        scene.steps(bounds, &mut steps);
        self.stats = RenderStats { frames: self.stats.frames + 1, ..RenderStats::default() };
        self.timing.begin(&self.gpu.device, &self.gpu.context);

        let viewport = [size.width as f32, size.height as f32];
        let view =
            D3D11_VIEWPORT { Width: viewport[0], Height: viewport[1], MaxDepth: 1.0, ..Default::default() };
        let premultiplied = [clear.r * clear.a, clear.g * clear.a, clear.b * clear.a, clear.a];
        // SAFETY: the target and the viewport are alive for the calls.
        unsafe {
            self.gpu.context.OMSetRenderTargets(Some(&[Some(target.clone())]), None);
            self.gpu.context.RSSetViewports(Some(&[view]));
            self.gpu.context.ClearRenderTargetView(&target, &premultiplied);
        }
        self.gpu.pipeline.bind(&self.gpu.context);
        let atlases =
            [self.coverage.as_ref().map(|t| t.view.clone()), self.color.as_ref().map(|t| t.view.clone())];
        // SAFETY: the views are alive for the call; a slot without a texture reads as zero.
        unsafe { self.gpu.context.PSSetShaderResources(1, Some(&atlases)) };
        let (mut current, mut behind) = (None, [0.0; 4]);
        let mut result = Ok(());
        for step in &steps {
            match *step {
                Step::Layer { id, world } => {
                    behind = match world.backdrop {
                        None => [0.0; 4],
                        Some(backdrop) => {
                            match Backdrop::take(&mut self.backdrop, &self.gpu, &target, size, backdrop) {
                                Ok(behind) => behind,
                                Err(e) => {
                                    result = Err(e);
                                    break;
                                }
                            }
                        }
                    };
                    let instances = match self.layers.sync(&self.gpu, scene, id, &mut self.stats) {
                        Ok(view) => Some(view),
                        Err(e) => {
                            result = Err(e);
                            break;
                        }
                    };
                    // SAFETY: the view and the rectangle are alive for the calls.
                    unsafe {
                        self.gpu.context.VSSetShaderResources(0, Some(std::slice::from_ref(&instances)));
                        self.gpu.context.PSSetShaderResources(0, Some(&[instances]));
                        self.gpu.context.RSSetScissorRects(Some(&[scissor(world.scissor, size)]));
                    }
                    current = Some(world);
                    self.stats.layers_bound += 1;
                }
                Step::Run { start, end } => {
                    let Some(world) = &current else { continue };
                    if let Err(e) =
                        self.gpu.pipeline.set_run(&self.gpu.context, world, viewport, (start, behind))
                    {
                        result = Err(e);
                        break;
                    }
                    // SAFETY: the pipeline, its constants and the layer's instances are bound.
                    unsafe { self.gpu.context.DrawInstanced(4, end - start, 0, 0) };
                    self.stats.draw_calls += 1;
                    self.stats.instances_drawn += end - start;
                }
            }
        }
        self.steps = steps;
        self.timing.end(&self.gpu.context);
        self.stats.gpu = self.timing.last.map(|(_, time)| time);
        result
    }

    fn present(&mut self, surface: SurfaceId) -> Result<(), RenderError> {
        self.gpu.ready()?;
        self.surface(surface)?.present(&self.gpu)
    }

    fn read_pixels(&mut self, surface: SurfaceId) -> Result<Pixels, RenderError> {
        self.gpu.ready()?;
        self.surface(surface)?.read(&self.gpu)
    }

    fn forget_scene(&mut self, scene: SceneId) {
        self.layers.forget(scene);
    }

    fn recover(&mut self) -> Result<(), RenderError> {
        let software = self.gpu.is_software();
        let made_for: Vec<_> = self.surfaces.iter().map(|s| s.as_ref().map(Surface::made_for)).collect();
        // A window takes a new swap chain only once the old one is gone, and a swap chain
        // goes when nothing is bound and the context has been flushed.
        self.surfaces.clear();
        if let Some(old) = self.gpu.get() {
            // SAFETY: plain calls on the old context, which is alive until it is replaced below.
            unsafe {
                old.context.ClearState();
                old.context.Flush();
            }
        }
        self.gpu = Gpu::made(match self.upgrade.take().and_then(Upgrade::take) {
            Some(waiting) => waiting,
            None => Device::new(software)?,
        });
        self.layers.clear();
        (self.coverage, self.color, self.backdrop) = (None, None, None);
        self.timing.reset();
        for made_for in made_for {
            let surface = made_for.map(|(target, size)| self.make_surface(target, size)).transpose()?;
            self.surfaces.push(surface);
        }
        Ok(())
    }

    fn wants_frame(&mut self) -> bool {
        self.upgrade.as_mut().is_some_and(Upgrade::is_ready)
    }

    fn stats(&self) -> RenderStats {
        self.stats
    }

    fn measure_gpu(&mut self, on: bool) {
        self.timing.on = on;
    }
}

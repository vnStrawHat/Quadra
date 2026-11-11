//! The D3D11 renderer (specs/07): it keeps a buffer per layer, uploads what a layer says
//! changed, and draws the scene's steps.

use std::collections::HashMap;
use std::ffi::c_void;

use quadra_prim::{Color, Rect};
use quadra_render::{
    AtlasCore, DeviceSize, Dirty, Instance, LayerId, Pixels, RenderError, RenderStats, Renderer, Scene,
    SceneId, Step, SurfaceId, SurfaceTarget,
};
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BOX, D3D11_VIEWPORT, ID3D11Buffer, ID3D11ShaderResourceView,
};

use crate::gpu::Gpu;
use crate::pipeline::Pipeline;
use crate::surface::Offscreen;

const STRIDE: u32 = size_of::<Instance>() as u32;

/// A layer's instances on the GPU.
struct LayerBuffer {
    /// The layer it was last filled for: another layer may have taken the same id.
    serial: u64,
    capacity: u32,
    view: ID3D11ShaderResourceView,
    buffer: ID3D11Buffer,
}

pub struct D3d11Renderer {
    gpu: Gpu,
    pipeline: Pipeline,
    surfaces: Vec<Option<Offscreen>>,
    layers: HashMap<(SceneId, LayerId), LayerBuffer>,
    steps: Vec<Step>,
    stats: RenderStats,
}

impl D3d11Renderer {
    /// A renderer on the hardware device, or on the software rasterizer: no GPU, the same
    /// pixels on every machine (tests, and the first frames of a window).
    pub fn new(software: bool) -> Result<D3d11Renderer, RenderError> {
        let gpu = Gpu::new(software)?;
        let pipeline = Pipeline::new(&gpu)?;
        Ok(D3d11Renderer {
            gpu,
            pipeline,
            surfaces: Vec::new(),
            layers: HashMap::new(),
            steps: Vec::new(),
            stats: RenderStats::default(),
        })
    }

    pub fn is_software(&self) -> bool {
        self.gpu.software
    }

    fn surface(&self, id: SurfaceId) -> Result<&Offscreen, RenderError> {
        self.surfaces.get(id.0 as usize).and_then(Option::as_ref).ok_or(RenderError::BadSurface)
    }

    /// Brings a layer's buffer in step with the scene and returns its view.
    fn sync(&mut self, scene: &mut Scene, id: LayerId) -> Result<ID3D11ShaderResourceView, RenderError> {
        let key = (scene.id(), id);
        let layer = scene.layer_mut(id);
        let dirty = layer.take_dirty();
        let (instances, serial) = (layer.instances(), layer.serial());
        let count = instances.len() as u32;
        let stale = self.layers.get(&key).is_none_or(|b| b.serial != serial || b.capacity < count);
        if stale {
            let capacity = count.next_power_of_two().max(64);
            let (buffer, view) = self.gpu.structured_buffer(capacity, STRIDE)?;
            self.layers.insert(key, LayerBuffer { serial, capacity, view, buffer });
        }
        let buffer = &self.layers[&key];
        let range = match dirty {
            _ if stale => 0..count,
            Dirty::Clean => 0..0,
            Dirty::Range { start, end } => start..end.min(count),
            Dirty::All => 0..count,
        };
        if !range.is_empty() {
            let part = D3D11_BOX {
                left: range.start * STRIDE,
                right: range.end * STRIDE,
                top: 0,
                bottom: 1,
                front: 0,
                back: 1,
            };
            let from = instances[range.start as usize..].as_ptr().cast::<c_void>();
            // SAFETY: `from` points at range.len() instances, which is what the box takes.
            unsafe { self.gpu.update(&buffer.buffer, 0, Some(part), from, 0)? };
            self.stats.instances_uploaded += range.len() as u32;
            self.stats.layers_uploaded += 1;
        }
        Ok(buffer.view.clone())
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
        if target != SurfaceTarget::Offscreen {
            return Err(RenderError::Backend("window surfaces come with the window".into()));
        }
        self.surfaces.push(Some(Offscreen::new(&self.gpu, size)?));
        Ok(SurfaceId(self.surfaces.len() as u32 - 1))
    }

    fn resize_surface(&mut self, surface: SurfaceId, size: DeviceSize) -> Result<(), RenderError> {
        if self.surface(surface)?.size != size {
            self.surfaces[surface.0 as usize] = Some(Offscreen::new(&self.gpu, size)?);
        }
        Ok(())
    }

    fn drop_surface(&mut self, surface: SurfaceId) {
        if let Some(slot) = self.surfaces.get_mut(surface.0 as usize) {
            *slot = None;
        }
    }

    fn sync_atlas(&mut self, atlas: &mut AtlasCore) -> Result<(), RenderError> {
        // ponytail: glyph textures come with the text backend; until then the pixels are taken
        // and dropped, as the null renderer does.
        atlas.take_uploads();
        Ok(())
    }

    fn draw(&mut self, surface: SurfaceId, scene: &mut Scene, clear: Color) -> Result<(), RenderError> {
        let (size, target) = {
            let s = self.surface(surface)?;
            (s.size, s.target.clone())
        };
        for layer in scene.take_dropped() {
            self.layers.remove(&(scene.id(), layer));
        }
        let mut steps = std::mem::take(&mut self.steps);
        let bounds = Rect::new(0.0, 0.0, size.width as f32, size.height as f32);
        scene.steps(bounds, &mut steps);
        self.stats = RenderStats { frames: self.stats.frames + 1, ..RenderStats::default() };

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
        self.pipeline.bind(&self.gpu);
        let mut current = None;
        let mut result = Ok(());
        for step in &steps {
            match *step {
                Step::Layer { id, world } => {
                    let instances = match self.sync(scene, id) {
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
                    if let Err(e) = self.pipeline.set_run(&self.gpu, world, viewport, start) {
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
        result
    }

    fn present(&mut self, surface: SurfaceId) -> Result<(), RenderError> {
        self.surface(surface).map(|_| ())
    }

    fn read_pixels(&mut self, surface: SurfaceId) -> Result<Pixels, RenderError> {
        self.surface(surface)?.read(&self.gpu)
    }

    fn forget_scene(&mut self, scene: SceneId) {
        self.layers.retain(|key, _| key.0 != scene);
    }

    fn recover(&mut self) -> Result<(), RenderError> {
        let software = self.gpu.software;
        let sizes: Vec<Option<DeviceSize>> =
            self.surfaces.iter().map(|s| s.as_ref().map(|s| s.size)).collect();
        self.gpu = Gpu::new(software)?;
        self.pipeline = Pipeline::new(&self.gpu)?;
        self.layers.clear();
        self.surfaces.clear();
        for size in sizes {
            let surface = size.map(|size| Offscreen::new(&self.gpu, size)).transpose()?;
            self.surfaces.push(surface);
        }
        Ok(())
    }

    fn stats(&self) -> RenderStats {
        self.stats
    }
}

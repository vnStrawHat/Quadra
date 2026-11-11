//! A renderer with no GPU: it does the bookkeeping of a real backend and keeps a copy of what
//! it was told to upload, so tests can check that copy against the scene, read the steps of a
//! frame, and count draws (specs/07; specs/12, "Snapshots").

use std::collections::HashMap;

use quadra_prim::{Color, Rect};

use crate::instance::Instance;
use crate::layer::{Dirty, LayerId};
use crate::renderer::{DeviceSize, Pixels, RenderError, RenderStats, Renderer, SurfaceId, SurfaceTarget};
use crate::scene::{Scene, SceneId, Step};

#[derive(Default)]
pub struct NullRenderer {
    surfaces: Vec<Option<DeviceSize>>,
    /// Per layer: the serial it was uploaded for, and the "GPU" copy of its instances.
    buffers: HashMap<(SceneId, LayerId), (u64, Vec<Instance>)>,
    steps: Vec<Step>,
    stats: RenderStats,
    lost: bool,
}

impl NullRenderer {
    pub fn new() -> NullRenderer {
        NullRenderer::default()
    }

    /// The device is gone: draws fail with `DeviceLost` until `recover`.
    pub fn lose_device(&mut self) {
        self.lost = true;
        self.buffers.clear();
    }

    /// What the "GPU" holds of a layer.
    pub fn uploaded(&self, scene: SceneId, layer: LayerId) -> Option<&[Instance]> {
        self.buffers.get(&(scene, layer)).map(|(_, instances)| instances.as_slice())
    }

    /// The steps of the last frame drawn.
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// How many layers the renderer holds buffers for.
    pub fn buffers(&self) -> usize {
        self.buffers.len()
    }

    fn size(&self, surface: SurfaceId) -> Result<DeviceSize, RenderError> {
        self.surfaces.get(surface.0 as usize).copied().flatten().ok_or(RenderError::BadSurface)
    }

    fn sync(&mut self, scene: &mut Scene, id: LayerId) {
        let key = (scene.id(), id);
        let layer = scene.layer_mut(id);
        let dirty = layer.take_dirty();
        let (serial, copy) = self.buffers.entry(key).or_default();
        let range = match dirty {
            _ if *serial != layer.serial() => None,
            Dirty::Clean => return,
            Dirty::Range { start, end } => Some(start as usize..end as usize),
            Dirty::All => None,
        };
        *serial = layer.serial();
        self.stats.layers_uploaded += 1;
        match range {
            Some(range) => {
                self.stats.instances_uploaded += range.len() as u32;
                copy[range.clone()].copy_from_slice(&layer.instances()[range]);
            }
            None => {
                self.stats.instances_uploaded += layer.instances().len() as u32;
                copy.clear();
                copy.extend_from_slice(layer.instances());
            }
        }
    }
}

impl Renderer for NullRenderer {
    fn create_surface(&mut self, _: SurfaceTarget, size: DeviceSize) -> Result<SurfaceId, RenderError> {
        self.surfaces.push(Some(size));
        Ok(SurfaceId(self.surfaces.len() as u32 - 1))
    }

    fn resize_surface(&mut self, surface: SurfaceId, size: DeviceSize) -> Result<(), RenderError> {
        self.size(surface)?;
        self.surfaces[surface.0 as usize] = Some(size);
        Ok(())
    }

    fn drop_surface(&mut self, surface: SurfaceId) {
        if let Some(slot) = self.surfaces.get_mut(surface.0 as usize) {
            *slot = None;
        }
    }

    fn draw(&mut self, surface: SurfaceId, scene: &mut Scene, _clear: Color) -> Result<(), RenderError> {
        let size = self.size(surface)?;
        if self.lost {
            return Err(RenderError::DeviceLost);
        }
        for layer in scene.take_dropped() {
            self.buffers.remove(&(scene.id(), layer));
        }
        let mut steps = std::mem::take(&mut self.steps);
        scene.steps(Rect::new(0.0, 0.0, size.width as f32, size.height as f32), &mut steps);
        self.stats = RenderStats { frames: self.stats.frames + 1, ..RenderStats::default() };
        for step in &steps {
            match *step {
                Step::Layer { id, .. } => {
                    self.sync(scene, id);
                    self.stats.layers_bound += 1;
                }
                Step::Run { start, end } => {
                    self.stats.draw_calls += 1;
                    self.stats.instances_drawn += end - start;
                }
            }
        }
        self.steps = steps;
        Ok(())
    }

    fn present(&mut self, surface: SurfaceId) -> Result<(), RenderError> {
        self.size(surface).map(|_| ())
    }

    fn read_pixels(&mut self, surface: SurfaceId) -> Result<Pixels, RenderError> {
        // Nothing is rasterized: the right size, all transparent.
        let size = self.size(surface)?;
        Ok(Pixels { size, rgba: vec![0; size.width as usize * size.height as usize * 4] })
    }

    fn forget_scene(&mut self, scene: SceneId) {
        self.buffers.retain(|key, _| key.0 != scene);
    }

    fn recover(&mut self) -> Result<(), RenderError> {
        self.lost = false;
        Ok(())
    }

    fn stats(&self) -> RenderStats {
        self.stats
    }
}

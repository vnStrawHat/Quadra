//! The layers' instances on the GPU: a buffer per layer, in step with what the layer says
//! changed (specs/07, "Display lists").

use std::collections::HashMap;

use objc2_metal::{MTLBuffer, MTLDevice, MTLResourceOptions};
use quadra_render::{Dirty, Instance, LayerId, RenderError, RenderStats, Scene, SceneId};

use super::gpu::{Buffer, Gpu, made};

const STRIDE: usize = size_of::<Instance>();

struct LayerBuffer {
    /// The layer it was last filled for: another layer may have taken the same id.
    serial: u64,
    capacity: u32,
    buffer: Buffer,
}

#[derive(Default)]
pub(crate) struct LayerBuffers(HashMap<(SceneId, LayerId), LayerBuffer>);

impl LayerBuffers {
    /// Brings a layer's buffer in step with the scene and returns it.
    pub fn sync(
        &mut self,
        gpu: &Gpu,
        scene: &mut Scene,
        id: LayerId,
        stats: &mut RenderStats,
    ) -> Result<Buffer, RenderError> {
        let key = (scene.id(), id);
        let layer = scene.layer_mut(id);
        let dirty = layer.take_dirty();
        let (instances, serial) = (layer.instances(), layer.serial());
        let count = instances.len() as u32;
        let stale = self.0.get(&key).is_none_or(|b| b.serial != serial || b.capacity < count);
        if stale {
            let capacity = count.next_power_of_two().max(64);
            let shared = MTLResourceOptions::StorageModeShared;
            let buffer = gpu.device.newBufferWithLength_options(capacity as usize * STRIDE, shared);
            self.0.insert(key, LayerBuffer { serial, capacity, buffer: made(buffer, "instance buffer")? });
        }
        let buffer = &self.0[&key];
        let range = match dirty {
            _ if stale => 0..count,
            Dirty::Clean => 0..0,
            Dirty::Range { start, end } => start..end.min(count),
            Dirty::All => 0..count,
        };
        if !range.is_empty() {
            let (start, len) = (range.start as usize, range.len());
            // SAFETY: the buffer holds `capacity` instances, of which the range is a part, and
            // `instances` has every one of the range; the two do not overlap.
            unsafe {
                let to = buffer.buffer.contents().as_ptr().cast::<u8>().add(start * STRIDE);
                std::ptr::copy_nonoverlapping(instances[start..].as_ptr().cast::<u8>(), to, len * STRIDE);
            }
            stats.instances_uploaded += len as u32;
            stats.layers_uploaded += 1;
        }
        Ok(buffer.buffer.clone())
    }

    /// A layer of a scene is gone.
    pub fn remove(&mut self, scene: SceneId, layer: LayerId) {
        self.0.remove(&(scene, layer));
    }

    /// A scene is gone: its window closed.
    pub fn forget(&mut self, scene: SceneId) {
        self.0.retain(|key, _| key.0 != scene);
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

//! The layers' instances on the GPU: a buffer per layer, in step with what the layer says
//! changed (specs/07, "Display lists").

use std::collections::HashMap;
use std::ffi::c_void;

use quadra_render::{Dirty, Instance, LayerId, RenderError, RenderStats, Scene, SceneId};
use windows::Win32::Graphics::Direct3D11::{D3D11_BOX, ID3D11Buffer, ID3D11ShaderResourceView};

use crate::gpu::Device;

const STRIDE: u32 = size_of::<Instance>() as u32;

struct LayerBuffer {
    /// The layer it was last filled for: another layer may have taken the same id.
    serial: u64,
    capacity: u32,
    view: ID3D11ShaderResourceView,
    buffer: ID3D11Buffer,
}

#[derive(Default)]
pub(crate) struct LayerBuffers(HashMap<(SceneId, LayerId), LayerBuffer>);

impl LayerBuffers {
    /// Brings a layer's buffer in step with the scene and returns its view.
    pub fn sync(
        &mut self,
        device: &Device,
        scene: &mut Scene,
        id: LayerId,
        stats: &mut RenderStats,
    ) -> Result<ID3D11ShaderResourceView, RenderError> {
        let key = (scene.id(), id);
        let layer = scene.layer_mut(id);
        let dirty = layer.take_dirty();
        let (instances, serial) = (layer.instances(), layer.serial());
        let count = instances.len() as u32;
        let stale = self.0.get(&key).is_none_or(|b| b.serial != serial || b.capacity < count);
        if stale {
            let capacity = count.next_power_of_two().max(64);
            let (buffer, view) = device.structured_buffer(capacity, STRIDE)?;
            self.0.insert(key, LayerBuffer { serial, capacity, view, buffer });
        }
        let buffer = &self.0[&key];
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
            unsafe { device.update(&buffer.buffer, 0, Some(part), from, 0)? };
            stats.instances_uploaded += range.len() as u32;
            stats.layers_uploaded += 1;
        }
        Ok(buffer.view.clone())
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

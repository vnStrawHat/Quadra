//! What a scene is drawn into (specs/07, "Surfaces"): so far a texture that can be read
//! back, for tests and snapshots. A window's surface comes with the AppKit platform.

use std::ptr::NonNull;

use objc2_metal::{MTLOrigin, MTLRegion, MTLSize, MTLTexture, MTLTextureUsage};
use quadra_render::{DeviceSize, Pixels, RenderError, SurfaceTarget};

use super::gpu::{Gpu, TARGET, Texture, texture};

pub(crate) struct Surface {
    pub size: DeviceSize,
    pub texture: Texture,
}

impl Surface {
    pub fn new(gpu: &Gpu, target: SurfaceTarget, size: DeviceSize) -> Result<Surface, RenderError> {
        if target != SurfaceTarget::Offscreen {
            return Err(RenderError::BadSurface);
        }
        // Drawn into, read by the copy a blurring box takes of it, and read back.
        let usage = MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead;
        let size = DeviceSize { width: size.width.max(1), height: size.height.max(1) };
        Ok(Surface {
            size,
            texture: texture(&gpu.device, TARGET, (size.width, size.height), None, false, usage)?,
        })
    }

    /// The pixels drawn, rows from the top.
    pub fn read(&self) -> Pixels {
        let (width, height) = (self.size.width as usize, self.size.height as usize);
        let mut rgba = vec![0u8; width * height * 4];
        let all =
            MTLRegion { origin: MTLOrigin { x: 0, y: 0, z: 0 }, size: MTLSize { width, height, depth: 1 } };
        // SAFETY: the buffer is `height` rows of `width` pixels of four bytes, which is what
        // the region of the texture's format gives.
        unsafe {
            self.texture.getBytes_bytesPerRow_fromRegion_mipmapLevel(
                NonNull::from(rgba.as_mut_slice()).cast(),
                width * 4,
                all,
                0,
            );
        }
        Pixels { size: self.size, rgba }
    }
}

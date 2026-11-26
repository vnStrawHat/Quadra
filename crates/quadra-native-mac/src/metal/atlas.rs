//! Atlas textures: one texture array per atlas, kept in step with the atlas's upload queue
//! (specs/07, "Atlases").

use std::ptr::NonNull;

use objc2_metal::{
    MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLOrigin, MTLPixelFormat, MTLRegion,
    MTLSize, MTLTexture, MTLTextureUsage,
};
use quadra_render::{AtlasCore, AtlasKind, RenderError};

use super::gpu::{Gpu, Texture, finish, made, texture};

pub(crate) struct AtlasTexture {
    /// The atlas epoch the texture holds: after a clear it starts again.
    epoch: u64,
    size: u32,
    slices: u32,
    pub texture: Texture,
}

fn format(kind: AtlasKind) -> MTLPixelFormat {
    match kind {
        AtlasKind::Coverage => MTLPixelFormat::R8Unorm,
        AtlasKind::Color => MTLPixelFormat::RGBA8Unorm,
    }
}

const NOWHERE: MTLOrigin = MTLOrigin { x: 0, y: 0, z: 0 };

impl AtlasTexture {
    /// Brings the texture of an atlas in step: a new one after a clear, a larger one (with the
    /// old slices copied) when the atlas grew, then the queued pixels.
    pub fn sync(
        slot: &mut Option<AtlasTexture>,
        gpu: &Gpu,
        atlas: &mut AtlasCore,
    ) -> Result<(), RenderError> {
        let (kind, epoch, size, slices) =
            (atlas.kind(), atlas.epoch(), atlas.slice_size(), atlas.slices().max(1));
        let current = slot.as_ref().filter(|t| t.epoch == epoch && t.size == size);
        if current.is_none_or(|t| t.slices < slices) {
            let read = MTLTextureUsage::ShaderRead;
            let larger = texture(&gpu.device, format(kind), (size, size), Some(slices), false, read)?;
            if let Some(old) = current {
                let commands = gpu.commands()?;
                let blit = made(commands.blitCommandEncoder(), "blit encoder")?;
                let whole = MTLSize { width: size as usize, height: size as usize, depth: 1 };
                for slice in 0..old.slices as usize {
                    // SAFETY: both textures are alive, of one format and slice size, and
                    // `slice` exists in both.
                    unsafe {
                        blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                            &old.texture, slice, 0, NOWHERE, whole, &larger, slice, 0, NOWHERE,
                        );
                    }
                }
                blit.endEncoding();
                finish(&commands)?;
            }
            *slot = Some(AtlasTexture { epoch, size, slices, texture: larger });
        }
        let Some(texture) = slot.as_ref() else { return Ok(()) };
        for upload in atlas.take_uploads() {
            let r = upload.rect;
            let pitch = r.w as usize * kind.bytes_per_pixel();
            debug_assert_eq!(upload.pixels.len(), pitch * r.h as usize);
            if upload.pixels.is_empty() {
                continue;
            }
            let part = MTLRegion {
                origin: MTLOrigin { x: r.x as usize, y: r.y as usize, z: 0 },
                size: MTLSize { width: r.w as usize, height: r.h as usize, depth: 1 },
            };
            // SAFETY: the pixels are `r.h` rows of `pitch` bytes, which is what the region
            // takes; a slice of an array is given no size of an image.
            unsafe {
                texture.texture.replaceRegion_mipmapLevel_slice_withBytes_bytesPerRow_bytesPerImage(
                    part,
                    0,
                    r.slice as usize,
                    NonNull::from(upload.pixels.as_slice()).cast(),
                    pitch,
                    0,
                );
            }
        }
        Ok(())
    }
}

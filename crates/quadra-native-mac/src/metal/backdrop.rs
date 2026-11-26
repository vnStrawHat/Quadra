//! What is behind a box that blurs it (specs/07, "Instances"): the part of the target drawn so
//! far under the box, copied into a texture whose mips are the blur.

use objc2_metal::{
    MTLBlitCommandEncoder, MTLCommandBuffer, MTLCommandEncoder, MTLOrigin, MTLSize, MTLTextureUsage,
};
use quadra_prim::Rect;
use quadra_render::{DeviceSize, RenderError};

use super::gpu::{Commands, Gpu, TARGET, Texture, made, texture};

/// The texture's sides are whole blocks of the largest blur, so each mip's texels are whole
/// blocks of the surface.
const BLOCK: u32 = 64;

pub(crate) struct Backdrop {
    pub texture: Texture,
    size: (u32, u32),
}

/// The block a blur averages: the mip level it is sampled at is its power of two.
fn block(blur: f32) -> u32 {
    (2.0 * blur).max(1.0).log2().ceil().clamp(0.0, BLOCK.ilog2() as f32).exp2() as u32
}

impl Backdrop {
    /// Copies what `target` holds under `rect` (surface pixels), padded by twice the blur and
    /// aligned to its blocks, into the texture in `slot`, and makes its mips: commands of
    /// `commands`, which is drawing into nothing meanwhile. Gives where the copy starts on the
    /// surface and the texture's size, as the shader's constants want them.
    pub fn take(
        slot: &mut Option<Backdrop>,
        gpu: &Gpu,
        commands: &Commands,
        target: &Texture,
        surface: DeviceSize,
        (rect, blur): (Rect, f32),
    ) -> Result<[f32; 4], RenderError> {
        let (b, pad) = (block(blur) as f32, 2.0 * blur);
        let snap = |v: f32, max: u32, up: bool| {
            let v = if up { (v / b).ceil() } else { (v / b).floor() } * b;
            v.clamp(0.0, max as f32) as u32
        };
        let (left, top) =
            (snap(rect.x - pad, surface.width, false), snap(rect.y - pad, surface.height, false));
        let (right, bottom) =
            (snap(rect.right() + pad, surface.width, true), snap(rect.bottom() + pad, surface.height, true));
        if right <= left || bottom <= top {
            return Ok([0.0; 4]);
        }
        let (w, h) = ((right - left).next_multiple_of(BLOCK), (bottom - top).next_multiple_of(BLOCK));
        if !slot.as_ref().is_some_and(|b| b.size.0 >= w && b.size.1 >= h) {
            let grown = slot.as_ref().map_or((0, 0), |b| b.size);
            let size = (w.max(grown.0), h.max(grown.1));
            // Its mips are drawn into when they are made.
            let usage = MTLTextureUsage::ShaderRead | MTLTextureUsage::RenderTarget;
            *slot = Some(Backdrop { texture: texture(&gpu.device, TARGET, size, None, true, usage)?, size });
        }
        let backdrop = slot.as_ref().expect("made above");
        let blit = made(commands.blitCommandEncoder(), "blit encoder")?;
        let from = MTLOrigin { x: left as usize, y: top as usize, z: 0 };
        let part = MTLSize { width: (right - left) as usize, height: (bottom - top) as usize, depth: 1 };
        // SAFETY: the textures are alive and of one format, and the part lies inside both.
        unsafe {
            blit.copyFromTexture_sourceSlice_sourceLevel_sourceOrigin_sourceSize_toTexture_destinationSlice_destinationLevel_destinationOrigin(
                target, 0, 0, from, part, &backdrop.texture, 0, 0, MTLOrigin { x: 0, y: 0, z: 0 },
            );
        }
        blit.generateMipmapsForTexture(&backdrop.texture);
        blit.endEncoding();
        Ok([left as f32, top as f32, backdrop.size.0 as f32, backdrop.size.1 as f32])
    }
}

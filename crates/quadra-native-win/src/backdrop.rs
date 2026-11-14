//! What is behind a box that blurs it (specs/07, "Instances"): the part of the target drawn so
//! far under the box, copied into a texture whose mips are the blur.

use quadra_prim::Rect;
use quadra_render::{DeviceSize, RenderError};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BOX, D3D11_RESOURCE_MISC_GENERATE_MIPS,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, ID3D11RenderTargetView, ID3D11ShaderResourceView,
    ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC;
use windows::core::Interface;

use crate::gpu::{Device, error, made};

/// The texture's sides are whole blocks of the largest blur, so each mip's texels are whole
/// blocks of the surface.
const BLOCK: u32 = 64;

pub(crate) struct Backdrop {
    texture: ID3D11Texture2D,
    pub view: ID3D11ShaderResourceView,
    desc: D3D11_TEXTURE2D_DESC,
}

/// The block a blur averages: the mip level it is sampled at is its power of two.
fn block(blur: f32) -> u32 {
    (2.0 * blur).max(1.0).log2().ceil().clamp(0.0, BLOCK.ilog2() as f32).exp2() as u32
}

impl Backdrop {
    /// Copies what `target` holds under `rect` (surface pixels), padded by twice the blur and
    /// aligned to its blocks, into the texture in `slot`, and makes its mips. Gives where the
    /// copy starts on the surface and the texture's size, as the shader's constants want them.
    pub fn take(
        slot: &mut Option<Backdrop>,
        gpu: &Device,
        target: &ID3D11RenderTargetView,
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
        // SAFETY: a view always has a resource.
        let source = unsafe { target.GetResource() }.map_err(error)?;
        let source: ID3D11Texture2D = source.cast().map_err(error)?;
        let mut format = D3D11_TEXTURE2D_DESC::default();
        // SAFETY: the out struct is valid for the call.
        unsafe { source.GetDesc(&mut format) };
        let (w, h) = ((right - left).next_multiple_of(BLOCK), (bottom - top).next_multiple_of(BLOCK));
        let fits = |b: &Backdrop| b.desc.Format == format.Format && b.desc.Width >= w && b.desc.Height >= h;
        if !slot.as_ref().is_some_and(fits) {
            let grown = slot.as_ref().map_or((0, 0), |b| (b.desc.Width, b.desc.Height));
            *slot = Some(Backdrop::new(gpu, (w.max(grown.0), h.max(grown.1)), format)?);
        }
        let backdrop = slot.as_ref().expect("made above");
        let from = D3D11_BOX { left, top, front: 0, right, bottom, back: 1 };
        // The target is unbound while it is read, and bound again for the layer's runs.
        // SAFETY: the resources and the box are alive for the calls, and the box lies inside
        // both textures.
        unsafe {
            gpu.context.OMSetRenderTargets(None, None);
            gpu.context.CopySubresourceRegion(&backdrop.texture, 0, 0, 0, 0, &source, 0, Some(&from));
            gpu.context.GenerateMips(&backdrop.view);
            gpu.context.OMSetRenderTargets(Some(&[Some(target.clone())]), None);
            gpu.context.PSSetShaderResources(3, Some(&[Some(backdrop.view.clone())]));
        }
        Ok([left as f32, top as f32, backdrop.desc.Width as f32, backdrop.desc.Height as f32])
    }

    fn new(
        gpu: &Device,
        (width, height): (u32, u32),
        like: D3D11_TEXTURE2D_DESC,
    ) -> Result<Backdrop, RenderError> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 0, // all of them
            ArraySize: 1,
            Format: like.Format,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: (D3D11_BIND_SHADER_RESOURCE.0 | D3D11_BIND_RENDER_TARGET.0) as u32,
            CPUAccessFlags: 0,
            MiscFlags: D3D11_RESOURCE_MISC_GENERATE_MIPS.0 as u32,
        };
        let (mut texture, mut view) = (None, None);
        // SAFETY: the description and the out pointers are valid for the calls.
        unsafe {
            gpu.device.CreateTexture2D(&desc, None, Some(&mut texture)).map_err(error)?;
            let texture = made(texture, "backdrop texture")?;
            gpu.device.CreateShaderResourceView(&texture, None, Some(&mut view)).map_err(error)?;
            Ok(Backdrop { texture, view: made(view, "backdrop view")?, desc })
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_blur_averages_a_block_twice_as_wide_in_whole_powers_of_two() {
        let blocks: Vec<u32> = [0.5, 1.0, 3.0, 8.0, 12.0, 100.0].map(super::block).to_vec();
        assert_eq!(blocks, [1, 2, 8, 16, 32, 64]);
    }
}

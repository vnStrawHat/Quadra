//! Atlas textures: one texture array per atlas, kept in step with the atlas's upload queue
//! (specs/07, "Atlases").

use std::ffi::c_void;

use quadra_render::{AtlasCore, AtlasKind, RenderError};
use windows::Win32::Graphics::Direct3D::D3D_SRV_DIMENSION_TEXTURE2DARRAY;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_SHADER_RESOURCE, D3D11_BOX, D3D11_SHADER_RESOURCE_VIEW_DESC,
    D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_TEX2D_ARRAY_SRV, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
    ID3D11ShaderResourceView, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_R8_UNORM, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC,
};

use crate::gpu::{Device, error, made};

pub(crate) struct AtlasTexture {
    /// The atlas epoch the texture holds: after a clear it starts again.
    epoch: u64,
    size: u32,
    slices: u32,
    texture: ID3D11Texture2D,
    pub view: ID3D11ShaderResourceView,
}

fn format(kind: AtlasKind) -> DXGI_FORMAT {
    match kind {
        AtlasKind::Coverage => DXGI_FORMAT_R8_UNORM,
        AtlasKind::Color => DXGI_FORMAT_R8G8B8A8_UNORM,
    }
}

impl AtlasTexture {
    fn new(
        gpu: &Device,
        kind: AtlasKind,
        epoch: u64,
        size: u32,
        slices: u32,
    ) -> Result<AtlasTexture, RenderError> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: size,
            Height: size,
            MipLevels: 1,
            ArraySize: slices,
            Format: format(kind),
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        // Always viewed as an array, as the shader declares it, even with one slice.
        let view_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
            Format: desc.Format,
            ViewDimension: D3D_SRV_DIMENSION_TEXTURE2DARRAY,
            Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2DArray: D3D11_TEX2D_ARRAY_SRV {
                    MostDetailedMip: 0,
                    MipLevels: 1,
                    FirstArraySlice: 0,
                    ArraySize: slices,
                },
            },
        };
        let (mut texture, mut view) = (None, None);
        // SAFETY: the descriptions and the out pointers are valid for the calls.
        unsafe {
            gpu.device.CreateTexture2D(&desc, None, Some(&mut texture)).map_err(error)?;
            let texture = made(texture, "atlas texture")?;
            gpu.device
                .CreateShaderResourceView(&texture, Some(&view_desc), Some(&mut view))
                .map_err(error)?;
            Ok(AtlasTexture { epoch, size, slices, texture, view: made(view, "atlas view")? })
        }
    }

    /// Brings the texture of an atlas in step: a new one after a clear, a larger one (with the
    /// old slices copied) when the atlas grew, then the queued pixels.
    pub fn sync(
        slot: &mut Option<AtlasTexture>,
        gpu: &Device,
        atlas: &mut AtlasCore,
    ) -> Result<(), RenderError> {
        let (kind, epoch, size, slices) =
            (atlas.kind(), atlas.epoch(), atlas.slice_size(), atlas.slices().max(1));
        let current = slot.as_ref().filter(|t| t.epoch == epoch && t.size == size);
        if current.is_none_or(|t| t.slices < slices) {
            let larger = AtlasTexture::new(gpu, kind, epoch, size, slices)?;
            if let Some(old) = current {
                for slice in 0..old.slices {
                    // SAFETY: both textures are alive, of one format and slice size, and
                    // `slice` exists in both.
                    unsafe {
                        gpu.context.CopySubresourceRegion(
                            &larger.texture,
                            slice,
                            0,
                            0,
                            0,
                            &old.texture,
                            slice,
                            None,
                        );
                    }
                }
            }
            *slot = Some(larger);
        }
        let Some(texture) = slot.as_ref() else { return Ok(()) };
        for upload in atlas.take_uploads() {
            let r = upload.rect;
            let part =
                D3D11_BOX { left: r.x, top: r.y, front: 0, right: r.x + r.w, bottom: r.y + r.h, back: 1 };
            let pitch = r.w * kind.bytes_per_pixel() as u32;
            debug_assert_eq!(upload.pixels.len() as u32, pitch * r.h);
            // SAFETY: the pixels are `r.h` rows of `pitch` bytes, which is what the box takes.
            unsafe {
                gpu.update(
                    &texture.texture,
                    r.slice,
                    Some(part),
                    upload.pixels.as_ptr().cast::<c_void>(),
                    pitch,
                )?
            };
        }
        Ok(())
    }
}

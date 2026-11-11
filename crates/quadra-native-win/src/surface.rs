//! Surfaces: what a frame is drawn into. Offscreen textures first (tests, snapshots); window
//! swap chains come with the window.

use quadra_render::{DeviceSize, Pixels, RenderError};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_RENDER_TARGET, D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING, ID3D11RenderTargetView, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};

use crate::gpu::{Gpu, error, made};

fn texture_desc(size: DeviceSize) -> D3D11_TEXTURE2D_DESC {
    D3D11_TEXTURE2D_DESC {
        Width: size.width.max(1),
        Height: size.height.max(1),
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: D3D11_BIND_RENDER_TARGET.0 as u32,
        CPUAccessFlags: 0,
        MiscFlags: 0,
    }
}

/// A texture that is drawn into and can be read back.
pub(crate) struct Offscreen {
    pub size: DeviceSize,
    texture: ID3D11Texture2D,
    pub target: ID3D11RenderTargetView,
}

impl Offscreen {
    pub fn new(gpu: &Gpu, size: DeviceSize) -> Result<Offscreen, RenderError> {
        let (mut texture, mut target) = (None, None);
        // SAFETY: the description and the out pointers are valid for the calls.
        unsafe {
            gpu.device.CreateTexture2D(&texture_desc(size), None, Some(&mut texture)).map_err(error)?;
            let texture = made(texture, "texture")?;
            gpu.device.CreateRenderTargetView(&texture, None, Some(&mut target)).map_err(error)?;
            Ok(Offscreen { size, texture, target: made(target, "render target")? })
        }
    }

    /// The pixels as RGBA, rows from the top.
    pub fn read(&self, gpu: &Gpu) -> Result<Pixels, RenderError> {
        let desc = D3D11_TEXTURE2D_DESC {
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            ..texture_desc(self.size)
        };
        let (width, height) = (desc.Width as usize, desc.Height as usize);
        let mut staging = None;
        let mut rgba = vec![0u8; width * height * 4];
        // SAFETY: the staging texture has the format and size of the target, so CopyResource
        // is valid; the mapped rows are RowPitch apart and hold at least width * 4 bytes each,
        // for `height` rows, until Unmap.
        unsafe {
            gpu.device.CreateTexture2D(&desc, None, Some(&mut staging)).map_err(error)?;
            let staging = made(staging, "staging texture")?;
            gpu.context.CopyResource(&staging, &self.texture);
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            gpu.context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)).map_err(error)?;
            for y in 0..height {
                let row = mapped.pData.cast::<u8>().add(y * mapped.RowPitch as usize);
                let bgra = std::slice::from_raw_parts(row, width * 4);
                for (out, px) in rgba[y * width * 4..].chunks_exact_mut(4).zip(bgra.chunks_exact(4)) {
                    out.copy_from_slice(&[px[2], px[1], px[0], px[3]]);
                }
            }
            gpu.context.Unmap(&staging, 0);
        }
        Ok(Pixels { size: self.size, rgba })
    }
}

//! The one pipeline: its shaders, states and constants (specs/07, "Instances").

use quadra_render::{RenderError, World};
use windows::Win32::Graphics::Direct3D::D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_CONSTANT_BUFFER, D3D11_BLEND_DESC, D3D11_BLEND_INV_SRC1_ALPHA, D3D11_BLEND_INV_SRC1_COLOR,
    D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD, D3D11_BUFFER_DESC, D3D11_COLOR_WRITE_ENABLE_ALL,
    D3D11_CPU_ACCESS_WRITE, D3D11_CULL_NONE, D3D11_FILL_SOLID, D3D11_FILTER_MIN_MAG_MIP_LINEAR,
    D3D11_MAP_WRITE_DISCARD, D3D11_MAPPED_SUBRESOURCE, D3D11_RASTERIZER_DESC, D3D11_RENDER_TARGET_BLEND_DESC,
    D3D11_SAMPLER_DESC, D3D11_TEXTURE_ADDRESS_CLAMP, D3D11_USAGE_DYNAMIC, ID3D11BlendState, ID3D11Buffer,
    ID3D11Device, ID3D11DeviceContext, ID3D11PixelShader, ID3D11RasterizerState, ID3D11SamplerState,
    ID3D11VertexShader,
};

use crate::gpu::{error, made};

/// Compiled by `tools/build_shaders.sh` from `shaders/prims.hlsl` and embedded: start-up
/// compiles no shader.
const VS: &[u8] = include_bytes!("../shaders/prims_vs.cso");
const PS: &[u8] = include_bytes!("../shaders/prims_ps.cso");

/// `cbuffer Constants` of the shader: the layer of the run and where the run starts.
#[repr(C)]
struct Constants {
    layer_matrix: [f32; 4],
    layer_translate: [f32; 2],
    viewport: [f32; 2],
    layer_opacity: f32,
    instance_offset: u32,
    layer_turned: u32,
    pad: u32,
    /// Where the layer's backdrop was copied from on the surface, and its texture's size.
    backdrop: [f32; 4],
    /// The mask the layer is cut to: its box on the surface, its tile, its slice, and 1 when
    /// there is one.
    mask_rect: [f32; 4],
    mask_uv: [f32; 4],
    mask: [u32; 4],
}

pub(crate) struct Pipeline {
    vs: ID3D11VertexShader,
    ps: ID3D11PixelShader,
    blend: ID3D11BlendState,
    raster: ID3D11RasterizerState,
    sampler: ID3D11SamplerState,
    constants: ID3D11Buffer,
}

impl Pipeline {
    pub fn new(device: &ID3D11Device) -> Result<Pipeline, RenderError> {
        // Dual-source blending: the second output is a weight per channel, so subpixel text
        // blends each subpixel on its own. For everything else it is premultiplied alpha.
        let mut blend_desc = D3D11_BLEND_DESC::default();
        blend_desc.RenderTarget[0] = D3D11_RENDER_TARGET_BLEND_DESC {
            BlendEnable: true.into(),
            SrcBlend: D3D11_BLEND_ONE,
            DestBlend: D3D11_BLEND_INV_SRC1_COLOR,
            BlendOp: D3D11_BLEND_OP_ADD,
            SrcBlendAlpha: D3D11_BLEND_ONE,
            DestBlendAlpha: D3D11_BLEND_INV_SRC1_ALPHA,
            BlendOpAlpha: D3D11_BLEND_OP_ADD,
            RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
        };
        let raster_desc = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_SOLID,
            CullMode: D3D11_CULL_NONE,
            DepthClipEnable: true.into(),
            ScissorEnable: true.into(), // a layer is cut to its scissor
            ..Default::default()
        };
        let sampler_desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            MaxLOD: f32::MAX,
            ..Default::default()
        };
        let constants_desc = D3D11_BUFFER_DESC {
            ByteWidth: size_of::<Constants>() as u32,
            Usage: D3D11_USAGE_DYNAMIC,
            BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            ..Default::default()
        };
        let (mut vs, mut ps, mut blend, mut raster, mut sampler, mut constants) =
            (None, None, None, None, None, None);
        // SAFETY: bytecode, descriptions and out pointers are valid for the calls.
        unsafe {
            device.CreateVertexShader(VS, None, Some(&mut vs)).map_err(error)?;
            device.CreatePixelShader(PS, None, Some(&mut ps)).map_err(error)?;
            device.CreateBlendState(&blend_desc, Some(&mut blend)).map_err(error)?;
            device.CreateRasterizerState(&raster_desc, Some(&mut raster)).map_err(error)?;
            device.CreateSamplerState(&sampler_desc, Some(&mut sampler)).map_err(error)?;
            device.CreateBuffer(&constants_desc, None, Some(&mut constants)).map_err(error)?;
        }
        Ok(Pipeline {
            vs: made(vs, "vertex shader")?,
            ps: made(ps, "pixel shader")?,
            blend: made(blend, "blend state")?,
            raster: made(raster, "rasterizer state")?,
            sampler: made(sampler, "sampler")?,
            constants: made(constants, "constant buffer")?,
        })
    }

    /// Makes this pipeline the one draws use.
    pub fn bind(&self, c: &ID3D11DeviceContext) {
        // SAFETY: every object passed is alive for the call; the context keeps its own
        // references to what it binds.
        unsafe {
            c.IASetInputLayout(None);
            c.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);
            c.VSSetShader(&self.vs, None);
            c.PSSetShader(&self.ps, None);
            c.OMSetBlendState(&self.blend, None, u32::MAX);
            c.RSSetState(&self.raster);
            c.PSSetSamplers(0, Some(&[Some(self.sampler.clone())]));
            c.VSSetConstantBuffers(0, Some(&[Some(self.constants.clone())]));
            c.PSSetConstantBuffers(0, Some(&[Some(self.constants.clone())]));
        }
    }

    /// Sets the constants of a run: its layer's place on the surface, and its first instance.
    pub fn set_run(
        &self,
        context: &ID3D11DeviceContext,
        world: &World,
        viewport: [f32; 2],
        (start, backdrop): (u32, [f32; 4]),
    ) -> Result<(), RenderError> {
        let t = world.transform;
        let constants = Constants {
            layer_matrix: [t.a, t.b, t.c, t.d],
            layer_translate: [t.tx, t.ty],
            viewport,
            layer_opacity: world.opacity,
            instance_offset: start,
            layer_turned: !t.is_translation() as u32,
            pad: 0,
            backdrop,
            mask_rect: world.mask.map_or([0.0; 4], |(_, r)| [r.x, r.y, r.width, r.height]),
            mask_uv: world.mask.map_or([0.0; 4], |(m, _)| m.uv),
            mask: world.mask.map_or([0; 4], |(m, _)| [m.slice, 1, 0, 0]),
        };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY: a write-discard map of a dynamic buffer gives at least its ByteWidth, which
        // is the size of Constants; nothing else touches the buffer between Map and Unmap.
        unsafe {
            context.Map(&self.constants, 0, D3D11_MAP_WRITE_DISCARD, 0, Some(&mut mapped)).map_err(error)?;
            mapped.pData.cast::<Constants>().write(constants);
            context.Unmap(&self.constants, 0);
        }
        Ok(())
    }
}

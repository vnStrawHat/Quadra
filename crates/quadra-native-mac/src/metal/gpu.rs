//! The device and what every draw uses of it: its queue, the one pipeline, the sampler, and
//! textures made the one way.

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_foundation::{NSError, NSString};
use objc2_metal::{
    MTLBlendFactor, MTLBuffer, MTLCommandBuffer, MTLCommandQueue, MTLCreateSystemDefaultDevice, MTLDevice,
    MTLLibrary, MTLPixelFormat, MTLRenderCommandEncoder, MTLRenderPipelineDescriptor, MTLRenderPipelineState,
    MTLSamplerAddressMode, MTLSamplerDescriptor, MTLSamplerMinMagFilter, MTLSamplerMipFilter,
    MTLSamplerState, MTLStorageMode, MTLTexture, MTLTextureDescriptor, MTLTextureType, MTLTextureUsage,
};
use quadra_render::RenderError;

pub(crate) type Device = Retained<ProtocolObject<dyn MTLDevice>>;
pub(crate) type Texture = Retained<ProtocolObject<dyn MTLTexture>>;
pub(crate) type Buffer = Retained<ProtocolObject<dyn MTLBuffer>>;
pub(crate) type Commands = Retained<ProtocolObject<dyn MTLCommandBuffer>>;
pub(crate) type Encoder = Retained<ProtocolObject<dyn MTLRenderCommandEncoder>>;

/// The format of what is drawn into: RGBA, 8 bits a channel, as pixels are read back. Colours
/// are blended as they are encoded, as on Windows.
pub(crate) const TARGET: MTLPixelFormat = MTLPixelFormat::RGBA8Unorm;

/// Compiled when the device is made: `shaders/prims.metal`.
// ponytail: compiled from source at start-up; a library built by Xcode's tools, embedded as
// the Windows bytecode is, when a Mac is at hand to build it and start-up is measured (M5).
const SHADERS: &str = include_str!("../../shaders/prims.metal");

pub(crate) fn error(e: Retained<NSError>) -> RenderError {
    RenderError::Backend(e.localizedDescription().to_string())
}

/// What Metal gave, or an error that names what it did not.
pub(crate) fn made<T>(it: Option<T>, what: &str) -> Result<T, RenderError> {
    it.ok_or_else(|| RenderError::Backend(format!("Metal made no {what}")))
}

pub(crate) struct Gpu {
    pub device: Device,
    pub queue: Retained<ProtocolObject<dyn MTLCommandQueue>>,
    pub pipeline: Retained<ProtocolObject<dyn MTLRenderPipelineState>>,
    pub sampler: Retained<ProtocolObject<dyn MTLSamplerState>>,
    /// What the atlases' slots and the backdrop's are bound to before their textures exist:
    /// a draw may read no texture that is not bound. Coverage, colour, backdrop.
    pub nothing: [Texture; 3],
}

impl Gpu {
    pub fn new() -> Result<Gpu, RenderError> {
        let device = made(MTLCreateSystemDefaultDevice(), "device")?;
        let queue = made(device.newCommandQueue(), "command queue")?;
        let library =
            device.newLibraryWithSource_options_error(&NSString::from_str(SHADERS), None).map_err(error)?;
        let function = |name: &str| made(library.newFunctionWithName(&NSString::from_str(name)), name);
        let (vertex, fragment) = (function("vs_main")?, function("fs_main")?);
        let about = MTLRenderPipelineDescriptor::new();
        about.setVertexFunction(Some(&vertex));
        about.setFragmentFunction(Some(&fragment));
        // SAFETY: a render pipeline has a first colour attachment.
        let color = unsafe { about.colorAttachments().objectAtIndexedSubscript(0) };
        // Premultiplied alpha over what is there.
        color.setPixelFormat(TARGET);
        color.setBlendingEnabled(true);
        color.setSourceRGBBlendFactor(MTLBlendFactor::One);
        color.setDestinationRGBBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
        color.setSourceAlphaBlendFactor(MTLBlendFactor::One);
        color.setDestinationAlphaBlendFactor(MTLBlendFactor::OneMinusSourceAlpha);
        let pipeline = device.newRenderPipelineStateWithDescriptor_error(&about).map_err(error)?;
        let about = MTLSamplerDescriptor::new();
        about.setMinFilter(MTLSamplerMinMagFilter::Linear);
        about.setMagFilter(MTLSamplerMinMagFilter::Linear);
        about.setMipFilter(MTLSamplerMipFilter::Linear);
        about.setSAddressMode(MTLSamplerAddressMode::ClampToEdge);
        about.setTAddressMode(MTLSamplerAddressMode::ClampToEdge);
        let sampler = made(device.newSamplerStateWithDescriptor(&about), "sampler")?;
        let read = MTLTextureUsage::ShaderRead;
        let nothing = [
            texture(&device, MTLPixelFormat::R8Unorm, (1, 1), Some(1), false, read)?,
            texture(&device, MTLPixelFormat::RGBA8Unorm, (1, 1), Some(1), false, read)?,
            texture(&device, TARGET, (1, 1), None, false, read)?,
        ];
        Ok(Gpu { device, queue, pipeline, sampler, nothing })
    }

    pub fn commands(&self) -> Result<Commands, RenderError> {
        made(self.queue.commandBuffer(), "command buffer")
    }
}

/// A texture of a size, an array of `slices` when some are asked for, with all its mips
/// when `mips`. In memory the CPU shares with the GPU, as every Mac of Apple's silicon has
/// it: pixels are written to it and read from it without a copy.
pub(crate) fn texture(
    device: &Device,
    format: MTLPixelFormat,
    (width, height): (u32, u32),
    slices: Option<u32>,
    mips: bool,
    usage: MTLTextureUsage,
) -> Result<Texture, RenderError> {
    // SAFETY: a description of a texture of a format and a size of one at least.
    let about = unsafe {
        MTLTextureDescriptor::texture2DDescriptorWithPixelFormat_width_height_mipmapped(
            format,
            width.max(1) as usize,
            height.max(1) as usize,
            mips,
        )
    };
    if let Some(slices) = slices {
        about.setTextureType(MTLTextureType::Type2DArray);
        // SAFETY: an array of one slice at least.
        unsafe { about.setArrayLength(slices.max(1) as usize) };
    }
    about.setUsage(usage);
    about.setStorageMode(MTLStorageMode::Shared);
    made(device.newTextureWithDescriptor(&about), "texture")
}

/// Sends the commands and waits for them: what they drew is there to read afterwards.
pub(crate) fn finish(commands: &Commands) -> Result<(), RenderError> {
    commands.commit();
    commands.waitUntilCompleted();
    commands.error().map_or(Ok(()), |e| Err(error(e)))
}

//! The D3D11 device, the few calls that make its resources, and the renderer's hold on it.

use std::cell::{OnceCell, RefCell};
use std::ffi::c_void;
use std::ops::Deref;
use std::sync::mpsc::Receiver;

use quadra_render::RenderError;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP, D3D_FEATURE_LEVEL_11_0,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11_BIND_SHADER_RESOURCE, D3D11_BOX, D3D11_BUFFER_DESC, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_RESOURCE_MISC_BUFFER_STRUCTURED, D3D11_SDK_VERSION, D3D11_USAGE_DEFAULT, D3D11CreateDevice,
    ID3D11Buffer, ID3D11Device, ID3D11DeviceContext, ID3D11Resource, ID3D11ShaderResourceView,
};
use windows::Win32::Graphics::Dxgi::{DXGI_ERROR_DEVICE_REMOVED, DXGI_ERROR_DEVICE_RESET};
use windows::core::Interface;

use crate::pipeline::Pipeline;

/// What a D3D11 call reported, as the renderer's error.
pub(crate) fn error(e: windows::core::Error) -> RenderError {
    if e.code() == DXGI_ERROR_DEVICE_REMOVED || e.code() == DXGI_ERROR_DEVICE_RESET {
        RenderError::DeviceLost
    } else {
        RenderError::Backend(e.message().to_string())
    }
}

/// A call that fills an out parameter and reports success must have filled it.
pub(crate) fn made<T>(value: Option<T>, what: &str) -> Result<T, RenderError> {
    value.ok_or_else(|| RenderError::Backend(format!("D3D11 returned no {what}")))
}

/// A device, its context, and the pipeline made for it.
pub(crate) struct Device {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext,
    /// The software rasterizer: no GPU, the same pixels on every machine.
    pub software: bool,
    pub pipeline: Pipeline,
}

impl Device {
    pub fn new(software: bool) -> Result<Device, RenderError> {
        let driver = if software { D3D_DRIVER_TYPE_WARP } else { D3D_DRIVER_TYPE_HARDWARE };
        let (mut device, mut context) = (None, None);
        // SAFETY: the out pointers and the feature level slice are valid for the call.
        unsafe {
            D3D11CreateDevice(
                None,
                driver,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
        }
        .map_err(error)?;
        let (device, context) = (made(device, "device")?, made(context, "context")?);
        let pipeline = Pipeline::new(&device)?;
        Ok(Device { device, context, software, pipeline })
    }

    /// A buffer of `capacity` records of `stride` bytes that shaders index, with its view.
    pub fn structured_buffer(
        &self,
        capacity: u32,
        stride: u32,
    ) -> Result<(ID3D11Buffer, ID3D11ShaderResourceView), RenderError> {
        let desc = D3D11_BUFFER_DESC {
            ByteWidth: capacity * stride,
            Usage: D3D11_USAGE_DEFAULT, // UpdateSubresource can write any range of it
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: D3D11_RESOURCE_MISC_BUFFER_STRUCTURED.0 as u32,
            StructureByteStride: stride,
        };
        let (mut buffer, mut view) = (None, None);
        // SAFETY: the description and the out pointers are valid for the calls; a null view
        // description views the whole structured buffer.
        unsafe {
            self.device.CreateBuffer(&desc, None, Some(&mut buffer)).map_err(error)?;
            let buffer = made(buffer, "buffer")?;
            self.device.CreateShaderResourceView(&buffer, None, Some(&mut view)).map_err(error)?;
            Ok((buffer, made(view, "buffer view")?))
        }
    }

    /// Copies `bytes` into a buffer or texture subresource. With a box, into that part of it.
    ///
    /// # Safety
    /// `bytes` must hold what the box (or the whole subresource) takes, at `row_pitch` bytes a
    /// row.
    pub unsafe fn update(
        &self,
        resource: &impl Interface,
        subresource: u32,
        part: Option<D3D11_BOX>,
        bytes: *const c_void,
        row_pitch: u32,
    ) -> Result<(), RenderError> {
        let resource: ID3D11Resource = resource.cast().map_err(error)?;
        let part = part.as_ref().map(|b| b as *const D3D11_BOX);
        // SAFETY: the caller's contract.
        unsafe { self.context.UpdateSubresource(&resource, subresource, part, bytes, row_pitch, 0) };
        Ok(())
    }
}

/// A device on its way from the thread that made it.
pub(crate) struct Handoff(pub Result<Device, RenderError>);

// SAFETY: a D3D11 device and the state objects made from it are free-threaded. The context is
// not, and is used by one thread at a time: the thread that makes a device does nothing more
// with it once it is sent.
unsafe impl Send for Handoff {}

/// The renderer's device: there, or on its way from the thread that makes it at start-up
/// (specs/07). Every call of the renderer that can fail starts with [`Gpu::ready`], which
/// waits for it; after that it is there.
pub(crate) struct Gpu {
    made: OnceCell<Device>,
    coming: RefCell<Option<Receiver<Handoff>>>,
}

impl Gpu {
    pub fn made(device: Device) -> Gpu {
        Gpu { made: OnceCell::from(device), coming: RefCell::new(None) }
    }

    /// The software device, which a worker sends when it has made it.
    pub fn coming(from: Receiver<Handoff>) -> Gpu {
        Gpu { made: OnceCell::new(), coming: RefCell::new(Some(from)) }
    }

    /// The device, if it is there.
    pub fn get(&self) -> Option<&Device> {
        self.made.get()
    }

    /// Waits for the device if it is still on its way, and says why there is none.
    pub fn ready(&self) -> Result<(), RenderError> {
        if self.made.get().is_some() {
            return Ok(());
        }
        let gone = || RenderError::Backend("the thread making the device ended without one".into());
        let device = self.coming.borrow_mut().take().ok_or_else(gone)?.recv().map_err(|_| gone())?.0?;
        let _ = self.made.set(device);
        Ok(())
    }

    /// The device that arrives first is the software one.
    pub fn is_software(&self) -> bool {
        self.made.get().is_none_or(|device| device.software)
    }
}

impl Deref for Gpu {
    type Target = Device;

    fn deref(&self) -> &Device {
        self.made.get().expect("the renderer waits for its device before it uses it")
    }
}

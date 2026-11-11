//! A window's swap chain: flip model, with a frame-latency waitable so a frame starts as late
//! as it can and still be shown (the lowest input latency).

use std::cell::Cell;

use quadra_prim::Color;
use quadra_render::{DeviceSize, RenderError};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND};
use windows::Win32::Graphics::Direct3D11::{ID3D11RenderTargetView, ID3D11Texture2D};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_IGNORE, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_UNKNOWN, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    DXGI_MWA_NO_ALT_ENTER, DXGI_PRESENT, DXGI_RGBA, DXGI_SCALING_NONE, DXGI_SWAP_CHAIN_DESC1,
    DXGI_SWAP_CHAIN_FLAG, DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT, DXGI_SWAP_EFFECT_FLIP_DISCARD,
    DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGIDevice, IDXGIFactory2, IDXGISwapChain2,
};
use windows::core::Interface;

use crate::gpu::{Gpu, error, made};

const FLAGS: i32 = DXGI_SWAP_CHAIN_FLAG_FRAME_LATENCY_WAITABLE_OBJECT.0;

pub(crate) struct SwapChain {
    pub size: DeviceSize,
    pub hwnd: isize,
    chain: IDXGISwapChain2,
    /// The view of the back buffer; none only while the buffers are being resized.
    target: Option<ID3D11RenderTargetView>,
    /// Signaled when the swap chain can take a new frame.
    pub waitable: HANDLE,
    /// The colour last given to the OS for the part of the window no buffer covers.
    background: Cell<Option<Color>>,
}

impl SwapChain {
    pub fn new(gpu: &Gpu, hwnd: isize, size: DeviceSize) -> Result<SwapChain, RenderError> {
        let size = DeviceSize { width: size.width.max(1), height: size.height.max(1) };
        let desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: size.width,
            Height: size.height,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_NONE, // no blurry stretch while the window is resized
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
            AlphaMode: DXGI_ALPHA_MODE_IGNORE,
            Flags: FLAGS as u32,
            ..Default::default()
        };
        let window = HWND(hwnd as *mut _);
        // SAFETY: the device is alive and the description valid for the calls; a handle that
        // is not a window makes the creation fail, which is reported.
        unsafe {
            let device: IDXGIDevice = gpu.device.cast().map_err(error)?;
            let factory: IDXGIFactory2 = device.GetAdapter().and_then(|a| a.GetParent()).map_err(error)?;
            let chain =
                factory.CreateSwapChainForHwnd(&gpu.device, window, &desc, None, None).map_err(error)?;
            let _ = factory.MakeWindowAssociation(window, DXGI_MWA_NO_ALT_ENTER);
            let chain: IDXGISwapChain2 = chain.cast().map_err(error)?;
            chain.SetMaximumFrameLatency(1).map_err(error)?;
            let waitable = chain.GetFrameLatencyWaitableObject();
            let mut made =
                SwapChain { size, hwnd, chain, target: None, waitable, background: Cell::new(None) };
            made.target = Some(made.back_buffer(gpu)?);
            Ok(made)
        }
    }

    pub fn back_texture(&self) -> Result<ID3D11Texture2D, RenderError> {
        // SAFETY: buffer 0 exists in every swap chain.
        unsafe { self.chain.GetBuffer(0) }.map_err(error)
    }

    fn back_buffer(&self, gpu: &Gpu) -> Result<ID3D11RenderTargetView, RenderError> {
        let mut view = None;
        // SAFETY: the texture is alive and the out pointer valid for the call.
        unsafe { gpu.device.CreateRenderTargetView(&self.back_texture()?, None, Some(&mut view)) }
            .map_err(error)?;
        made(view, "back buffer view")
    }

    pub fn target(&self) -> Result<ID3D11RenderTargetView, RenderError> {
        self.target.clone().ok_or(RenderError::BadSurface)
    }

    /// A size of zero (a minimized window) keeps the buffers as they are.
    pub fn resize(&mut self, gpu: &Gpu, size: DeviceSize) -> Result<(), RenderError> {
        if size.width == 0 || size.height == 0 || size == self.size {
            return Ok(());
        }
        self.target = None;
        // SAFETY: nothing refers to the buffers now: the view is dropped and the context
        // unbound, which is what resizing them requires.
        unsafe {
            gpu.context.OMSetRenderTargets(None, None);
            self.chain
                .ResizeBuffers(0, size.width, size.height, DXGI_FORMAT_UNKNOWN, DXGI_SWAP_CHAIN_FLAG(FLAGS))
                .map_err(error)?;
        }
        self.size = size;
        self.target = Some(self.back_buffer(gpu)?);
        Ok(())
    }

    pub fn present(&self) -> Result<(), RenderError> {
        // SAFETY: the swap chain is alive; presenting has no other precondition.
        unsafe { self.chain.Present(1, DXGI_PRESENT(0)) }.ok().map_err(error)
    }

    /// Gives the OS the colour of the window area no buffer covers: while a window grows
    /// faster than frames arrive, the gap shows the canvas colour, not black.
    pub fn set_background(&self, color: Color) {
        if self.background.replace(Some(color)) != Some(color) {
            let rgba = DXGI_RGBA { r: color.r, g: color.g, b: color.b, a: 1.0 };
            // SAFETY: the colour is valid for the call.
            let _ = unsafe { self.chain.SetBackgroundColor(&rgba) };
        }
    }
}

impl Drop for SwapChain {
    fn drop(&mut self) {
        // SAFETY: the handle was given by this swap chain and is closed once, here.
        let _ = unsafe { CloseHandle(self.waitable) };
    }
}

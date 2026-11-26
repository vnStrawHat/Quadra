//! What a scene is drawn into (specs/07, "Surfaces"): a texture that can be read back, for
//! tests and snapshots, or the layer of a window's view.

use std::ptr::NonNull;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2_app_kit::NSView;
use objc2_core_foundation::CGSize;
use objc2_metal::{MTLOrigin, MTLRegion, MTLSize, MTLTexture, MTLTextureUsage};
use objc2_quartz_core::{CAMetalDrawable, CAMetalLayer};
use quadra_render::{DeviceSize, Pixels, RenderError, SurfaceTarget};

use super::gpu::{Commands, Gpu, TARGET, Texture, WINDOW, texture};

pub(crate) type Drawable = Retained<ProtocolObject<dyn CAMetalDrawable>>;

pub(crate) enum Shown {
    /// Read back.
    Texture(Texture),
    /// In a window's view. `drawn` is the frame drawn into it that waits to be shown: its
    /// commands, not sent yet, and what they drew into.
    Layer { layer: Retained<CAMetalLayer>, drawn: Option<(Commands, Drawable)> },
}

pub(crate) struct Surface {
    pub size: DeviceSize,
    pub shown: Shown,
}

fn at_least_one(size: DeviceSize) -> DeviceSize {
    DeviceSize { width: size.width.max(1), height: size.height.max(1) }
}

fn of(size: DeviceSize) -> CGSize {
    CGSize { width: f64::from(size.width), height: f64::from(size.height) }
}

impl Surface {
    pub fn new(gpu: &Gpu, target: SurfaceTarget, size: DeviceSize) -> Result<Surface, RenderError> {
        let size = at_least_one(size);
        let shown = match target {
            SurfaceTarget::Offscreen => {
                // Drawn into, read by the copy a blurring box takes of it, and read back.
                let usage = MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead;
                Shown::Texture(texture(&gpu.device, TARGET, (size.width, size.height), None, false, usage)?)
            }
            SurfaceTarget::AppKit { view } => {
                // SAFETY: the address is of the content view of a window of this thread, the
                // main one, which is alive as long as the window: the app drops a window's
                // surface before the window.
                let view = unsafe { &*(view as *const NSView) };
                let layer = CAMetalLayer::new();
                layer.setDevice(Some(&gpu.device));
                layer.setPixelFormat(WINDOW);
                // What is drawn is read too: by the copy a blurring box takes of it.
                layer.setFramebufferOnly(false);
                layer.setDrawableSize(of(size));
                view.setWantsLayer(true);
                view.setLayer(Some(&layer));
                Shown::Layer { layer, drawn: None }
            }
            SurfaceTarget::Win32 { .. } => return Err(RenderError::BadSurface),
        };
        Ok(Surface { size, shown })
    }

    /// Another size: a new texture, or the layer's next frames at it.
    pub fn resize(&mut self, gpu: &Gpu, size: DeviceSize) -> Result<(), RenderError> {
        match &self.shown {
            Shown::Texture(_) => *self = Surface::new(gpu, SurfaceTarget::Offscreen, size)?,
            Shown::Layer { layer, .. } => {
                self.size = at_least_one(size);
                layer.setDrawableSize(of(self.size));
            }
        }
        Ok(())
    }

    /// The pixels drawn, rows from the top: of a texture, not of a window.
    pub fn read(&self) -> Result<Pixels, RenderError> {
        let Shown::Texture(texture) = &self.shown else { return Err(RenderError::BadSurface) };
        let (width, height) = (self.size.width as usize, self.size.height as usize);
        let mut rgba = vec![0u8; width * height * 4];
        let all =
            MTLRegion { origin: MTLOrigin { x: 0, y: 0, z: 0 }, size: MTLSize { width, height, depth: 1 } };
        // SAFETY: the buffer is `height` rows of `width` pixels of four bytes, which is what
        // the region of the texture's format gives.
        unsafe {
            texture.getBytes_bytesPerRow_fromRegion_mipmapLevel(
                NonNull::from(rgba.as_mut_slice()).cast(),
                width * 4,
                all,
                0,
            );
        }
        Ok(Pixels { size: self.size, rgba })
    }
}

//! Start-up on two devices (specs/07, "Start-up"): one thread makes the software device, which
//! takes ten milliseconds and draws the first frames, then the hardware device, which takes
//! over a hundred. Meanwhile the UI thread opens its window and builds its view.

use std::sync::mpsc::{Receiver, channel};

use quadra_platform::{Platform, Waker};
use quadra_render::RenderError;

use crate::gpu::{Device, Gpu, Handoff};
use crate::{D3d11Renderer, WinPlatform};

/// The hardware device on its way.
pub(crate) struct Upgrade {
    ready: Receiver<Handoff>,
    arrived: Option<Result<Device, RenderError>>,
}

impl Upgrade {
    /// Starts the thread that makes both devices. `wake` is called, on that thread, when the
    /// hardware one is there.
    fn start(wake: Waker) -> (Gpu, Upgrade) {
        let ((first, software), (second, hardware)) = (channel(), channel());
        std::thread::spawn(move || {
            // One after the other: a hardware driver holds up whatever loads a library while
            // it starts, the software device too. Nobody is left to take a device when the
            // renderer is gone: nothing to do then.
            let _ = first.send(Handoff(Device::new(true)));
            let _ = second.send(Handoff(Device::new(false)));
            wake.wake();
        });
        (Gpu::coming(software), Upgrade { ready: hardware, arrived: None })
    }

    /// Whether the worker is done, with a device or without one.
    pub fn is_ready(&mut self) -> bool {
        if self.arrived.is_none() {
            self.arrived = self.ready.try_recv().ok().map(|handoff| handoff.0);
        }
        self.arrived.is_some()
    }

    /// The device that arrived, if one did.
    pub fn take(self) -> Option<Device> {
        self.arrived.and_then(Result::ok)
    }
}

impl D3d11Renderer {
    /// A renderer for an app's start. It returns at once: its devices are made on another
    /// thread. The first thing asked of it waits for the software device; it moves to the
    /// hardware device when that is ready, which it makes known by wanting a frame and then
    /// reporting its device lost. Its window surfaces pace that platform's loop.
    pub fn starting(platform: &WinPlatform) -> D3d11Renderer {
        let (gpu, upgrade) = Upgrade::start(platform.waker());
        let mut renderer = D3d11Renderer::on(gpu, platform.0.pacing.clone());
        renderer.upgrade = Some(upgrade);
        renderer
    }
}

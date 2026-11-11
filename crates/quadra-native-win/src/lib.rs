//! Quadra on Windows (specs/10, "The native crate"): the D3D11 renderer first; the DirectWrite
//! text backend and the Win32 platform follow.
//!
//! This is one of the two crates that may hold `unsafe` code (specs/01, R16): calls into the
//! operating system. Every block says what makes it sound.

mod a11y;
mod atlas;
mod clipboard;
mod dwrite;
mod gpu;
mod ime;
mod input;
mod layers;
mod pacing;
mod pipeline;
mod platform;
mod renderer;
mod surface;
mod swapchain;
mod upgrade;
mod window;
mod wndproc;

pub use dwrite::DirectWrite;
pub use platform::WinPlatform;
pub use renderer::D3d11Renderer;

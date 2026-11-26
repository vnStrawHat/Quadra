//! Quadra on macOS (specs/23): the AppKit platform, the Metal renderer and the Core Text
//! text backend. Nothing here is built for another system.
#![cfg(target_os = "macos")]

mod appkit;
mod coretext;
mod metal;

pub use appkit::MacPlatform;
pub use coretext::{CoreText, SYSTEM_FAMILY};
pub use metal::MetalRenderer;

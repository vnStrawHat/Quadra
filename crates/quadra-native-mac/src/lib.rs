//! Quadra on macOS (specs/23): the AppKit platform, the Metal renderer, the Core Text text
//! backend and the system's image codecs. Nothing here is built for another system.
#![cfg(target_os = "macos")]

mod appkit;
mod coretext;
mod images;
mod metal;

pub use appkit::MacPlatform;
pub use coretext::{CoreText, SYSTEM_FAMILY};
pub use images::ImageIo;
pub use metal::MetalRenderer;

//! Quadra on macOS (specs/23): the Metal renderer and the Core Text text backend. The AppKit
//! platform is to come. Nothing here is built for another system.
#![cfg(target_os = "macos")]

mod coretext;
mod metal;

pub use coretext::{CoreText, SYSTEM_FAMILY};
pub use metal::MetalRenderer;

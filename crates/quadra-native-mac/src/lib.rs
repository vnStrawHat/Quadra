//! Quadra on macOS (specs/23): the Core Text text backend. The Metal renderer and the AppKit
//! platform are to come. Nothing here is built for another system.
#![cfg(target_os = "macos")]

mod coretext;

pub use coretext::{CoreText, SYSTEM_FAMILY};

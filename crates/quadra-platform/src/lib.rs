//! The OS boundary (specs/10): the traits a windowing system implements, the events it
//! delivers, and the headless platform that tests and benches run on.
//!
//! Nothing here names an operating system concept; what is specific to one OS lives in that
//! OS's native crate.

mod clipboard;
mod event;
mod headless;
mod image;
mod keys;
mod platform;
mod system;
mod window;

pub use clipboard::{Clipboard, MemoryClipboard};
pub use event::{Mods, MouseButton, PlatformEvent, PointerKind};
pub use headless::HeadlessPlatform;
pub use image::{
    Decoded, FlatImages, ImageDecoder, ImageError, ImageInfo, MAX_IMAGE_FILE_BYTES, MAX_IMAGE_PIXELS,
    MAX_IMAGE_SIDE, MAX_PIXELS_PER_BYTE, admit, quarter,
};
pub use keys::Key;
pub use platform::{Platform, PlatformError, PlatformHandler, TimerId, Waker};
pub use system::{Appearance, Dropped, DroppedFiles, DroppedText, SystemColors};
pub use window::{
    Cursor, HitRegion, Placement, PlatformWindow, WindowHandle, WindowId, WindowOptions, fit_size,
};

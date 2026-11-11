//! The OS boundary (specs/10): the traits a windowing system implements, the events it
//! delivers, and the headless platform that tests and benches run on.
//!
//! Nothing here names an operating system concept; what is specific to one OS lives in that
//! OS's native crate.

mod clipboard;
mod event;
mod headless;
mod keys;
mod platform;
mod window;

pub use clipboard::{Clipboard, MemoryClipboard};
pub use event::{Mods, MouseButton, PlatformEvent, PointerKind};
pub use headless::HeadlessPlatform;
pub use keys::Key;
pub use platform::{Platform, PlatformError, PlatformHandler, TimerId, Waker};
pub use window::{Cursor, HitRegion, PlatformWindow, WindowHandle, WindowId, WindowOptions, fit_size};

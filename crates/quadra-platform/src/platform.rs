//! The platform: the message loop, windows, timers, the clock.

use std::fmt;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::clipboard::Clipboard;
use crate::event::PlatformEvent;
use quadra_prim::Point;

use crate::window::{HitRegion, PlatformWindow, WindowId, WindowOptions};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TimerId(pub u64);

/// What the OS refused, in its own words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformError(pub String);

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the platform failed: {}", self.0)
    }
}

impl std::error::Error for PlatformError {}

/// Wakes the loop from any thread: [`PlatformHandler::wake`] then runs on the UI thread.
/// Several wakes before the loop turns may arrive as one.
#[derive(Clone)]
pub struct Waker(Arc<dyn Fn() + Send + Sync>);

impl Waker {
    pub fn new(wake: impl Fn() + Send + Sync + 'static) -> Waker {
        Waker(Arc::new(wake))
    }

    pub fn wake(&self) {
        (self.0)()
    }
}

/// Every method takes `&self`: the handler calls the platform from inside [`Platform::run`].
pub trait Platform {
    /// The message loop. Returns after [`Platform::quit`].
    fn run(&self, handler: &mut dyn PlatformHandler) -> Result<(), PlatformError>;
    fn quit(&self);
    fn open_window(&self, options: &WindowOptions) -> Result<Box<dyn PlatformWindow>, PlatformError>;
    fn waker(&self) -> Waker;
    /// A timer that fires once after `after`, or again every `after`.
    fn set_timer(&self, after: Duration, repeat: bool) -> TimerId;
    fn cancel_timer(&self, id: TimerId);
    /// The clipboard. The headless platform's is a variable in memory.
    fn clipboard(&self) -> Rc<dyn Clipboard>;
    /// The time, which only moves forward. The headless platform's is moved by its test.
    fn now(&self) -> Instant;
}

/// The app's side. It is never entered twice: what an OS call inside a handler sets off is
/// delivered after the handler returns.
pub trait PlatformHandler {
    fn event(&mut self, window: WindowId, event: PlatformEvent);
    /// The window can show a frame now, and one was requested.
    fn frame(&mut self, window: WindowId);
    fn timer(&mut self, id: TimerId);
    /// A [`Waker`] fired: run what other threads posted.
    fn wake(&mut self);
    /// What a point of a window with its own frame is, in logical pixels of the window. It
    /// is asked while the pointer moves, and answered at once; a platform that cannot ask
    /// (the handler is running) takes the point for the app's.
    fn hit_test(&mut self, _window: WindowId, _at: Point) -> HitRegion {
        HitRegion::Client
    }
}

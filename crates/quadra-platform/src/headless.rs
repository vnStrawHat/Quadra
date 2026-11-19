//! The headless platform (specs/10): windows without an OS window, a clock a test moves, and
//! events a test queues. `run` is replaced by stepping.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use quadra_a11y::TreeUpdate;
use quadra_prim::{Rect, Size};

use crate::clipboard::{Clipboard, MemoryClipboard};
use crate::event::PlatformEvent;
use crate::platform::{Platform, PlatformError, PlatformHandler, TimerId, Waker};
use crate::system::Appearance;
use crate::window::{Cursor, Placement, PlatformWindow, WindowId, WindowOptions, at_least};

mod window;

struct Timer {
    id: TimerId,
    due: Duration,
    every: Option<Duration>,
}

struct WindowState {
    id: WindowId,
    size: Cell<Size>,
    /// What a resize is held at, as the OS holds a drag of the border.
    min: Size,
    scale: Cell<f32>,
    title: RefCell<String>,
    frame: Cell<bool>,
    text_input: Cell<Option<Rect>>,
    cursor: Cell<Cursor>,
    semantics: RefCell<Vec<TreeUpdate>>,
    minimized: Cell<bool>,
    maximized: Cell<bool>,
    /// Where the window is on a desktop of no displays: any place is on one.
    at: Cell<(i32, i32)>,
}

struct Inner {
    start: Instant,
    /// The time since `start`, moved only by [`HeadlessPlatform::advance`].
    clock: Cell<Duration>,
    next_id: Cell<u64>,
    windows: RefCell<Vec<Weak<WindowState>>>,
    events: RefCell<VecDeque<(WindowId, PlatformEvent)>>,
    timers: RefCell<Vec<Timer>>,
    woken: Arc<AtomicBool>,
    quit: Cell<bool>,
    clipboard: Rc<MemoryClipboard>,
    decoder: RefCell<Arc<dyn crate::image::ImageDecoder>>,
    utc_offset: Cell<i32>,
    appearance: Cell<Appearance>,
}

#[derive(Clone)]
pub struct HeadlessPlatform(Rc<Inner>);

struct HeadlessWindow(Rc<WindowState>);

impl Default for HeadlessPlatform {
    fn default() -> Self {
        HeadlessPlatform::new()
    }
}

impl HeadlessPlatform {
    pub fn new() -> HeadlessPlatform {
        HeadlessPlatform(Rc::new(Inner {
            start: Instant::now(),
            clock: Cell::new(Duration::ZERO),
            next_id: Cell::new(0),
            windows: RefCell::new(Vec::new()),
            events: RefCell::new(VecDeque::new()),
            timers: RefCell::new(Vec::new()),
            woken: Arc::new(AtomicBool::new(false)),
            quit: Cell::new(false),
            clipboard: Rc::default(),
            decoder: RefCell::new(Arc::new(crate::image::FlatImages)),
            utc_offset: Cell::new(0),
            appearance: Cell::new(Appearance::default()),
        }))
    }

    fn next_id(&self) -> u64 {
        self.0.next_id.set(self.0.next_id.get() + 1);
        self.0.next_id.get()
    }

    fn window(&self, id: WindowId) -> Option<Rc<WindowState>> {
        self.0.windows.borrow().iter().filter_map(Weak::upgrade).find(|w| w.id == id)
    }

    /// Queues an event, as the OS would deliver it.
    pub fn push(&self, window: WindowId, event: PlatformEvent) {
        self.0.events.borrow_mut().push_back((window, event));
    }

    /// Gives a window a new size, no smaller than its minimum, and scale, and queues the
    /// event that says so.
    pub fn resize(&self, window: WindowId, size: Size, scale: f32) {
        if let Some(w) = self.window(window) {
            let size = at_least(size, w.min);
            w.size.set(size);
            w.scale.set(scale);
            self.push(window, PlatformEvent::Resize { size, scale });
        }
    }

    /// The caret a window was last told of: `None` while nothing that takes text has the
    /// focus.
    pub fn text_input(&self, window: WindowId) -> Option<Rect> {
        self.window(window).and_then(|w| w.text_input.get())
    }

    /// The semantic updates a window was given since this was last asked.
    pub fn take_semantics(&self, window: WindowId) -> Vec<TreeUpdate> {
        self.window(window).map(|w| w.semantics.take()).unwrap_or_default()
    }

    /// The cursor a window was last given.
    pub fn cursor(&self, window: WindowId) -> Option<Cursor> {
        self.window(window).map(|w| w.cursor.get())
    }

    /// Whether a window was minimized, and whether it is maximized.
    pub fn is_minimized(&self, window: WindowId) -> bool {
        self.window(window).is_some_and(|w| w.minimized.get())
    }

    pub fn is_maximized(&self, window: WindowId) -> bool {
        self.window(window).is_some_and(|w| w.maximized.get())
    }

    /// Where a window is: as it was opened, moved and resized.
    pub fn placement(&self, window: WindowId) -> Option<Placement> {
        self.window(window).map(|w| HeadlessWindow(w).placement())
    }

    /// The user moves a window.
    pub fn move_window(&self, window: WindowId, x: i32, y: i32) {
        if let Some(w) = self.window(window) {
            w.at.set((x, y));
        }
    }

    /// The title a window was last given.
    pub fn title(&self, window: WindowId) -> Option<String> {
        self.window(window).map(|w| w.title.borrow().clone())
    }

    /// The offset of the time zone the platform says, in seconds: 0 unless a test sets it.
    /// Sets the system's appearance, and tells every window, as the OS does.
    pub fn set_appearance(&self, appearance: Appearance) {
        self.0.appearance.set(appearance);
        for window in self.0.windows.borrow().iter().filter_map(Weak::upgrade) {
            self.push(window.id, PlatformEvent::Appearance(appearance));
        }
    }

    pub fn set_utc_offset(&self, seconds: i32) {
        self.0.utc_offset.set(seconds);
    }

    /// Gives the platform another decoder than that of flat test images: the system's, for a
    /// tool that draws real files without a window. An app takes it when it is made.
    pub fn set_image_decoder(&self, decoder: Arc<dyn crate::image::ImageDecoder>) {
        self.0.decoder.replace(decoder);
    }

    /// One turn of the loop: the queued events, a wake if a waker fired, then a frame for
    /// each window that asked for one. Returns whether anything was delivered.
    pub fn step(&self, handler: &mut dyn PlatformHandler) -> bool {
        let mut worked = false;
        loop {
            let next = self.0.events.borrow_mut().pop_front();
            let Some((window, event)) = next else { break };
            handler.event(window, event);
            worked = true;
        }
        if self.0.woken.swap(false, Ordering::AcqRel) {
            handler.wake();
            worked = true;
        }
        let windows: Vec<_> = self.0.windows.borrow().iter().filter_map(Weak::upgrade).collect();
        self.0.windows.borrow_mut().retain(|w| w.strong_count() > 0);
        for window in windows {
            if window.frame.replace(false) {
                handler.frame(window.id);
                worked = true;
            }
        }
        worked
    }

    /// Moves the clock forward, firing the timers that come due in order, each at its time,
    /// with a [`HeadlessPlatform::step`] after each and at the end.
    pub fn advance(&self, by: Duration, handler: &mut dyn PlatformHandler) {
        let end = self.0.clock.get() + by;
        loop {
            let due = {
                let mut timers = self.0.timers.borrow_mut();
                let first =
                    (0..timers.len()).filter(|&i| timers[i].due <= end).min_by_key(|&i| timers[i].due);
                first.map(|i| match timers[i].every {
                    Some(every) => {
                        let at = timers[i].due;
                        timers[i].due += every;
                        (timers[i].id, at)
                    }
                    None => {
                        let timer = timers.remove(i);
                        (timer.id, timer.due)
                    }
                })
            };
            let Some((id, at)) = due else { break };
            self.0.clock.set(at.max(self.0.clock.get()));
            handler.timer(id);
            self.step(handler);
        }
        self.0.clock.set(end);
        self.step(handler);
    }
}

impl Platform for HeadlessPlatform {
    /// Steps until nothing is left to deliver, or until `quit`.
    fn run(&self, handler: &mut dyn PlatformHandler) -> Result<(), PlatformError> {
        while !self.0.quit.get() && self.step(handler) {}
        Ok(())
    }

    fn quit(&self) {
        self.0.quit.set(true);
    }

    fn open_window(&self, options: &WindowOptions) -> Result<Box<dyn PlatformWindow>, PlatformError> {
        let placed = options.placement;
        let size = placed.map_or(options.size, |p| Size::new(p.width as f32, p.height as f32));
        let state = Rc::new(WindowState {
            id: WindowId(self.next_id() as u32),
            size: Cell::new(at_least(size, options.min_size)),
            min: options.min_size,
            scale: Cell::new(1.0),
            title: RefCell::new(options.title.clone()),
            frame: Cell::new(false),
            text_input: Cell::new(None),
            cursor: Cell::default(),
            semantics: RefCell::default(),
            minimized: Cell::new(false),
            maximized: Cell::new(placed.is_some_and(|p| p.maximized)),
            at: Cell::new(placed.map_or((0, 0), |p| (p.x, p.y))),
        });
        self.0.windows.borrow_mut().push(Rc::downgrade(&state));
        Ok(Box::new(HeadlessWindow(state)))
    }

    fn waker(&self) -> Waker {
        let woken = self.0.woken.clone();
        Waker::new(move || woken.store(true, Ordering::Release))
    }

    fn set_timer(&self, after: Duration, repeat: bool) -> TimerId {
        let after = after.max(Duration::from_millis(1)); // as an OS timer: never a busy loop
        let id = TimerId(self.next_id());
        let due = self.0.clock.get() + after;
        self.0.timers.borrow_mut().push(Timer { id, due, every: repeat.then_some(after) });
        id
    }

    fn cancel_timer(&self, id: TimerId) {
        self.0.timers.borrow_mut().retain(|t| t.id != id);
    }

    fn now(&self) -> Instant {
        self.0.start + self.0.clock.get()
    }

    fn clipboard(&self) -> Rc<dyn Clipboard> {
        self.0.clipboard.clone()
    }

    fn image_decoder(&self) -> Arc<dyn crate::image::ImageDecoder> {
        self.0.decoder.borrow().clone()
    }

    fn utc_offset(&self, _unix_seconds: i64) -> i32 {
        self.0.utc_offset.get()
    }

    fn appearance(&self) -> Appearance {
        self.0.appearance.get()
    }
}

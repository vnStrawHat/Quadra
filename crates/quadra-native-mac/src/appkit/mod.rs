//! The AppKit platform (specs/10; specs/23): the loop, timers, wakes and windows. All of it
//! lives on the main thread, as AppKit does.

mod access;
mod buttons;
mod delivery;
mod drops;
mod input;
mod keys;
mod look;
mod menu;
mod place;
mod view;
mod window;

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::VecDeque;
use std::ptr::NonNull;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSEvent, NSEventMask, NSEventModifierFlags, NSEventType,
    NSPasteboard, NSPasteboardTypeString,
};
use objc2_foundation::{NSDate, NSDefaultRunLoopMode, NSPoint, NSString, NSTimeZone};
use quadra_platform::{
    Appearance, Clipboard, ImageDecoder, KeyNames, Platform, PlatformError, PlatformHandler, PlatformWindow,
    TimerId, Waker, WindowOptions,
};

use delivery::Delivery;

struct Timer {
    id: u64,
    due: Instant,
    /// How long after it fires it fires again; none for a timer that fires once.
    every: Option<Duration>,
}

pub(crate) struct Shared {
    mtm: MainThreadMarker,
    pub windows: RefCell<Vec<Weak<window::State>>>,
    queue: RefCell<VecDeque<Delivery>>,
    /// The handler `run` was given, while it runs.
    handler: Cell<Option<NonNull<dyn PlatformHandler>>>,
    /// A handler method is on the stack: what arrives now waits in the queue.
    busy: Cell<bool>,
    timers: RefCell<Vec<Timer>>,
    next_id: Cell<u32>,
    quitting: Cell<bool>,
    /// A waker fired, and the event that turns the loop for it is posted.
    woken: Arc<AtomicBool>,
    /// What answers the menu bar's Quit: the application holds it weakly.
    delegate: OnceCell<objc2::rc::Retained<menu::AppDelegate>>,
}

impl Shared {
    pub fn next_id(&self) -> u32 {
        self.next_id.set(self.next_id.get() + 1);
        self.next_id.get()
    }

    /// The timers whose time has come: each is given to the handler, and set again when it
    /// repeats.
    fn timers_due(&self) {
        let now = Instant::now();
        let due: Vec<u64> = {
            let mut timers = self.timers.borrow_mut();
            let due = timers.iter().filter(|it| it.due <= now).map(|it| it.id).collect();
            timers.retain_mut(|it| match (it.due <= now, it.every) {
                (false, _) => true,
                (true, None) => false,
                (true, Some(every)) => {
                    it.due = now + every;
                    true
                }
            });
            due
        };
        for id in due {
            self.deliver(Delivery::Timer(TimerId(id)));
        }
    }

    /// How long the loop may sleep: not at all while a window asks for a frame, until the
    /// next timer otherwise, for ever when there is none.
    fn sleep(&self) -> Option<Duration> {
        if self.windows.borrow().iter().filter_map(Weak::upgrade).any(|it| it.wants_frame.get()) {
            return Some(Duration::ZERO);
        }
        let next = self.timers.borrow().iter().map(|it| it.due).min()?;
        Some(next.saturating_duration_since(Instant::now()))
    }
}

/// Posts an event that does nothing but turn the loop: from any thread.
fn turn() {
    // SAFETY: the application object exists (the platform made it on the main thread before
    // any waker), and posting an event to it is what AppKit lets any thread do. The marker
    // is used for nothing but to name that object.
    let mtm = unsafe { MainThreadMarker::new_unchecked() };
    let event =
        NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
            NSEventType::ApplicationDefined,
            NSPoint::new(0.0, 0.0),
            NSEventModifierFlags::empty(),
            0.0,
            0,
            None,
            0,
            0,
            0,
        );
    if let Some(event) = event {
        NSApplication::sharedApplication(mtm).postEvent_atStart(&event, false);
    }
}

/// The system's clipboard, as text.
struct Pasteboard;

impl Clipboard for Pasteboard {
    fn read_text(&self) -> Option<String> {
        // SAFETY: the name of the type is AppKit's constant.
        let text = NSPasteboard::generalPasteboard().stringForType(unsafe { NSPasteboardTypeString });
        text.map(|it| it.to_string())
    }

    fn write_text(&self, text: &str) {
        let board = NSPasteboard::generalPasteboard();
        board.clearContents();
        // SAFETY: as above.
        board.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString });
    }
}

/// The AppKit platform. Clones share it; all of it lives on the main thread.
#[derive(Clone)]
pub struct MacPlatform(pub(crate) Rc<Shared>);

impl MacPlatform {
    /// On the main thread alone: AppKit runs nowhere else.
    pub fn new() -> Result<MacPlatform, PlatformError> {
        let mtm = MainThreadMarker::new()
            .ok_or_else(|| PlatformError("AppKit runs on the main thread alone".to_string()))?;
        // An app with a place in the Dock and a menu bar, whatever started it.
        NSApplication::sharedApplication(mtm).setActivationPolicy(NSApplicationActivationPolicy::Regular);
        let shared = Rc::new(Shared {
            mtm,
            windows: RefCell::new(Vec::new()),
            queue: RefCell::new(VecDeque::new()),
            handler: Cell::new(None),
            busy: Cell::new(false),
            timers: RefCell::new(Vec::new()),
            next_id: Cell::new(0),
            quitting: Cell::new(false),
            woken: Arc::new(AtomicBool::new(false)),
            delegate: OnceCell::new(),
        });
        let _ = shared.delegate.set(menu::install(mtm, &shared));
        Ok(MacPlatform(shared))
    }
}

impl Platform for MacPlatform {
    fn run(&self, handler: &mut dyn PlatformHandler) -> Result<(), PlatformError> {
        let s = &*self.0;
        if s.handler.get().is_some() {
            return Err(PlatformError("run was called from inside run".into()));
        }
        struct Clear<'a>(&'a Shared);
        impl Drop for Clear<'_> {
            fn drop(&mut self) {
                self.0.handler.set(None);
            }
        }
        let handler: NonNull<dyn PlatformHandler + '_> = NonNull::from(handler);
        // SAFETY: only the lifetime changes. The pointer is used while this call is on the
        // stack and cleared by `Clear` when it leaves.
        s.handler.set(Some(unsafe {
            std::mem::transmute::<NonNull<dyn PlatformHandler + '_>, NonNull<dyn PlatformHandler>>(handler)
        }));
        let _clear = Clear(s);
        s.quitting.set(false);
        let app = NSApplication::sharedApplication(s.mtm);
        app.finishLaunching();
        s.drain(); // what was queued before the loop had a handler
        while !s.quitting.get() {
            s.drain(); // what was put off until the loop turned
            s.frames();
            let until = match s.sleep() {
                None => NSDate::distantFuture(),
                Some(sleep) => NSDate::dateWithTimeIntervalSinceNow(sleep.as_secs_f64()),
            };
            // SAFETY: the mode is the run loop's constant.
            let mode = unsafe { NSDefaultRunLoopMode };
            let event = app.nextEventMatchingMask_untilDate_inMode_dequeue(
                NSEventMask::Any,
                Some(&until),
                mode,
                true,
            );
            // An event of the app's own only turns the loop; every other is AppKit's to send
            // to its window, whose view and delegate tell the handler.
            if let Some(event) = event.filter(|it| it.r#type() != NSEventType::ApplicationDefined) {
                app.sendEvent(&event);
            }
            s.timers_due();
            if s.woken.swap(false, Ordering::AcqRel) {
                s.deliver(Delivery::Wake);
            }
        }
        Ok(())
    }

    fn quit(&self) {
        self.0.quitting.set(true);
        turn();
    }

    fn open_window(&self, options: &WindowOptions) -> Result<Box<dyn PlatformWindow>, PlatformError> {
        Ok(Box::new(window::MacWindow::open(&self.0, options)))
    }

    fn waker(&self) -> Waker {
        let woken = self.0.woken.clone();
        Waker::new(move || {
            if !woken.swap(true, Ordering::AcqRel) {
                turn();
            }
        })
    }

    fn set_timer(&self, after: Duration, repeat: bool) -> TimerId {
        let id = u64::from(self.0.next_id());
        let timer = Timer { id, due: Instant::now() + after, every: repeat.then_some(after) };
        self.0.timers.borrow_mut().push(timer);
        TimerId(id)
    }

    fn cancel_timer(&self, id: TimerId) {
        self.0.timers.borrow_mut().retain(|it| it.id != id.0);
    }

    fn clipboard(&self) -> Rc<dyn Clipboard> {
        Rc::new(Pasteboard)
    }

    fn image_decoder(&self) -> Arc<dyn ImageDecoder> {
        Arc::new(crate::images::ImageIo)
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn utc_offset(&self, unix_seconds: i64) -> i32 {
        let then = NSDate::dateWithTimeIntervalSince1970(unix_seconds as f64);
        NSTimeZone::localTimeZone().secondsFromGMTForDate(&then) as i32
    }

    fn key_names(&self) -> KeyNames {
        KeyNames::Signs
    }

    /// As the app is shown now.
    fn appearance(&self) -> Appearance {
        look::appearance(&NSApplication::sharedApplication(self.0.mtm).effectiveAppearance())
    }
}

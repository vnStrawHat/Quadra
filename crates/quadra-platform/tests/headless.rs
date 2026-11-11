//! The headless platform: a clock the test moves, events the test queues, no OS window.

use std::time::Duration;

use quadra_platform::{
    HeadlessPlatform, Key, Mods, Platform, PlatformEvent, PlatformHandler, PlatformWindow, TimerId, WindowId,
    WindowOptions, fit_size,
};
use quadra_prim::Size;

/// Writes down what it is told, with the platform's time in milliseconds.
struct Log {
    platform: HeadlessPlatform,
    start: std::time::Instant,
    lines: Vec<String>,
    /// A window to ask for another frame from inside `frame`, this many times.
    again: Option<(Box<dyn PlatformWindow>, u32)>,
}

impl Log {
    fn new(platform: &HeadlessPlatform) -> Log {
        Log { platform: platform.clone(), start: platform.now(), lines: Vec::new(), again: None }
    }

    fn take(&mut self) -> Vec<String> {
        std::mem::take(&mut self.lines)
    }
}

impl PlatformHandler for Log {
    fn event(&mut self, window: WindowId, event: PlatformEvent) {
        self.lines.push(format!("event {} {event:?}", window.0));
    }

    fn frame(&mut self, window: WindowId) {
        self.lines.push(format!("frame {}", window.0));
        if let Some((w, left)) = &mut self.again
            && *left > 0
        {
            *left -= 1;
            w.request_frame();
        }
    }

    fn timer(&mut self, id: TimerId) {
        let at = (self.platform.now() - self.start).as_millis();
        self.lines.push(format!("timer {} at {at}", id.0));
    }

    fn wake(&mut self) {
        self.lines.push("wake".into());
    }
}

#[test]
fn timers_fire_in_order_at_their_time_on_the_virtual_clock() {
    let p = HeadlessPlatform::new();
    let mut log = Log::new(&p);
    let slow = p.set_timer(Duration::from_millis(50), false);
    let fast = p.set_timer(Duration::from_millis(20), true);
    p.advance(Duration::from_millis(10), &mut log);
    assert!(log.take().is_empty()); // nothing is due, and no real time is waited for
    p.advance(Duration::from_millis(50), &mut log);
    assert_eq!(
        log.take(),
        [
            format!("timer {} at 20", fast.0),
            format!("timer {} at 40", fast.0),
            format!("timer {} at 50", slow.0),
            format!("timer {} at 60", fast.0)
        ]
    );
    p.cancel_timer(fast);
    p.advance(Duration::from_secs(10), &mut log);
    assert!(log.take().is_empty()); // one was cancelled, the other fired once
    assert_eq!(p.now() - log.start, Duration::from_millis(10_060));
}

#[test]
fn a_step_delivers_events_in_order_then_the_frames_that_were_asked_for() {
    let p = HeadlessPlatform::new();
    let mut log = Log::new(&p);
    let (a, b) =
        (p.open_window(&WindowOptions::new("a")).unwrap(), p.open_window(&WindowOptions::new("b")).unwrap());
    assert!(!p.step(&mut log)); // nothing to do
    b.request_frame();
    let key = PlatformEvent::Key { key: Key::A, code: 65, mods: Mods::CTRL, down: true, repeat: false };
    p.push(a.id(), PlatformEvent::Text("x".into()));
    p.push(b.id(), key.clone());
    assert!(p.step(&mut log));
    let (ia, ib) = (a.id().0, b.id().0);
    assert_eq!(
        log.take(),
        [format!("event {ia} Text(\"x\")"), format!("event {ib} {key:?}"), format!("frame {ib}")]
    );
    assert!(!p.step(&mut log)); // a frame is drawn once per request
}

#[test]
fn a_resize_changes_what_the_window_reports_and_says_so() {
    let p = HeadlessPlatform::new();
    let mut log = Log::new(&p);
    let w = p.open_window(&WindowOptions::new("w").size(400.0, 300.0)).unwrap();
    assert_eq!((w.size(), w.scale()), (Size::new(400.0, 300.0), 1.0));
    p.resize(w.id(), Size::new(200.0, 100.0), 1.5);
    assert_eq!((w.size(), w.scale()), (Size::new(200.0, 100.0), 1.5));
    p.step(&mut log);
    assert_eq!(log.take().len(), 1);
    w.set_title("renamed");
    assert_eq!(p.title(w.id()).as_deref(), Some("renamed"));
    let id = w.id();
    drop(w); // closes it
    assert_eq!(p.title(id), None);
}

#[test]
fn a_waker_fired_on_another_thread_wakes_the_loop_once() {
    let p = HeadlessPlatform::new();
    let mut log = Log::new(&p);
    let waker = p.waker();
    std::thread::spawn(move || {
        waker.wake();
        waker.wake();
    })
    .join()
    .unwrap();
    p.step(&mut log);
    p.step(&mut log);
    assert_eq!(log.take(), ["wake"]);
}

#[test]
fn run_returns_when_nothing_is_left_or_on_quit() {
    let p = HeadlessPlatform::new();
    let mut log = Log::new(&p);
    let w = p.open_window(&WindowOptions::new("w")).unwrap();
    let id = w.id().0;
    w.request_frame();
    log.again = Some((w, 2)); // each frame asks for the next, twice
    p.run(&mut log).unwrap();
    assert_eq!(log.take(), vec![format!("frame {id}"); 3]);

    p.push(WindowId(id), PlatformEvent::CloseRequested);
    p.quit();
    p.run(&mut log).unwrap();
    assert!(log.take().is_empty()); // quit: the loop does not turn again
}

#[test]
fn fit_caps_each_side_at_a_fraction_of_the_display() {
    let display = Size::new(1000.0, 500.0);
    assert_eq!(fit_size(Size::new(960.0, 600.0), display, 0.9), Size::new(900.0, 450.0));
    assert_eq!(fit_size(Size::new(960.0, 600.0), display, 0.0), Size::new(960.0, 600.0));
    assert_eq!(fit_size(Size::new(300.0, 200.0), display, 0.9), Size::new(300.0, 200.0));
}

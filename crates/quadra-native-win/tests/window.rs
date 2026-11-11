//! The Win32 platform on windows that are never shown: messages become events, timers and
//! wakes arrive, frames are given when asked for. Input is posted to the window's own queue;
//! nothing here touches the desktop the tests run on.

use std::time::Duration;

use quadra_native_win::WinPlatform;
use quadra_platform::{
    Key, MouseButton, Platform, PlatformEvent, PlatformHandler, PlatformWindow, PointerKind, TimerId,
    WindowHandle, WindowId, WindowOptions,
};
use quadra_prim::{Point, Size};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    IsWindow, PostMessageW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos, WM_CHAR, WM_CLOSE,
    WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_XBUTTONDOWN,
};

fn hwnd(window: &dyn PlatformWindow) -> HWND {
    let WindowHandle::Win32 { hwnd } = window.handle() else { panic!("not a Win32 window") };
    HWND(hwnd as *mut _)
}

fn post(window: &dyn PlatformWindow, msg: u32, wparam: usize, lparam: isize) {
    // SAFETY: posting a message to a window of this thread.
    unsafe { PostMessageW(Some(hwnd(window)), msg, WPARAM(wparam), LPARAM(lparam)).unwrap() };
}

fn resize(window: &dyn PlatformWindow, width: i32, height: i32) {
    let flags = SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE;
    // SAFETY: resizing a window of this thread.
    unsafe { SetWindowPos(hwnd(window), None, 0, 0, width, height, flags).unwrap() };
}

/// Writes down what it is told. The loop ends at a wake, or after `timers_left` timers.
struct Recorder {
    platform: WinPlatform,
    events: Vec<(WindowId, PlatformEvent)>,
    frames: Vec<WindowId>,
    timers: Vec<TimerId>,
    timers_left: u32,
    /// Handler methods on the stack, and the most there ever were.
    depth: u32,
    deepest: u32,
    /// Run once from inside the first text event.
    inside: Option<Box<dyn FnOnce()>>,
}

impl Recorder {
    fn new(platform: &WinPlatform) -> Recorder {
        Recorder {
            platform: platform.clone(),
            events: Vec::new(),
            frames: Vec::new(),
            timers: Vec::new(),
            timers_left: 0,
            depth: 0,
            deepest: 0,
            inside: None,
        }
    }

    /// The events without modifiers (a key may be held on the machine), without resizes, and
    /// without the leave the OS reports at once for a window the pointer is not over.
    fn input(&self) -> Vec<String> {
        let leave = |e: &PlatformEvent| matches!(e, PlatformEvent::Pointer { kind: PointerKind::Leave, .. });
        let plain =
            self.events.iter().filter(|(_, e)| !matches!(e, PlatformEvent::Resize { .. }) && !leave(e));
        plain.map(|(_, e)| format!("{e:?}").replace(", mods: Mods(0)", "")).collect()
    }
}

impl PlatformHandler for Recorder {
    fn event(&mut self, window: WindowId, event: PlatformEvent) {
        self.depth += 1;
        self.deepest = self.deepest.max(self.depth);
        if matches!(event, PlatformEvent::Text(_))
            && let Some(inside) = self.inside.take()
        {
            inside();
        }
        self.events.push((window, event));
        self.depth -= 1;
    }

    fn frame(&mut self, window: WindowId) {
        self.frames.push(window);
        self.platform.quit();
    }

    fn timer(&mut self, id: TimerId) {
        self.timers.push(id);
        self.timers_left -= 1;
        if self.timers_left == 0 {
            self.platform.quit();
        }
    }

    fn wake(&mut self) {
        self.platform.quit();
    }
}

fn hidden(platform: &WinPlatform, width: f32, height: f32) -> Box<dyn PlatformWindow> {
    platform.open_window(&WindowOptions::new("test").size(width, height).hidden()).unwrap()
}

#[test]
fn a_window_has_the_content_size_asked_for_and_is_closed_by_dropping_it() {
    let p = WinPlatform::new().unwrap();
    let w = hidden(&p, 320.0, 200.0);
    let (size, scale) = (w.size(), w.scale());
    assert!(scale >= 1.0);
    assert!((size.width - 320.0).abs() < 1.0 && (size.height - 200.0).abs() < 1.0, "{size:?}");
    w.set_title("renamed");
    let handle = hwnd(&*w);
    // SAFETY: asking whether a handle is a window has no preconditions.
    assert!(unsafe { IsWindow(Some(handle)) }.as_bool());
    drop(w);
    assert!(!unsafe { IsWindow(Some(handle)) }.as_bool());
}

#[test]
fn posted_input_arrives_as_events_in_logical_pixels() {
    let p = WinPlatform::new().unwrap();
    let w = hidden(&p, 320.0, 200.0);
    let at = |x: f32, y: f32| ((y * w.scale()) as isize) << 16 | (x * w.scale()) as isize;
    post(&*w, WM_MOUSEMOVE, 0, at(30.0, 40.0));
    post(&*w, WM_LBUTTONDOWN, 1, at(30.0, 40.0));
    post(&*w, WM_LBUTTONUP, 0, at(32.0, 40.0));
    post(&*w, WM_XBUTTONDOWN, 2 << 16, at(0.0, 0.0)); // the second extra button
    post(&*w, WM_KEYDOWN, 0x25, 0); // an arrow: a letter would also be translated into text
    post(&*w, WM_KEYDOWN, 0x25, 1 << 30); // held: a repeat
    post(&*w, WM_CHAR, 'a' as usize, 0);
    post(&*w, WM_CHAR, 0x08, 0); // backspace is a key, not text
    post(&*w, WM_CHAR, 0xD83D, 0); // a pair of surrogates is one character
    post(&*w, WM_CHAR, 0xDE00, 0);
    post(&*w, WM_CLOSE, 0, 0);
    p.waker().wake();
    let mut seen = Recorder::new(&p);
    p.run(&mut seen).unwrap();

    let pointer = |kind: PointerKind, button: MouseButton, x: f32, y: f32| {
        format!(
            "{:?}",
            PlatformEvent::Pointer { kind, button, at: Point::new(x, y), mods: Default::default() }
        )
        .replace(", mods: Mods(0)", "")
    };
    let key = |repeat: bool| {
        format!(
            "{:?}",
            PlatformEvent::Key { key: Key::Left, code: 0x25, mods: Default::default(), down: true, repeat }
        )
        .replace(", mods: Mods(0)", "")
    };
    assert_eq!(
        seen.input(),
        [
            pointer(PointerKind::Move, MouseButton::None, 30.0, 40.0),
            pointer(PointerKind::Down, MouseButton::Left, 30.0, 40.0),
            pointer(PointerKind::Up, MouseButton::Left, 32.0, 40.0),
            pointer(PointerKind::Down, MouseButton::Forward, 0.0, 0.0),
            key(false),
            key(true),
            "Text(\"a\")".to_string(),
            "Text(\"\u{1F600}\")".to_string(),
            "CloseRequested".to_string(),
        ]
    );
    assert!(seen.events.iter().all(|(id, _)| *id == w.id()));
    assert!(unsafe { IsWindow(Some(hwnd(&*w))) }.as_bool()); // a close request closes nothing
}

#[test]
fn a_resize_is_reported_with_the_new_size() {
    let p = WinPlatform::new().unwrap();
    let w = hidden(&p, 320.0, 200.0);
    let before = w.size();
    resize(&*w, 500, 400); // the outer size, in device pixels
    let after = w.size();
    assert!(after.width > before.width && after.height > before.height);
    p.waker().wake();
    let mut seen = Recorder::new(&p);
    p.run(&mut seen).unwrap(); // what happened before the loop ran is delivered when it starts
    let last = seen.events.iter().rev().find_map(|(_, e)| match e {
        PlatformEvent::Resize { size, scale } => Some((*size, *scale)),
        _ => None,
    });
    assert_eq!(last, Some((after, w.scale())));
}

#[test]
fn what_a_handler_sets_off_is_delivered_after_it_returns() {
    let p = WinPlatform::new().unwrap();
    let w = hidden(&p, 320.0, 200.0);
    let target = hwnd(&*w).0 as isize;
    let mut seen = Recorder::new(&p);
    seen.inside = Some(Box::new(move || {
        let flags = SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE;
        // SAFETY: resizing a window of this thread; the OS sends its size message at once.
        unsafe { SetWindowPos(HWND(target as *mut _), None, 0, 0, 640, 480, flags).unwrap() };
    }));
    post(&*w, WM_CHAR, 'a' as usize, 0);
    p.waker().wake();
    p.run(&mut seen).unwrap();
    assert_eq!(seen.deepest, 1); // the handler was never entered twice
    let size = Size::new(w.size().width, w.size().height);
    let position = |wanted: &dyn Fn(&PlatformEvent) -> bool| seen.events.iter().position(|(_, e)| wanted(e));
    let text = position(&|e| matches!(e, PlatformEvent::Text(_))).unwrap();
    let resized = position(&|e| matches!(e, PlatformEvent::Resize { size: s, .. } if *s == size)).unwrap();
    assert!(resized > text); // and the resize it caused came after it
}

#[test]
fn timers_fire_once_or_repeat_until_cancelled() {
    let p = WinPlatform::new().unwrap();
    let mut seen = Recorder::new(&p);
    let once = p.set_timer(Duration::from_millis(5), false);
    let every = p.set_timer(Duration::from_millis(10), true);
    let cancelled = p.set_timer(Duration::from_millis(1), true);
    p.cancel_timer(cancelled);
    seen.timers_left = 4;
    p.run(&mut seen).unwrap();
    assert_eq!(seen.timers.iter().filter(|&&t| t == once).count(), 1);
    assert_eq!(seen.timers.iter().filter(|&&t| t == every).count(), 3);
    p.cancel_timer(every);
}

#[test]
fn a_waker_wakes_the_loop_from_another_thread_and_a_frame_comes_when_asked() {
    let p = WinPlatform::new().unwrap();
    let mut seen = Recorder::new(&p);
    let waker = p.waker();
    let thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(20)); // the loop is asleep by now
        waker.wake();
    });
    p.run(&mut seen).unwrap(); // returns: the wake arrived
    thread.join().unwrap();
    assert!(seen.frames.is_empty()); // nobody asked for a frame

    let w = hidden(&p, 100.0, 100.0);
    w.request_frame();
    p.run(&mut seen).unwrap();
    assert_eq!(seen.frames, [w.id()]);
}

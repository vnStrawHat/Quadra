//! The Win32 platform (specs/10): the message loop, timers, wakes and frame pacing.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::ptr::NonNull;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use quadra_platform::{
    Clipboard, Platform, PlatformError, PlatformHandler, PlatformWindow, TimerId, Waker, WindowOptions,
};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetWindowLongPtrW, HWND_MESSAGE,
    KillTimer, MSG, MWMO_INPUTAVAILABLE, MsgWaitForMultipleObjectsEx, PM_REMOVE, PeekMessageW, PostMessageW,
    QS_ALLINPUT, SetTimer, SetWindowLongPtrW, TranslateMessage, WM_APP, WM_QUIT, WM_TIMER,
};
use windows::core::w;

use crate::clipboard::WinClipboard;
pub(crate) use crate::delivery::Delivery;
use crate::pacing::Pacing;
use crate::window::{self, WindowState};

const INFINITE: u32 = u32::MAX;
/// Posted to the message window: turn the loop (a quit), and a waker fired.
const WM_TURN: u32 = WM_APP;
const WM_WAKE: u32 = WM_APP + 1;

pub(crate) fn failed(e: windows::core::Error) -> PlatformError {
    PlatformError(e.message().to_string())
}

pub(crate) struct Shared {
    /// The invisible window that receives timers and posts.
    message_window: Cell<HWND>,
    pub windows: RefCell<Vec<Weak<WindowState>>>,
    pub(crate) queue: RefCell<VecDeque<Delivery>>,
    /// The handler `run` was given, while it runs.
    pub(crate) handler: Cell<Option<NonNull<dyn PlatformHandler>>>,
    /// A handler method is on the stack: what arrives now waits in the queue.
    pub(crate) busy: Cell<bool>,
    /// Timer id to whether it repeats.
    timers: RefCell<HashMap<usize, bool>>,
    next_id: Cell<u32>,
    quitting: Cell<bool>,
    wake_posted: Arc<AtomicBool>,
    pub pacing: Pacing,
}

impl Shared {
    pub fn next_id(&self) -> u32 {
        self.next_id.set(self.next_id.get() + 1);
        self.next_id.get()
    }

    fn on_message(&self, msg: u32, wparam: WPARAM) -> bool {
        match msg {
            WM_TURN => {}
            WM_WAKE => {
                self.wake_posted.store(false, Ordering::Release);
                self.deliver(Delivery::Wake);
            }
            WM_TIMER => {
                let repeat = self.timers.borrow().get(&wparam.0).copied();
                if let Some(repeat) = repeat {
                    if !repeat {
                        self.cancel_timer(wparam.0);
                    }
                    self.deliver(Delivery::Timer(TimerId(wparam.0 as u64)));
                }
            }
            _ => return false,
        }
        true
    }

    fn cancel_timer(&self, id: usize) {
        if self.timers.borrow_mut().remove(&id).is_some() {
            // SAFETY: the message window is alive as long as `self`.
            let _ = unsafe { KillTimer(Some(self.message_window.get()), id) };
        }
    }
}

impl Drop for Shared {
    fn drop(&mut self) {
        let hwnd = self.message_window.get();
        // SAFETY: the window is ours; its pointer to `self` is cleared first, so nothing sent
        // while it is destroyed reaches a `Shared` being dropped.
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(hwnd);
        }
    }
}

unsafe extern "system" fn message_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: the user data is null or points at the `Shared` that owns this window, which
    // clears it before it is dropped.
    let shared = unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Shared).as_ref() };
    if shared.is_some_and(|s| s.on_message(msg, wparam)) {
        return LRESULT(0);
    }
    // SAFETY: the arguments are the ones this procedure was called with.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// The Win32 platform. Clones share it; all of it lives on the thread that made it.
#[derive(Clone)]
pub struct WinPlatform(pub(crate) Rc<Shared>);

impl WinPlatform {
    pub fn new() -> Result<WinPlatform, PlatformError> {
        // SAFETY: plain calls with valid arguments; the window class lives as long as the
        // process. Setting the awareness fails when the process already chose one, which is
        // then what applies.
        let message_window = unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
            let class = w!("QuadraMessages");
            window::register_class(class, message_proc)?;
            window::create(class, w!(""), Default::default(), Some(HWND_MESSAGE), std::ptr::null())?
        };
        let shared = Rc::new(Shared {
            message_window: Cell::new(message_window),
            windows: RefCell::new(Vec::new()),
            queue: RefCell::new(VecDeque::new()),
            handler: Cell::new(None),
            busy: Cell::new(false),
            timers: RefCell::new(HashMap::new()),
            next_id: Cell::new(0),
            quitting: Cell::new(false),
            wake_posted: Arc::new(AtomicBool::new(false)),
            pacing: Pacing::default(),
        });
        // SAFETY: the pointer stays valid until `Shared` is dropped, which clears it.
        unsafe { SetWindowLongPtrW(message_window, GWLP_USERDATA, Rc::as_ptr(&shared) as isize) };
        Ok(WinPlatform(shared))
    }

    /// One pass of the loop's frame pacing: frames for windows that asked and need not wait,
    /// then a wait for messages or for a swap chain that can take a frame.
    fn pace(&self) {
        let s = &self.0;
        s.windows.borrow_mut().retain(|w| w.strong_count() > 0);
        let asked: Vec<Rc<WindowState>> = s.windows.borrow().iter().filter_map(Weak::upgrade).collect();
        let (mut waiting, mut handles) = (Vec::new(), Vec::new());
        for window in asked.iter().filter(|w| w.frame_requested.get()) {
            match s.pacing.get(window.hwnd.get()) {
                Some(handle) if handles.len() < 63 => {
                    waiting.push(window);
                    handles.push(handle);
                }
                _ => window.frame_now(),
            }
        }
        if s.quitting.get() {
            return;
        }
        // A frame that asked for the next one, in a window that waits for nothing: no sleep.
        let again =
            asked.iter().any(|w| w.frame_requested.get() && !waiting.iter().any(|x| Rc::ptr_eq(x, w)));
        let timeout = if again { 0 } else { INFINITE };
        // SAFETY: the handles are swap chain waitables kept alive by their surfaces.
        let woke =
            unsafe { MsgWaitForMultipleObjectsEx(Some(&handles), timeout, QS_ALLINPUT, MWMO_INPUTAVAILABLE) };
        if let Some(window) = waiting.get(woke.0 as usize) {
            window.frame_now();
        }
    }
}

impl Platform for WinPlatform {
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
        let mut msg = MSG::default();
        while !s.quitting.get() {
            // SAFETY: `msg` is a valid message structure for the three calls.
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        return Ok(());
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            s.drain(); // what was queued before the loop had a handler
            if !s.quitting.get() {
                self.pace();
            }
        }
        Ok(())
    }

    fn quit(&self) {
        self.0.quitting.set(true);
        // SAFETY: the message window is alive as long as the platform.
        let _ = unsafe { PostMessageW(Some(self.0.message_window.get()), WM_TURN, WPARAM(0), LPARAM(0)) };
    }

    fn open_window(&self, options: &WindowOptions) -> Result<Box<dyn PlatformWindow>, PlatformError> {
        Ok(Box::new(window::open(&self.0, options)?))
    }

    fn waker(&self) -> Waker {
        let (hwnd, posted) = (self.0.message_window.get().0 as isize, self.0.wake_posted.clone());
        Waker::new(move || {
            if !posted.swap(true, Ordering::AcqRel) {
                // SAFETY: posting to a window handle is allowed from any thread; if the
                // window is gone the call fails and nothing is woken.
                let _ = unsafe { PostMessageW(Some(HWND(hwnd as *mut _)), WM_WAKE, WPARAM(0), LPARAM(0)) };
            }
        })
    }

    fn set_timer(&self, after: Duration, repeat: bool) -> TimerId {
        let id = self.0.next_id() as usize;
        self.0.timers.borrow_mut().insert(id, repeat);
        let ms = after.as_millis().clamp(1, u32::MAX as u128 - 1) as u32;
        // SAFETY: the message window is alive as long as the platform.
        unsafe { SetTimer(Some(self.0.message_window.get()), id, ms, None) };
        TimerId(id as u64)
    }

    fn cancel_timer(&self, id: TimerId) {
        self.0.cancel_timer(id.0 as usize);
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn clipboard(&self) -> Rc<dyn Clipboard> {
        Rc::new(WinClipboard(self.0.message_window.get().0 as isize))
    }
}

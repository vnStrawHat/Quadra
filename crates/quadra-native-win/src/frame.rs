//! A window that draws its own title bar (specs/10, "Windows with their own frame"): the
//! content covers where the caption was, the OS keeps the borders that resize, and the app
//! says which of its points drag the window and which is the maximize button, so that
//! dragging, snapping and the snap layouts stay the system's.

use quadra_platform::{HitRegion, MouseButton, PlatformEvent, PointerKind};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::ScreenToClient;
use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    TME_LEAVE, TME_NONCLIENT, TRACKMOUSEEVENT, TrackMouseEvent,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, HTCAPTION, HTCLIENT, HTLEFT, HTMAXBUTTON, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT,
    IsIconic, IsZoomed, SM_CXPADDEDBORDER, SM_CYSIZEFRAME, SW_MAXIMIZE, SW_MINIMIZE, SW_RESTORE,
    SetForegroundWindow, ShowWindow, WM_NCCALCSIZE, WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP,
    WM_NCMOUSEMOVE,
};

use crate::input::{current_mods, position};
use crate::window::WindowState;

/// Not among the window messages of the bindings: it is declared with the common controls.
const WM_NCMOUSELEAVE: u32 = 0x02A2;

impl WindowState {
    pub fn is_maximized(&self) -> bool {
        // SAFETY: asks about a window handle; one that is gone is not maximized.
        unsafe { IsZoomed(self.hwnd.get()) }.as_bool()
    }

    pub fn minimize(&self) {
        // SAFETY: a window that is gone refuses the call.
        let _ = unsafe { ShowWindow(self.hwnd.get(), SW_MINIMIZE) };
    }

    pub fn to_front(&self) {
        let hwnd = self.hwnd.get();
        // SAFETY: a window that is gone refuses the calls; the OS may refuse the foreground to
        // a process the user is not using, and then flashes the window's button instead.
        unsafe {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
        }
    }

    pub fn toggle_maximize(&self) {
        let to = if self.is_maximized() { SW_RESTORE } else { SW_MAXIMIZE };
        // SAFETY: a window that is gone refuses the call.
        let _ = unsafe { ShowWindow(self.hwnd.get(), to) };
    }

    /// The height of the border that resizes, in device pixels: the band at the top of the
    /// content that still resizes, and what a maximized window hangs over the screen by.
    fn border(&self, hwnd: HWND) -> i32 {
        // SAFETY: plain questions about a window that is alive.
        unsafe {
            let dpi = GetDpiForWindow(hwnd).max(1);
            GetSystemMetricsForDpi(SM_CYSIZEFRAME, dpi) + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi)
        }
    }

    /// A pointer event from a message of the frame, whose position is on the screen.
    fn frame_pointer(&self, hwnd: HWND, kind: PointerKind, button: MouseButton, lparam: LPARAM) {
        let (x, y) = position(lparam);
        let mut at = POINT { x, y };
        // SAFETY: the point is valid and the window alive.
        let _ = unsafe { ScreenToClient(hwnd, &mut at) };
        let (at, mods) = (self.point(at.x, at.y), current_mods());
        self.emit(PlatformEvent::Pointer { kind, button, at, mods });
    }

    /// The messages a frame of the app's own changes; `None` leaves one to the usual handling.
    ///
    /// # Safety
    /// The arguments must be those of a message sent to this state's window.
    pub(crate) unsafe fn frame_message(
        &self,
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<LRESULT> {
        let code = wparam.0 as u32;
        match msg {
            WM_NCCALCSIZE if wparam.0 != 0 => {
                // The usual client rect, with its top given back: the content covers the
                // caption. A maximized window hangs over the screen by its border: not that.
                // SAFETY: with a non-zero `wparam` the message carries the rects to size,
                // the first of which is the window's.
                unsafe {
                    let rect = lparam.0 as *mut RECT;
                    let top = (*rect).top;
                    DefWindowProcW(hwnd, msg, wparam, lparam);
                    (*rect).top = top + if self.is_maximized() { self.border(hwnd) } else { 0 };
                }
                Some(LRESULT(0))
            }
            WM_NCHITTEST => {
                // SAFETY: the arguments are those of this message.
                let hit = unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }.0 as u32;
                let (x, y) = position(lparam);
                let mut at = POINT { x, y };
                // SAFETY: the point is valid and the window alive.
                let _ = unsafe { ScreenToClient(hwnd, &mut at) };
                // The band at the top that the caption took still resizes, and makes the
                // corners with the side borders.
                let top = self.resizable && !self.is_maximized() && at.y < self.border(hwnd);
                let hit = match hit {
                    HTLEFT if top => HTTOPLEFT,
                    HTRIGHT if top => HTTOPRIGHT,
                    HTCLIENT if top => HTTOP,
                    HTCLIENT => match self.shared.hit_test(self.id, self.point(at.x, at.y)) {
                        HitRegion::Client => HTCLIENT,
                        HitRegion::Caption => HTCAPTION,
                        HitRegion::Maximize => HTMAXBUTTON,
                    },
                    border => border,
                };
                Some(LRESULT(hit as isize))
            }
            // Over the bar and the maximize button the OS sends the frame's messages: the app
            // is told of the pointer all the same, so what is under it shows it.
            WM_NCMOUSEMOVE if code == HTMAXBUTTON || code == HTCAPTION => {
                if !self.tracking_leave.replace(true) {
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE | TME_NONCLIENT,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    // SAFETY: the structure is complete and the window is alive.
                    let _ = unsafe { TrackMouseEvent(&mut track) };
                }
                self.frame_pointer(hwnd, PointerKind::Move, MouseButton::None, lparam);
                None
            }
            // On a border, or gone: the pointer is over nothing of the app's.
            WM_NCMOUSEMOVE | WM_NCMOUSELEAVE => {
                if msg == WM_NCMOUSELEAVE {
                    self.tracking_leave.set(false);
                }
                self.frame_pointer(hwnd, PointerKind::Leave, MouseButton::None, lparam);
                None
            }
            // The app's maximize button is pressed and let go as any button of its own: no
            // button of the classic frame is drawn, and the click is the app's to act on.
            WM_NCLBUTTONDOWN if code == HTMAXBUTTON => {
                self.frame_down.set(true);
                self.frame_pointer(hwnd, PointerKind::Down, MouseButton::Left, lparam);
                Some(LRESULT(0))
            }
            WM_NCLBUTTONUP if self.frame_down.replace(false) => {
                self.frame_pointer(hwnd, PointerKind::Up, MouseButton::Left, lparam);
                Some(LRESULT(0))
            }
            _ => None,
        }
    }
}

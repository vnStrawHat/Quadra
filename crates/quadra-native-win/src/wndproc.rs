//! The window procedure: Win32 messages become platform events (behaviour/02).

use std::rc::Rc;

use quadra_platform::{MouseButton, PlatformEvent, PointerKind};
use quadra_prim::Point;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::Graphics::Gdi::{ScreenToClient, ValidateRect};
use windows::Win32::System::Threading::WaitForSingleObject;
use windows::Win32::UI::Input::Ime::ISC_SHOWUICOMPOSITIONWINDOW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::a11y::WM_SEMANTIC;
use crate::ime;
use crate::input::{button_of, char_text, current_mods, hi, key_from_vk, position, wheel_lines};
use crate::window::WindowState;

/// Not among the window messages of the bindings: it is declared with the common controls.
const WM_MOUSELEAVE: u32 = 0x02A3;

pub(crate) unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: `WM_NCCREATE` carries the pointer `window::open` gave: the window's state,
    // alive for the call that creates the window. After that the user data is that pointer
    // until the owner clears it before the state is freed, so a pointer read from it is
    // valid; holding a count for the call keeps the state alive if a handler drops the
    // window from inside it.
    unsafe {
        if msg == WM_NCCREATE {
            let state = (*(lparam.0 as *const CREATESTRUCTW)).lpCreateParams as *const WindowState;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
            (*state).hwnd.set(hwnd);
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;
        if state.is_null() {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        Rc::increment_strong_count(state);
        let state = Rc::from_raw(state);
        state.handle(hwnd, msg, wparam, lparam).unwrap_or_else(|| DefWindowProcW(hwnd, msg, wparam, lparam))
    }
}

impl WindowState {
    pub(crate) fn point(&self, x: i32, y: i32) -> Point {
        Point::new(x as f32 / self.scale.get(), y as f32 / self.scale.get())
    }

    fn pointer(&self, kind: PointerKind, button: MouseButton, lparam: LPARAM) {
        let (x, y) = position(lparam);
        self.emit(PlatformEvent::Pointer { kind, button, at: self.point(x, y), mods: current_mods() });
    }

    fn resized(&self) {
        self.emit(PlatformEvent::Resize { size: self.logical_size(), scale: self.scale.get() });
    }

    /// Inside the OS's move and resize loop ours does not turn, so frames are drawn from
    /// here, paced the same way: after the swap chain can take one, then waiting for the
    /// composition that shows it, so the content stays in step with the window's edge.
    fn frame_if_modal(&self, hwnd: HWND) {
        if self.in_size_move.get() && self.frame_requested.get() {
            if let Some(waitable) = self.shared.pacing.get(hwnd) {
                // SAFETY: the handle belongs to the window's swap chain, which is alive.
                unsafe { WaitForSingleObject(waitable, 100) };
            }
            self.frame_now();
            // SAFETY: no arguments; a failure only means no composition to wait for.
            let _ = unsafe { DwmFlush() };
        }
    }

    /// Handles a message, or returns `None` to leave it to the default procedure.
    ///
    /// # Safety
    /// The arguments must be those of a message sent to this state's window.
    unsafe fn handle(&self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
        let done = Some(LRESULT(0));
        if self.custom_frame {
            // SAFETY: the arguments are those this method was called with.
            let framed = unsafe { self.frame_message(hwnd, msg, wparam, lparam) };
            if framed.is_some() {
                return framed;
            }
        }
        match msg {
            WM_SIZE => {
                self.size.set(((lparam.0 & 0xFFFF) as i32, ((lparam.0 >> 16) & 0xFFFF) as i32));
                self.resized();
                self.frame_if_modal(hwnd);
                done
            }
            WM_PAINT => {
                // SAFETY: the window is alive: it is receiving this message.
                let _ = unsafe { ValidateRect(Some(hwnd), None) };
                // In the resize loop WM_SIZE has drawn this size: draw only what is pending.
                if !self.in_size_move.get() {
                    self.frame_requested.set(true);
                }
                self.frame_if_modal(hwnd);
                done
            }
            WM_ERASEBKGND => Some(LRESULT(1)),
            WM_ENTERSIZEMOVE | WM_EXITSIZEMOVE => {
                self.in_size_move.set(msg == WM_ENTERSIZEMOVE);
                None
            }
            WM_DPICHANGED => {
                self.scale.set(hi(wparam.0 as isize) as f32 / 96.0);
                // SAFETY: with this message `lparam` points at the rectangle the OS suggests.
                unsafe {
                    let r = *(lparam.0 as *const RECT);
                    let flags = SWP_NOZORDER | SWP_NOACTIVATE;
                    let _ =
                        SetWindowPos(hwnd, None, r.left, r.top, r.right - r.left, r.bottom - r.top, flags);
                }
                self.resized();
                done
            }
            WM_CLOSE => {
                self.emit(PlatformEvent::CloseRequested);
                done
            }
            WM_NCDESTROY => {
                // Destroyed by someone else: the owner's drop finds nothing left to destroy.
                self.hwnd.set(HWND::default());
                // SAFETY: the window is alive until this message returns.
                unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
                None
            }
            WM_SETFOCUS | WM_KILLFOCUS => {
                if let Some(access) = self.access.get() {
                    access.window_focused(msg == WM_SETFOCUS);
                }
                self.emit(PlatformEvent::Focus(msg == WM_SETFOCUS));
                done
            }
            // Assistive technology asks for the window's tree. The first time, the app is
            // told to start sending it; until it has, the adapter answers with an empty one.
            WM_GETOBJECT => {
                let (result, first) = self.access.get()?.object(wparam, lparam);
                if first {
                    self.emit(PlatformEvent::SemanticsWanted);
                }
                result
            }
            WM_SEMANTIC => {
                let asked: Vec<_> = self.access.get()?.requests().collect();
                for (node, action) in asked {
                    self.emit(PlatformEvent::Semantic { node, action });
                }
                done
            }
            WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP => {
                let down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
                self.emit(PlatformEvent::Key {
                    key: key_from_vk(wparam.0 as u32),
                    code: wparam.0 as u32,
                    mods: current_mods(),
                    down,
                    repeat: down && lparam.0 & (1 << 30) != 0,
                });
                // The default procedure keeps Alt+F4 and Alt+Space working.
                if msg == WM_SYSKEYDOWN || msg == WM_SYSKEYUP { None } else { done }
            }
            WM_CHAR => {
                let mut pending = self.high_surrogate.get();
                let text = char_text(wparam.0 as u16, &mut pending);
                self.high_surrogate.set(pending);
                if let Some(text) = text {
                    self.emit(PlatformEvent::Text(text));
                }
                done
            }
            WM_MOUSEMOVE => {
                if !self.tracking_leave.replace(true) {
                    let mut track = TRACKMOUSEEVENT {
                        cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        dwHoverTime: 0,
                    };
                    // SAFETY: the structure is complete and the window is alive.
                    let _ = unsafe { TrackMouseEvent(&mut track) };
                }
                self.pointer(PointerKind::Move, MouseButton::None, lparam);
                done
            }
            // Over the content the cursor is the app's; over the frame, the system's.
            WM_SETCURSOR if (lparam.0 & 0xFFFF) as u32 == HTCLIENT => {
                // SAFETY: the handle is one of the system's own cursors, or null for none.
                unsafe { SetCursor(Some(self.cursor.get())) };
                Some(LRESULT(1))
            }
            WM_MOUSELEAVE => {
                self.tracking_leave.set(false);
                self.pointer(PointerKind::Leave, MouseButton::None, LPARAM(0));
                done
            }
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN => {
                // SAFETY: the window is alive; capture keeps a drag coming here outside it.
                unsafe { SetCapture(hwnd) };
                self.pointer(PointerKind::Down, button_of(msg, wparam), lparam);
                Some(LRESULT((msg == WM_XBUTTONDOWN) as isize))
            }
            WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP | WM_XBUTTONUP => {
                // SAFETY: no arguments; it fails when nothing was captured.
                let _ = unsafe { ReleaseCapture() };
                self.pointer(PointerKind::Up, button_of(msg, wparam), lparam);
                Some(LRESULT((msg == WM_XBUTTONUP) as isize))
            }
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL => {
                let (x, y) = position(lparam);
                let mut at = POINT { x, y };
                // SAFETY: the point is valid; wheel positions arrive in screen coordinates.
                let _ = unsafe { ScreenToClient(hwnd, &mut at) };
                let lines = wheel_lines(wparam);
                let delta =
                    if msg == WM_MOUSEWHEEL { Point::new(0.0, lines) } else { Point::new(lines, 0.0) };
                self.emit(PlatformEvent::Wheel {
                    delta,
                    precise: false,
                    at: self.point(at.x, at.y),
                    mods: current_mods(),
                });
                done
            }
            // The app draws the text being composed: the OS's own composition window stays
            // hidden, its candidate window does not.
            WM_IME_SETCONTEXT => {
                let ui = LPARAM(lparam.0 & !(ISC_SHOWUICOMPOSITIONWINDOW as isize));
                // SAFETY: the arguments are those of this message, less one flag.
                Some(unsafe { DefWindowProcW(hwnd, msg, wparam, ui) })
            }
            WM_IME_STARTCOMPOSITION => done,
            WM_IME_COMPOSITION => {
                let caret = self.caret.get().unwrap_or_default();
                for event in ime::composition(hwnd, lparam.0 as u32, caret) {
                    self.emit(event);
                }
                done
            }
            WM_IME_ENDCOMPOSITION => {
                self.emit(PlatformEvent::Composition { text: String::new(), caret: 0 });
                done
            }
            _ => None,
        }
    }
}

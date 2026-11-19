//! Where a window is (specs/15, "Window state"): read as the OS keeps it, and put back only
//! where a display is; and how big its frame is for a content size.

use quadra_platform::Placement;
use quadra_prim::Size;
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONULL, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint,
    MonitorFromRect,
};
use windows::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetWindowLongW, GetWindowPlacement, SW_HIDE, SW_SHOWMAXIMIZED, SW_SHOWNORMAL,
    SetWindowPlacement, WINDOW_EX_STYLE, WINDOW_STYLE, WINDOWPLACEMENT, WPF_RESTORETOMAXIMIZED,
};

use crate::window::WindowState;

/// The work area of the primary display, in device pixels.
pub(crate) fn primary_display() -> RECT {
    let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
    // SAFETY: `info` is a valid structure with its size set.
    let _ =
        unsafe { GetMonitorInfoW(MonitorFromPoint(POINT::default(), MONITOR_DEFAULTTOPRIMARY), &mut info) };
    info.rcWork
}

fn empty() -> WINDOWPLACEMENT {
    WINDOWPLACEMENT { length: size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() }
}

impl WindowState {
    /// The window's frame, in device pixels, around `content` in logical pixels at `dpi`:
    /// the OS's frame, or with a frame of the app's own only the borders that resize, as the
    /// content covers the caption (`frame.rs`). Rounded up: the content is never smaller.
    pub fn frame_size(&self, hwnd: HWND, content: Size, dpi: u32) -> (i32, i32) {
        let scale = dpi as f32 / 96.0;
        let (right, bottom) = ((content.width * scale).ceil() as i32, (content.height * scale).ceil() as i32);
        let mut frame = RECT { left: 0, top: 0, right, bottom };
        // SAFETY: a question about a window, and a rectangle that is valid to adjust.
        unsafe {
            let style = WINDOW_STYLE(GetWindowLongW(hwnd, GWL_STYLE) as u32);
            let _ = AdjustWindowRectExForDpi(&mut frame, style, false, WINDOW_EX_STYLE(0), dpi);
        }
        if self.custom_frame {
            frame.top = 0;
        }
        (frame.right - frame.left, frame.bottom - frame.top)
    }

    /// The least frame the window is held to: its minimum content size at its DPI of now.
    pub fn min_frame(&self, hwnd: HWND) -> (i32, i32) {
        // SAFETY: a question about a window; one that is gone answers 0.
        let dpi = unsafe { GetDpiForWindow(hwnd) }.max(1);
        self.frame_size(hwnd, self.min_size, dpi)
    }

    /// The frame the window has when it is not maximized or minimized, and whether it is
    /// maximized, or would be again if it is minimized.
    pub fn placement(&self) -> Placement {
        let mut at = empty();
        // SAFETY: `at` is a valid structure with its size set; a window that is gone refuses.
        let _ = unsafe { GetWindowPlacement(self.hwnd.get(), &mut at) };
        let r = at.rcNormalPosition;
        let maximized = at.showCmd == SW_SHOWMAXIMIZED.0 as u32 || at.flags.contains(WPF_RESTORETOMAXIMIZED);
        Placement { x: r.left, y: r.top, width: r.right - r.left, height: r.bottom - r.top, maximized }
    }

    /// Puts the window at `place`, shown as it was when `visible`: false, and nothing done,
    /// where `place` is on no display. A place smaller than the window's minimum is grown.
    pub fn place(&self, place: Placement, visible: bool) -> bool {
        if place.width <= 0 || place.height <= 0 {
            return false;
        }
        let (min_width, min_height) = self.min_frame(self.hwnd.get());
        let (x, y) = (place.x, place.y);
        let (width, height) = (place.width.max(min_width), place.height.max(min_height));
        let r = RECT { left: x, top: y, right: x + width, bottom: y + height };
        // SAFETY: a question about a rectangle.
        if unsafe { MonitorFromRect(&r, MONITOR_DEFAULTTONULL) }.is_invalid() {
            return false;
        }
        let show = match (visible, place.maximized) {
            (false, _) => SW_HIDE,
            (true, true) => SW_SHOWMAXIMIZED,
            (true, false) => SW_SHOWNORMAL,
        };
        let at = WINDOWPLACEMENT { showCmd: show.0 as u32, rcNormalPosition: r, ..empty() };
        // SAFETY: `at` is complete; a window that is gone refuses the call.
        unsafe { SetWindowPlacement(self.hwnd.get(), &at) }.is_ok()
    }
}

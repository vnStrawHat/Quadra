//! Where a window is (specs/15, "Window state"): read as the OS keeps it, and put back only
//! where a display is.

use quadra_platform::Placement;
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONULL, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint,
    MonitorFromRect,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowPlacement, SW_HIDE, SW_SHOWMAXIMIZED, SW_SHOWNORMAL, SetWindowPlacement, WINDOWPLACEMENT,
    WPF_RESTORETOMAXIMIZED,
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
    /// where `place` is on no display.
    pub fn place(&self, place: Placement, visible: bool) -> bool {
        let (x, y) = (place.x, place.y);
        let r = RECT { left: x, top: y, right: x + place.width, bottom: y + place.height };
        // SAFETY: a question about a rectangle.
        if place.width <= 0
            || place.height <= 0
            || unsafe { MonitorFromRect(&r, MONITOR_DEFAULTTONULL) }.is_invalid()
        {
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

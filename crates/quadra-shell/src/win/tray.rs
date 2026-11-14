//! The tray (specs/16, "Tray and notifications"): an icon in the notification area with a
//! hidden window of its own, whose clicks and menu come back to the app through an inbox,
//! and notifications from it. A manual check: no test adds a real icon.

use quadra_async::Sender;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{CreateBitmap, DeleteObject};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NIM_SETVERSION, NIN_SELECT, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon,
    DestroyMenu, DestroyWindow, GWLP_USERDATA, GetCursorPos, GetWindowLongPtrW, HICON, HMENU, ICONINFO,
    IDI_APPLICATION, LoadIconW, MF_STRING, PostMessageW, RegisterClassExW, SetForegroundWindow,
    SetWindowLongPtrW, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, WINDOW_EX_STYLE, WM_APP,
    WM_CONTEXTMENU, WM_LBUTTONDBLCLK, WM_NULL, WNDCLASSEXW, WS_POPUP,
};
use windows::core::{HSTRING, w};

use crate::{TrayEvent, TrayImage};

const WM_TRAY: u32 = WM_APP + 7;
/// The icon chosen by the keyboard: `NIN_SELECT | NINF_KEY`.
const NIN_KEYSELECT: u32 = NIN_SELECT | 1;

struct State {
    events: Sender<TrayEvent>,
    menu: HMENU,
}

/// Copies `text` into a fixed field of the structure, cut to fit with its terminating zero.
fn put(field: &mut [u16], text: &str) {
    let units: Vec<u16> = text.encode_utf16().take(field.len() - 1).collect();
    field[..units.len()].copy_from_slice(&units);
    field[units.len()] = 0;
}

unsafe extern "system" fn tray_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: the window's data is the `State` its `WinTray` owns, cleared before it is freed.
    unsafe {
        let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const State;
        if msg != WM_TRAY || state.is_null() {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let state = &*state;
        match (lparam.0 & 0xFFFF) as u32 {
            NIN_SELECT | NIN_KEYSELECT => drop(state.events.push(TrayEvent::Click)),
            WM_LBUTTONDBLCLK => drop(state.events.push(TrayEvent::DoubleClick)),
            WM_CONTEXTMENU if !state.menu.is_invalid() => {
                let mut at = POINT::default();
                let _ = GetCursorPos(&mut at);
                // The menu closes when the user clicks elsewhere only if its window is in front.
                let _ = SetForegroundWindow(hwnd);
                let chosen =
                    TrackPopupMenu(state.menu, TPM_RETURNCMD | TPM_RIGHTBUTTON, at.x, at.y, None, hwnd, None);
                let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
                if chosen.0 > 0 {
                    let _ = state.events.push(TrayEvent::Item(chosen.0 as usize - 1));
                }
            }
            _ => {}
        }
        LRESULT(0)
    }
}

/// An icon of the app's, from its pixels: straight RGBA, `width` by `height`.
fn icon_of(image: &TrayImage) -> Option<HICON> {
    let bgra: Vec<u8> = image.rgba.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();
    let (w, h) = (image.width as i32, image.height as i32);
    // SAFETY: the bitmaps are made from buffers of their size and deleted once the icon has
    // copied them.
    unsafe {
        let color = CreateBitmap(w, h, 1, 32, Some(bgra.as_ptr().cast()));
        let mask = CreateBitmap(w, h, 1, 1, None);
        let info = ICONINFO { fIcon: true.into(), hbmMask: mask, hbmColor: color, ..Default::default() };
        let icon = CreateIconIndirect(&info).ok();
        let _ = (DeleteObject(color.into()), DeleteObject(mask.into()));
        icon
    }
}

/// An icon in the notification area. Dropping it takes it away.
pub struct WinTray {
    data: NOTIFYICONDATAW,
    state: *mut State,
    owned_icon: bool,
}

impl WinTray {
    pub(crate) fn add(
        image: Option<&TrayImage>,
        tip: &str,
        menu: &[&str],
        events: Sender<TrayEvent>,
    ) -> Option<WinTray> {
        // SAFETY: plain calls; the window's data is set once the window exists, and the state
        // is freed only after the window is gone (`Drop`).
        unsafe {
            let instance = GetModuleHandleW(None).ok()?;
            let class = w!("QuadraTray");
            let window_class = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(tray_proc),
                hInstance: instance.into(),
                lpszClassName: class,
                ..Default::default()
            };
            RegisterClassExW(&window_class);
            let ex = WINDOW_EX_STYLE(0);
            let hwnd = CreateWindowExW(
                ex,
                class,
                w!(""),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(instance.into()),
                None,
            )
            .ok()?;
            let items = CreatePopupMenu().ok()?;
            for (i, item) in menu.iter().enumerate() {
                let _ = AppendMenuW(items, MF_STRING, i + 1, &HSTRING::from(*item));
            }
            let state = Box::into_raw(Box::new(State { events, menu: items }));
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);
            let made = image.and_then(icon_of);
            let icon = made.or_else(|| LoadIconW(None, IDI_APPLICATION).ok())?;
            let mut data = NOTIFYICONDATAW {
                cbSize: size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP | NIF_SHOWTIP,
                uCallbackMessage: WM_TRAY,
                hIcon: icon,
                ..Default::default()
            };
            put(&mut data.szTip, tip);
            let tray = WinTray { data, state, owned_icon: made.is_some() };
            if !Shell_NotifyIconW(NIM_ADD, &tray.data).as_bool() {
                return None; // dropped: the window and the state go with it
            }
            let mut version = tray.data;
            version.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            let _ = Shell_NotifyIconW(NIM_SETVERSION, &version);
            Some(tray)
        }
    }

    pub(crate) fn notify(&self, title: &str, text: &str) {
        let mut data = self.data;
        data.uFlags = NIF_INFO;
        put(&mut data.szInfoTitle, title);
        put(&mut data.szInfo, text);
        data.dwInfoFlags = NIIF_INFO;
        // SAFETY: the structure is complete and the icon is ours.
        let _ = unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) };
    }

    pub(crate) fn set_tooltip(&mut self, tip: &str) {
        put(&mut self.data.szTip, tip);
        self.data.uFlags = NIF_TIP | NIF_SHOWTIP;
        // SAFETY: as in `notify`.
        let _ = unsafe { Shell_NotifyIconW(NIM_MODIFY, &self.data) };
    }
}

impl Drop for WinTray {
    fn drop(&mut self) {
        // SAFETY: the icon, the window and the state are ours; the window's pointer to the
        // state is cleared before the state is freed.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.data);
            SetWindowLongPtrW(self.data.hWnd, GWLP_USERDATA, 0);
            let _ = DestroyWindow(self.data.hWnd);
            let state = Box::from_raw(self.state);
            let _ = DestroyMenu(state.menu);
            if self.owned_icon {
                let _ = DestroyIcon(self.data.hIcon);
            }
        }
    }
}

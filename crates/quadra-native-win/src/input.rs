//! Keyboard and mouse: from Win32 messages to platform events.

use quadra_platform::{Cursor, Key, Mods, MouseButton};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
use windows::Win32::UI::WindowsAndMessaging::{
    HCURSOR, IDC_APPSTARTING, IDC_ARROW, IDC_CROSS, IDC_HAND, IDC_HELP, IDC_IBEAM, IDC_NO, IDC_SIZEALL,
    IDC_SIZENESW, IDC_SIZENS, IDC_SIZENWSE, IDC_SIZEWE, IDC_WAIT, LoadCursorW, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_MBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
};

pub(crate) fn lo(v: isize) -> i32 {
    (v & 0xFFFF) as u16 as i16 as i32
}

pub(crate) fn hi(v: isize) -> i32 {
    ((v >> 16) & 0xFFFF) as u16 as i16 as i32
}

/// The modifiers held as of the message being handled.
pub(crate) fn current_mods() -> Mods {
    // SAFETY: GetKeyState reads the calling thread's key state and has no preconditions.
    let down = |vk: i32| unsafe { GetKeyState(vk) } < 0;
    let mut mods = Mods::NONE;
    for (vk, m) in
        [(0x10, Mods::SHIFT), (0x11, Mods::CTRL), (0x12, Mods::ALT), (0x5B, Mods::SUPER), (0x5C, Mods::SUPER)]
    {
        if down(vk) {
            mods |= m;
        }
    }
    mods
}

pub(crate) fn key_from_vk(vk: u32) -> Key {
    match vk {
        0x41..=0x5A => Key::letter(vk - 0x41),
        0x30..=0x39 => Key::digit(vk - 0x30),
        0x70..=0x7B => Key::function(vk - 0x70),
        0x0D => Key::Enter,
        0x1B => Key::Escape,
        0x08 => Key::Backspace,
        0x09 => Key::Tab,
        0x20 => Key::Space,
        0x2E => Key::Delete,
        0x2D => Key::Insert,
        0x24 => Key::Home,
        0x23 => Key::End,
        0x21 => Key::PageUp,
        0x22 => Key::PageDown,
        0x25 => Key::Left,
        0x27 => Key::Right,
        0x26 => Key::Up,
        0x28 => Key::Down,
        0x10 => Key::Shift,
        0x11 => Key::Ctrl,
        0x12 => Key::Alt,
        0x5B | 0x5C => Key::Super,
        0x14 => Key::CapsLock,
        0x5D => Key::Menu,
        0xBD => Key::Minus,
        0xBB => Key::Equal,
        0xDB => Key::BracketLeft,
        0xDD => Key::BracketRight,
        0xDC => Key::Backslash,
        0xBA => Key::Semicolon,
        0xDE => Key::Quote,
        0xBC => Key::Comma,
        0xBE => Key::Period,
        0xBF => Key::Slash,
        0xC0 => Key::Backquote,
        _ => Key::Unknown,
    }
}

/// The button of a button message; the extra buttons say which in `wparam`.
pub(crate) fn button_of(msg: u32, wparam: WPARAM) -> MouseButton {
    match msg {
        WM_LBUTTONDOWN | WM_LBUTTONUP => MouseButton::Left,
        WM_RBUTTONDOWN | WM_RBUTTONUP => MouseButton::Right,
        WM_MBUTTONDOWN | WM_MBUTTONUP => MouseButton::Middle,
        _ if (wparam.0 >> 16) & 0xFFFF == 1 => MouseButton::Back,
        _ => MouseButton::Forward,
    }
}

/// The wheel's turn in lines: 120 units a notch.
pub(crate) fn wheel_lines(wparam: WPARAM) -> f32 {
    hi(wparam.0 as isize) as f32 / 120.0
}

/// A position packed in a message, in device pixels.
pub(crate) fn position(lparam: LPARAM) -> (i32, i32) {
    (lo(lparam.0), hi(lparam.0))
}

/// Joins the UTF-16 units of `WM_CHAR` into text: a high surrogate waits in `pending` for its
/// low half; control characters and halves without their pair give nothing.
pub(crate) fn char_text(unit: u16, pending: &mut u16) -> Option<String> {
    match unit {
        0xD800..=0xDBFF => {
            *pending = unit;
            None
        }
        0xDC00..=0xDFFF => {
            let high = std::mem::take(pending);
            (high != 0).then(|| String::from_utf16_lossy(&[high, unit]))
        }
        0..0x20 | 0x7F => None,
        _ => Some(String::from_utf16_lossy(&[unit])),
    }
}

/// One of the cursors the system has for everyone.
pub(crate) fn system_cursor(cursor: Cursor) -> HCURSOR {
    let id = match cursor {
        Cursor::Arrow => IDC_ARROW,
        Cursor::Text => IDC_IBEAM,
        Cursor::Pointer => IDC_HAND,
        Cursor::Crosshair => IDC_CROSS,
        Cursor::Move => IDC_SIZEALL,
        Cursor::NotAllowed => IDC_NO,
        Cursor::ResizeHorizontal => IDC_SIZEWE,
        Cursor::ResizeVertical => IDC_SIZENS,
        Cursor::ResizeNwse => IDC_SIZENWSE,
        Cursor::ResizeNesw => IDC_SIZENESW,
        Cursor::Wait => IDC_WAIT,
        Cursor::Progress => IDC_APPSTARTING,
        Cursor::Help => IDC_HELP,
    };
    // SAFETY: loading a cursor of the system by its id; they are shared and never freed.
    unsafe { LoadCursorW(None, id) }.unwrap_or_default()
}

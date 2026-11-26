//! Keys and modifiers as AppKit names them: a key by its code, which is of its place on the
//! keyboard, and the modifiers of an event.

use objc2_app_kit::NSEventModifierFlags;
use quadra_platform::{Key, Mods};

/// The modifiers of an event. Command is the Super key, Option the Alt key.
pub(crate) fn mods(flags: NSEventModifierFlags) -> Mods {
    let mut mods = Mods::NONE;
    for (flag, held) in [
        (NSEventModifierFlags::Shift, Mods::SHIFT),
        (NSEventModifierFlags::Control, Mods::CTRL),
        (NSEventModifierFlags::Option, Mods::ALT),
        (NSEventModifierFlags::Command, Mods::SUPER),
    ] {
        if flags.contains(flag) {
            mods |= held;
        }
    }
    mods
}

/// The flag a modifier key holds while it is down; none for another key.
pub(crate) fn flag_of(key: Key) -> Option<NSEventModifierFlags> {
    Some(match key {
        Key::Shift => NSEventModifierFlags::Shift,
        Key::Ctrl => NSEventModifierFlags::Control,
        Key::Alt => NSEventModifierFlags::Option,
        Key::Super => NSEventModifierFlags::Command,
        Key::CapsLock => NSEventModifierFlags::CapsLock,
        _ => return None,
    })
}

/// The key of a virtual key code.
#[rustfmt::skip]
pub(crate) fn key(code: u16) -> Key {
    const LETTERS: [(u16, u32); 26] = [
        (0x00, 0), (0x0B, 1), (0x08, 2), (0x02, 3), (0x0E, 4), (0x03, 5), (0x05, 6), (0x04, 7), (0x22, 8),
        (0x26, 9), (0x28, 10), (0x25, 11), (0x2E, 12), (0x2D, 13), (0x1F, 14), (0x23, 15), (0x0C, 16),
        (0x0F, 17), (0x01, 18), (0x11, 19), (0x20, 20), (0x09, 21), (0x0D, 22), (0x07, 23), (0x10, 24),
        (0x06, 25),
    ];
    const DIGITS: [u16; 10] = [0x1D, 0x12, 0x13, 0x14, 0x15, 0x17, 0x16, 0x1A, 0x1C, 0x19];
    const FUNCTIONS: [u16; 12] = [0x7A, 0x78, 0x63, 0x76, 0x60, 0x61, 0x62, 0x64, 0x65, 0x6D, 0x67, 0x6F];
    if let Some(&(_, letter)) = LETTERS.iter().find(|(it, _)| *it == code) {
        return Key::letter(letter);
    }
    if let Some(digit) = DIGITS.iter().position(|&it| it == code) {
        return Key::digit(digit as u32);
    }
    if let Some(function) = FUNCTIONS.iter().position(|&it| it == code) {
        return Key::function(function as u32);
    }
    match code {
        0x24 | 0x4C => Key::Enter,
        0x35 => Key::Escape,
        0x33 => Key::Backspace,
        0x30 => Key::Tab,
        0x31 => Key::Space,
        0x75 => Key::Delete,
        0x72 => Key::Insert,
        0x73 => Key::Home,
        0x77 => Key::End,
        0x74 => Key::PageUp,
        0x79 => Key::PageDown,
        0x7B => Key::Left,
        0x7C => Key::Right,
        0x7E => Key::Up,
        0x7D => Key::Down,
        0x38 | 0x3C => Key::Shift,
        0x3B | 0x3E => Key::Ctrl,
        0x3A | 0x3D => Key::Alt,
        0x37 | 0x36 => Key::Super,
        0x39 => Key::CapsLock,
        0x6E => Key::Menu,
        0x1B => Key::Minus,
        0x18 => Key::Equal,
        0x21 => Key::BracketLeft,
        0x1E => Key::BracketRight,
        0x2A => Key::Backslash,
        0x29 => Key::Semicolon,
        0x27 => Key::Quote,
        0x2B => Key::Comma,
        0x2F => Key::Period,
        0x2C => Key::Slash,
        0x32 => Key::Backquote,
        _ => Key::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_named_by_their_codes() {
        assert_eq!((key(0x00), key(0x06), key(0x1D), key(0x19)), (Key::A, Key::Z, Key::Digit0, Key::Digit9));
        assert_eq!((key(0x7A), key(0x6F), key(0x24), key(0x7E)), (Key::F1, Key::F12, Key::Enter, Key::Up));
        assert_eq!(key(0xFF), Key::Unknown);
        let held = mods(NSEventModifierFlags::Shift | NSEventModifierFlags::Command);
        assert!(held.contains(Mods::SHIFT | Mods::SUPER) && !held.contains(Mods::CTRL));
    }
}

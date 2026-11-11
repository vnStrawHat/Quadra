//! Keys, named independently of any operating system.

/// A key by its position's usual meaning. Text comes as [`crate::PlatformEvent::Text`], never
/// from keys.
#[rustfmt::skip]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Key {
    #[default]
    Unknown,
    A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    Enter, Escape, Backspace, Tab, Space, Delete, Insert,
    Home, End, PageUp, PageDown, Left, Right, Up, Down,
    Shift, Ctrl, Alt, Super, CapsLock, Menu,
    Minus, Equal, BracketLeft, BracketRight, Backslash, Semicolon, Quote, Comma, Period, Slash,
    Backquote,
}

impl Key {
    /// The letter key of an index from 0 (`A`) to 25 (`Z`).
    pub fn letter(index: u32) -> Key {
        Key::LETTERS.get(index as usize).copied().unwrap_or_default()
    }

    /// The digit key of 0 to 9.
    pub fn digit(index: u32) -> Key {
        Key::DIGITS.get(index as usize).copied().unwrap_or_default()
    }

    /// The function key of an index from 0 (`F1`) to 11 (`F12`).
    pub fn function(index: u32) -> Key {
        Key::FUNCTIONS.get(index as usize).copied().unwrap_or_default()
    }

    #[rustfmt::skip]
    const LETTERS: [Key; 26] = [
        Key::A, Key::B, Key::C, Key::D, Key::E, Key::F, Key::G, Key::H, Key::I, Key::J, Key::K, Key::L,
        Key::M, Key::N, Key::O, Key::P, Key::Q, Key::R, Key::S, Key::T, Key::U, Key::V, Key::W, Key::X,
        Key::Y, Key::Z,
    ];
    #[rustfmt::skip]
    const DIGITS: [Key; 10] = [
        Key::Digit0, Key::Digit1, Key::Digit2, Key::Digit3, Key::Digit4, Key::Digit5, Key::Digit6,
        Key::Digit7, Key::Digit8, Key::Digit9,
    ];
    #[rustfmt::skip]
    const FUNCTIONS: [Key; 12] = [
        Key::F1, Key::F2, Key::F3, Key::F4, Key::F5, Key::F6, Key::F7, Key::F8, Key::F9, Key::F10,
        Key::F11, Key::F12,
    ];
}

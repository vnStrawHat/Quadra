//! Keys to the program (specs/18, "Input"): what a platform key is to the engine's encoder,
//! and the terminal's own bindings (Tab, copy, paste), in the context `terminal`.

use oneterm_vt::input::{KeyEvent, KeyEventKind, KeyMods, KeySpec, NamedKey};
use quadra_platform::{Key, Mods};
use quadra_tree::{Action, Keymap};

/// The key context of the terminal.
pub const TERMINAL_CONTEXT: &str = "terminal";

/// What the terminal's bindings do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TermCommand {
    /// Tab, and with Shift: keys the window would take for the focus.
    Tab(bool),
    Copy,
    Paste,
    /// Through the history: a page, or to an end.
    Scroll(crate::Scroll),
}

impl Action for TermCommand {
    const NAME: &'static str = "terminal::Command";
}

/// Binds Tab and Shift+Tab to the program, and copy and paste to Ctrl+Shift+C and V (and
/// Ctrl+Insert, Shift+Insert), while a terminal has the focus. On a Mac Command with C and V
/// copy and paste too: Control is the key that is the program's there (`key_event`).
pub fn bind_keys(keymap: &mut Keymap) {
    let keys = [
        ("tab", TermCommand::Tab(false)),
        ("shift-tab", TermCommand::Tab(true)),
        ("ctrl-shift-c", TermCommand::Copy),
        ("ctrl-insert", TermCommand::Copy),
        ("ctrl-shift-v", TermCommand::Paste),
        ("shift-insert", TermCommand::Paste),
        ("shift-pageup", TermCommand::Scroll(crate::Scroll::PageUp)),
        ("shift-pagedown", TermCommand::Scroll(crate::Scroll::PageDown)),
        ("ctrl-shift-home", TermCommand::Scroll(crate::Scroll::Top)),
        ("ctrl-shift-end", TermCommand::Scroll(crate::Scroll::Bottom)),
    ];
    for (keys, command) in keys {
        keymap.bind_in(TERMINAL_CONTEXT, keys, command).expect("the terminal's bindings are well formed");
    }
    for (keys, command) in [("ctrl-c", TermCommand::Copy), ("ctrl-v", TermCommand::Paste)] {
        keymap.bind_in_on_mac(TERMINAL_CONTEXT, keys, command).expect("as above");
    }
}

/// The engine's key for a platform key with its modifiers, or `None` when typing gives it as
/// text: a printable key without Ctrl or Alt, or with both (AltGr).
///
/// On a Mac (`mac`; specs/23, "Keys"): the program's Control is the key the platform calls
/// `SUPER`, a key held with Command is the app's and never the program's, and Option with a
/// key that types gives what the layout says, as text.
pub(crate) fn key_event(key: Key, mods: Mods, repeat: bool, mac: bool) -> Option<KeyEvent> {
    let held = |it| mods.contains(it);
    let (ctrl, alt, super_key) = match mac {
        true if held(Mods::CTRL) => return None,
        true => (held(Mods::SUPER), held(Mods::ALT), false),
        false => (held(Mods::CTRL), held(Mods::ALT), held(Mods::SUPER)),
    };
    let spec = match named(key) {
        Some(named) => KeySpec::Named(named),
        None if ctrl != alt && (ctrl || !mac) => KeySpec::Character(printable(key)?.to_string()),
        None => return None,
    };
    let mods = KeyMods { shift: held(Mods::SHIFT), ctrl, alt, super_key };
    let mut event = KeyEvent::new(spec, mods);
    if repeat {
        event.kind = KeyEventKind::Repeat;
    }
    Some(event)
}

/// A key let go of: a named key, or one that types a character (its unshifted one). The
/// engine reports it only to a program that asked for event types.
pub(crate) fn release_event(key: Key, mods: Mods, mac: bool) -> Option<KeyEvent> {
    let mut event = key_event(key, mods, false, mac).or_else(|| {
        let shift = KeyMods { shift: mods.contains(Mods::SHIFT), ..KeyMods::default() };
        Some(KeyEvent::new(KeySpec::Character(printable(key)?.to_string()), shift))
    })?;
    event.kind = KeyEventKind::Release;
    Some(event)
}

fn named(key: Key) -> Option<NamedKey> {
    use NamedKey as N;
    Some(match key {
        Key::Enter => N::Enter,
        Key::Escape => N::Escape,
        Key::Backspace => N::Backspace,
        Key::Tab => N::Tab,
        Key::Delete => N::Delete,
        Key::Insert => N::Insert,
        Key::Home => N::Home,
        Key::End => N::End,
        Key::PageUp => N::PageUp,
        Key::PageDown => N::PageDown,
        Key::Left => N::ArrowLeft,
        Key::Right => N::ArrowRight,
        Key::Up => N::ArrowUp,
        Key::Down => N::ArrowDown,
        Key::F1 => N::F1,
        Key::F2 => N::F2,
        Key::F3 => N::F3,
        Key::F4 => N::F4,
        Key::F5 => N::F5,
        Key::F6 => N::F6,
        Key::F7 => N::F7,
        Key::F8 => N::F8,
        Key::F9 => N::F9,
        Key::F10 => N::F10,
        Key::F11 => N::F11,
        Key::F12 => N::F12,
        _ => return None,
    })
}

/// The character a key has on a US layout without Shift.
fn printable(key: Key) -> Option<char> {
    const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";
    const ROW: [(Key, char); 12] = [
        (Key::Space, ' '),
        (Key::Minus, '-'),
        (Key::Equal, '='),
        (Key::BracketLeft, '['),
        (Key::BracketRight, ']'),
        (Key::Backslash, '\\'),
        (Key::Semicolon, ';'),
        (Key::Quote, '\''),
        (Key::Comma, ','),
        (Key::Period, '.'),
        (Key::Slash, '/'),
        (Key::Backquote, '`'),
    ];
    let letter = (0..26).find(|&i| Key::letter(i) == key).map(|i| LETTERS.as_bytes()[i as usize] as char);
    let digit = (0..10).find(|&i| Key::digit(i) == key).and_then(|i| char::from_digit(i, 10));
    letter.or(digit).or_else(|| ROW.iter().find(|(k, _)| *k == key).map(|&(_, c)| c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_printable_key_is_the_terminals_only_with_ctrl_or_alt() {
        assert!(key_event(Key::A, Mods::NONE, false, false).is_none());
        assert!(key_event(Key::A, Mods::CTRL | Mods::ALT, false, false).is_none()); // AltGr types
        let ctrl = key_event(Key::C, Mods::CTRL, false, false).expect("Ctrl+C is a key");
        assert_eq!(ctrl.key, KeySpec::Character("c".into()));
        assert_eq!(
            key_event(Key::Up, Mods::NONE, false, false).map(|e| e.key),
            Some(KeySpec::Named(NamedKey::ArrowUp))
        );
    }

    #[test]
    fn on_a_mac_control_is_the_programs_and_command_and_option_are_not() {
        let control = key_event(Key::C, Mods::SUPER, false, true).expect("Control+C is a key");
        assert!(control.mods.ctrl && !control.mods.super_key);
        assert!(key_event(Key::C, Mods::CTRL, false, true).is_none(), "Command+C is the app's");
        assert!(key_event(Key::Left, Mods::CTRL, false, true).is_none());
        assert!(key_event(Key::A, Mods::ALT, false, true).is_none(), "Option+A types");
        assert!(key_event(Key::Left, Mods::ALT, false, true).is_some_and(|it| it.mods.alt));
    }
}

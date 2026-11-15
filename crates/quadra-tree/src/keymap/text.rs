//! A keystroke as the text `Keystroke::parse` reads: `ctrl-shift-t`, `alt-f4`, `ctrl-comma`.

use std::fmt;

use quadra_platform::{Key, Mods};

use super::{Keystroke, NAMED, key_named};

/// The name a key is written with, the first of its names.
fn name_of(key: Key) -> Option<String> {
    let letters = ('a'..='z').map(String::from);
    let digits = ('0'..='9').map(String::from);
    let functions = (1..=12).map(|n| format!("f{n}"));
    let named = NAMED.iter().map(|(name, _)| name.to_string());
    letters.chain(digits).chain(functions).chain(named).find(|name| key_named(name) == Some(key))
}

impl fmt::Display for Keystroke {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (held, word) in
            [(Mods::CTRL, "ctrl-"), (Mods::ALT, "alt-"), (Mods::SHIFT, "shift-"), (Mods::SUPER, "super-")]
        {
            if self.mods.contains(held) {
                f.write_str(word)?;
            }
        }
        f.write_str(&name_of(self.key).unwrap_or_else(|| "unknown".into()))
    }
}

impl Keystroke {
    /// Whether the key is one a binding can end with: not a modifier alone.
    pub fn is_bindable(&self) -> bool {
        !matches!(self.key, Key::Shift | Key::Ctrl | Key::Alt | Key::Super | Key::CapsLock | Key::Unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keystrokes_read_back_as_they_are_written() {
        for text in
            ["ctrl-shift-t", "alt-f4", "ctrl-comma", "escape", "ctrl-alt-left", "shift-insert", "super-9"]
        {
            assert_eq!(Keystroke::parse(text).unwrap().to_string(), text);
        }
        assert_eq!(Keystroke::parse("ctrl-,").unwrap().to_string(), "ctrl-comma");
    }
}

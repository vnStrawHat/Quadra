//! The keymap (specs/09, "Keys, contexts and actions"): bindings from keystroke sequences, in
//! a context, to actions. It is data: an app's defaults, then the user's.

use std::any::TypeId;
use std::fmt;
use std::rc::Rc;

use quadra_platform::{Key, Mods};

use quadra_text::Motion;

use crate::action::{Action, AnyAction, FocusNext, FocusPrevious, GoBack, GoForward};
use crate::field::TextCommand;

mod context;
mod file;
mod mac;
mod text;

use context::Context;
pub use file::Actions;

/// The key context of a text field: where the editing keys are bound.
pub const TEXT_CONTEXT: &str = "text";
/// The key context of a text field of several lines: the same keys, and those that go by
/// lines.
pub const TEXT_AREA_CONTEXT: &str = "text-area";
/// The key context of text that can be selected and not edited: select all, and copy.
pub const TEXT_SELECT_CONTEXT: &str = "text-select";

/// One key with the modifiers held.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Keystroke {
    pub key: Key,
    pub mods: Mods,
}

/// What is wrong with a binding, with the text that was given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeymapError(pub String);

impl fmt::Display for KeymapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the keymap is wrong: {}", self.0)
    }
}

impl std::error::Error for KeymapError {}

const NAMED: [(&str, Key); 34] = [
    ("enter", Key::Enter),
    ("escape", Key::Escape),
    ("esc", Key::Escape),
    ("backspace", Key::Backspace),
    ("tab", Key::Tab),
    ("space", Key::Space),
    ("delete", Key::Delete),
    ("insert", Key::Insert),
    ("home", Key::Home),
    ("end", Key::End),
    ("pageup", Key::PageUp),
    ("pagedown", Key::PageDown),
    ("left", Key::Left),
    ("right", Key::Right),
    ("up", Key::Up),
    ("down", Key::Down),
    ("minus", Key::Minus),
    ("equal", Key::Equal),
    ("=", Key::Equal),
    ("comma", Key::Comma),
    (",", Key::Comma),
    ("period", Key::Period),
    (".", Key::Period),
    ("slash", Key::Slash),
    ("/", Key::Slash),
    ("backslash", Key::Backslash),
    ("\\", Key::Backslash),
    ("semicolon", Key::Semicolon),
    (";", Key::Semicolon),
    ("quote", Key::Quote),
    ("'", Key::Quote),
    ("backquote", Key::Backquote),
    ("[", Key::BracketLeft),
    ("]", Key::BracketRight),
];

fn key_named(name: &str) -> Option<Key> {
    let byte = name.bytes().next().filter(|_| name.len() == 1);
    match (byte, name.strip_prefix('f').and_then(|n| n.parse::<u32>().ok())) {
        (Some(letter @ b'a'..=b'z'), _) => Some(Key::letter((letter - b'a') as u32)),
        (Some(digit @ b'0'..=b'9'), _) => Some(Key::digit((digit - b'0') as u32)),
        (_, Some(n @ 1..=12)) => Some(Key::function(n - 1)),
        _ => NAMED.iter().find(|(known, _)| *known == name).map(|&(_, key)| key),
    }
}

impl Keystroke {
    /// Parses "ctrl-shift-p", "f5", "alt-enter", "ctrl-,": modifiers, then one key, joined
    /// by "-". The minus key is written "minus".
    pub fn parse(text: &str) -> Result<Keystroke, KeymapError> {
        let wrong = || KeymapError(format!("no such keystroke: {text:?}"));
        let lower = text.to_ascii_lowercase();
        let mut parts: Vec<&str> = lower.split('-').collect();
        let key = key_named(parts.pop().filter(|k| !k.is_empty()).ok_or_else(wrong)?).ok_or_else(wrong)?;
        let mut mods = Mods::NONE;
        for part in parts {
            mods |= match part {
                // The key shortcuts are held with: Ctrl, and Command on a Mac.
                "ctrl" | "cmd" => Mods::CTRL,
                "shift" => Mods::SHIFT,
                "alt" => Mods::ALT,
                // The system's other: the Windows key, and Control on a Mac.
                "super" | "win" => Mods::SUPER,
                _ => return Err(wrong()),
            };
        }
        Ok(Keystroke { key, mods })
    }
}

struct Binding {
    keys: Vec<Keystroke>,
    /// The context it applies in: a predicate over the names on the focused node and those
    /// around it. `None`: everywhere.
    context: Option<Rc<Context>>,
    action: Rc<dyn AnyAction>,
}

/// What a keystroke means after the keys before it.
pub(crate) enum Match {
    /// It completes a binding: this is its action.
    Action(Rc<dyn AnyAction>),
    /// It is the start of a longer binding: the keys after it decide.
    Pending,
    None,
}

/// The bindings of an app. Later bindings win over earlier ones for the same keys in the same
/// context, so the user's are added after the defaults.
pub struct Keymap {
    bindings: Vec<Binding>,
    /// How many of them, the first ones, are those every keymap has.
    built_in: usize,
}

impl Default for Keymap {
    /// The bindings every window has: Tab and Shift+Tab move the focus.
    fn default() -> Keymap {
        let mut keymap = Keymap { bindings: Vec::new(), built_in: 0 };
        let tab = Keystroke { key: Key::Tab, mods: Mods::NONE };
        keymap.push(vec![tab], None, FocusNext);
        keymap.push(vec![Keystroke { mods: Mods::SHIFT, ..tab }], None, FocusPrevious);
        keymap.push(vec![Keystroke { key: Key::Left, mods: Mods::ALT }], None, GoBack);
        keymap.push(vec![Keystroke { key: Key::Right, mods: Mods::ALT }], None, GoForward);
        keymap.text_bindings();
        keymap.built_in = keymap.bindings.len();
        keymap
    }
}

impl Keymap {
    pub fn new() -> Keymap {
        Keymap::default()
    }

    /// What the keys do in a text field (the context "text"): the caret, the selection,
    /// deleting, undo.
    fn text_bindings(&mut self) {
        use TextCommand::{Copy, Cut, Delete, Move, Newline, Paste, Redo, SelectAll, Undo};
        let moves = [
            ("left", Motion::Left),
            ("right", Motion::Right),
            ("ctrl-left", Motion::WordLeft),
            ("ctrl-right", Motion::WordRight),
            ("home", Motion::LineStart),
            ("end", Motion::LineEnd),
            ("ctrl-home", Motion::Start),
            ("ctrl-end", Motion::End),
        ];
        let mut all: Vec<(String, TextCommand)> = Vec::new();
        for (keys, motion) in moves {
            all.push((keys.to_string(), Move(motion, false)));
            all.push((format!("shift-{keys}"), Move(motion, true)));
        }
        let rest = [
            ("backspace", Delete(Motion::Left)),
            ("shift-backspace", Delete(Motion::Left)),
            ("delete", Delete(Motion::Right)),
            ("ctrl-backspace", Delete(Motion::WordLeft)),
            ("ctrl-delete", Delete(Motion::WordRight)),
            ("ctrl-a", SelectAll),
            ("ctrl-z", Undo),
            ("ctrl-y", Redo),
            ("ctrl-shift-z", Redo),
            ("ctrl-c", Copy),
            ("ctrl-insert", Copy),
            ("ctrl-x", Cut),
            ("shift-delete", Cut),
            ("ctrl-v", Paste),
            ("shift-insert", Paste),
        ];
        all.extend(rest.map(|(keys, command)| (keys.to_string(), command)));
        // Where there are several lines: the same, the keys that go up and down, and Enter.
        // On one line those keys stay keys: for a list around the field, for submitting.
        let mut lines = vec![("enter".to_string(), Newline)];
        for (keys, motion) in [("up", Motion::Up), ("down", Motion::Down)] {
            lines.push((keys.to_string(), Move(motion, false)));
            lines.push((format!("shift-{keys}"), Move(motion, true)));
        }
        let read = [("ctrl-a", SelectAll), ("ctrl-c", Copy), ("ctrl-insert", Copy)];
        let read = read.map(|(keys, command)| (keys.to_string(), command));
        let both = all.iter().map(|binding| (TEXT_CONTEXT, binding));
        let area = all.iter().chain(&lines).map(|binding| (TEXT_AREA_CONTEXT, binding));
        let only = read.iter().map(|binding| (TEXT_SELECT_CONTEXT, binding));
        for (context, (keys, command)) in both.chain(area).chain(only) {
            let stroke = Keystroke::parse(keys).expect("the built-in bindings are well formed");
            self.push(vec![stroke], Some(Rc::new(Context::name(context))), *command);
        }
    }

    fn push(&mut self, keys: Vec<Keystroke>, context: Option<Rc<Context>>, action: impl Action) {
        self.bindings.push(Binding { keys, context, action: Rc::new(action) });
    }

    /// Binds a keystroke, or a sequence of them separated by spaces ("ctrl-k ctrl-c"), to an
    /// action, everywhere.
    pub fn bind(&mut self, keys: &str, action: impl Action) -> Result<(), KeymapError> {
        self.bind_in_context(None, keys, action)
    }

    /// As [`Keymap::bind`], where the focus is on a node of that context or inside one; or
    /// where a predicate of contexts holds (`editor && !find`, `a || b`). With the same keys,
    /// the binding whose context is nearest the focus wins.
    pub fn bind_in(&mut self, context: &str, keys: &str, action: impl Action) -> Result<(), KeymapError> {
        self.bind_in_context(Some(Rc::new(Context::parse(context)?)), keys, action)
    }

    fn bind_in_context(
        &mut self,
        context: Option<Rc<Context>>,
        keys: &str,
        action: impl Action,
    ) -> Result<(), KeymapError> {
        let keys: Vec<Keystroke> = keys.split_whitespace().map(Keystroke::parse).collect::<Result<_, _>>()?;
        if keys.is_empty() {
            return Err(KeymapError("a binding needs at least one keystroke".into()));
        }
        self.push(keys, context, action);
        Ok(())
    }

    /// The keys of the last binding of an action type: what a menu shows beside its item.
    pub fn keys_of<A: Action>(&self) -> Option<&[Keystroke]> {
        let of = |b: &&Binding| b.action.as_any().type_id() == TypeId::of::<A>();
        self.bindings.iter().rev().find(of).map(|b| b.keys.as_slice())
    }

    /// What `stroke` means after the `pending` keys of a sequence, where the focus has the
    /// `contexts` around it, the nearest first.
    pub(crate) fn find(&self, pending: &[Keystroke], stroke: Keystroke, contexts: &[&str]) -> Match {
        let mut best: Option<(usize, &Binding)> = None;
        let mut longer = false;
        for binding in &self.bindings {
            let (n, keys) = (pending.len(), &binding.keys);
            if keys.len() <= n || keys[..n] != *pending || keys[n] != stroke {
                continue;
            }
            // How near the focus its context is: the nearest is the largest.
            let near = match &binding.context {
                None => 0,
                Some(context) => match context.near(contexts) {
                    Some(near) => near,
                    None => continue,
                },
            };
            if keys.len() > n + 1 {
                longer = true;
            } else if best.is_none_or(|(known, _)| near >= known) {
                best = Some((near, binding)); // of equals, the one added last
            }
        }
        match best {
            Some((_, binding)) => Match::Action(binding.action.clone()),
            None if longer => Match::Pending,
            None => Match::None,
        }
    }
}

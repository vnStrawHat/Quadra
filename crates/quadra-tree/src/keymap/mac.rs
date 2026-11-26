//! The keys of a Mac (specs/09, "On a Mac"): where a PC holds Ctrl with an arrow to go by a
//! word, a Mac holds Option, and Command with an arrow goes to the end of the line or of
//! the text. `ctrl` in a binding is Command there (10, "Modifiers").

use std::any::TypeId;
use std::rc::Rc;

use quadra_text::Motion;

use super::{Context, Keymap, Keystroke, TEXT_AREA_CONTEXT, TEXT_CONTEXT};
use crate::action::{GoBack, GoForward};
use crate::field::TextCommand::{Delete, Move};

impl Keymap {
    /// The keymap as a Mac has its keys: the built-in bindings of text are joined by the
    /// Mac's, which win over them, and back and forward are Command with a bracket, not
    /// Option with an arrow. The bindings added after the built-in ones (a kit's, the
    /// app's, the user's) stay over these.
    pub fn as_a_mac_has_them(mut self) -> Keymap {
        let (back, forward) = (TypeId::of::<GoBack>(), TypeId::of::<GoForward>());
        let (built_in, mut at) = (self.built_in, 0);
        self.bindings.retain(|binding| {
            at += 1;
            let kind = binding.action.as_any().type_id();
            at > built_in || !(kind == back || kind == forward)
        });
        let kept = built_in - (at - self.bindings.len());

        let mut mac = Keymap { bindings: Vec::new(), built_in: 0 };
        let stroke =
            |keys: &str| vec![Keystroke::parse(keys).expect("the built-in bindings are well formed")];
        mac.push(stroke("ctrl-["), None, GoBack);
        mac.push(stroke("ctrl-]"), None, GoForward);
        let moves = [
            ("alt-left", Motion::WordLeft),
            ("alt-right", Motion::WordRight),
            ("ctrl-left", Motion::LineStart),
            ("ctrl-right", Motion::LineEnd),
            ("ctrl-up", Motion::Start),
            ("ctrl-down", Motion::End),
        ];
        let deletes = [
            ("alt-backspace", Motion::WordLeft),
            ("alt-delete", Motion::WordRight),
            ("ctrl-backspace", Motion::LineStart),
        ];
        for context in [TEXT_CONTEXT, TEXT_AREA_CONTEXT] {
            let context = Some(Rc::new(Context::name(context)));
            for (keys, motion) in moves {
                mac.push(stroke(keys), context.clone(), Move(motion, false));
                mac.push(stroke(&format!("shift-{keys}")), context.clone(), Move(motion, true));
            }
            for (keys, motion) in deletes {
                mac.push(stroke(keys), context.clone(), Delete(motion));
            }
        }
        self.built_in = kept + mac.bindings.len();
        self.bindings.splice(kept..kept, mac.bindings);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::super::Match;
    use super::*;
    use crate::field::TextCommand;

    fn in_text(keymap: &Keymap, keys: &str, contexts: &[&str]) -> Option<TextCommand> {
        match keymap.find(&[], Keystroke::parse(keys).unwrap(), contexts) {
            Match::Action(action) => action.as_any().downcast_ref::<TextCommand>().copied(),
            _ => None,
        }
    }

    #[test]
    fn a_mac_goes_by_words_with_option_and_to_the_ends_with_command() {
        let mut keymap = Keymap::new();
        // A binding of the app's, added before the keymap is made a Mac's, stays over it.
        keymap.bind_in(TEXT_CONTEXT, "alt-delete", TextCommand::SelectAll).unwrap();
        let (pc, mac) = (Keymap::new(), keymap.as_a_mac_has_them());

        let text = &[TEXT_CONTEXT][..];
        assert_eq!(in_text(&pc, "ctrl-left", text), Some(Move(Motion::WordLeft, false)));
        assert_eq!(in_text(&mac, "ctrl-left", text), Some(Move(Motion::LineStart, false)));
        assert_eq!(in_text(&mac, "alt-left", text), Some(Move(Motion::WordLeft, false)));
        assert_eq!(in_text(&mac, "shift-alt-right", text), Some(Move(Motion::WordRight, true)));
        assert_eq!(in_text(&mac, "ctrl-shift-down", &[TEXT_AREA_CONTEXT]), Some(Move(Motion::End, true)));
        assert_eq!(in_text(&mac, "alt-backspace", text), Some(Delete(Motion::WordLeft)));
        assert_eq!(in_text(&mac, "ctrl-backspace", text), Some(Delete(Motion::LineStart)));
        assert_eq!(in_text(&mac, "alt-delete", text), Some(TextCommand::SelectAll));
        // What a PC and a Mac share is still there.
        assert_eq!(in_text(&mac, "ctrl-a", text), Some(TextCommand::SelectAll));
        assert_eq!(in_text(&mac, "home", text), Some(Move(Motion::LineStart, false)));

        // Outside a text, Option with an arrow is no longer back: Command with a bracket is.
        let goes = |keymap: &Keymap, keys: &str| {
            matches!(keymap.find(&[], Keystroke::parse(keys).unwrap(), &[]), Match::Action(_))
        };
        assert!(goes(&pc, "alt-left") && !goes(&pc, "ctrl-["));
        assert!(!goes(&mac, "alt-left") && goes(&mac, "ctrl-[") && goes(&mac, "ctrl-]"));
    }
}

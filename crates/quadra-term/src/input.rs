//! What the user sends to a session, for an app that repeats it in others (specs/18,
//! "Input"): the hook told it, and the delivery that writes it to another session as its own.

use std::rc::Rc;

use oneterm_vt::input::{KeyEvent, KeyMods, KeySpec};
use quadra_tree::{NodeId, Tree};

use crate::session::Session;

/// What the user sent: a key as the view encoded it (or Tab), typed text, a paste.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input<'a> {
    Keys(&'a [u8]),
    Text(&'a str),
    Paste(&'a str),
}

pub(crate) type InputFn = dyn Fn(&Input);

impl Session {
    /// Runs `f` with what the user sends through a view of the session, [`Session::paste`]
    /// or [`Session::send`], after it is written; not the mouse, focus reports, replies, the
    /// wheel's arrows or what [`Session::deliver`] writes. One hook: another replaces it.
    pub fn on_input(&self, f: impl Fn(&Input) + 'static) {
        *self.0.input.borrow_mut() = Some(Rc::new(f));
    }

    /// Writes input as the user's own, as [`Session::deliver`] does, and tells the hook: what
    /// an app types for the user (an accepted completion).
    pub fn send(&self, input: &Input) {
        self.deliver(input);
        self.told(input);
    }

    pub(crate) fn told(&self, input: &Input) {
        let hook = self.0.input.borrow().clone();
        if let Some(hook) = hook {
            hook(input);
        }
    }

    /// Writes what was sent to another session as this one's own: keys as they are, text
    /// and a paste encoded by its engine (a paste bracketed only when its program asked),
    /// the view back at the bottom. The hook is not told.
    pub fn deliver(&self, input: &Input) {
        let (bytes, back) = {
            let mut engine = self.engine();
            let back = engine.term.screen().scroll_offset() > 0;
            if back {
                engine.term.scroll_viewport(i32::MAX);
            }
            let bytes = match *input {
                Input::Keys(bytes) => bytes.to_vec(),
                Input::Text(text) => {
                    let event = KeyEvent::new(KeySpec::Character(text.to_string()), KeyMods::default());
                    engine.term.encode_key_event(&event).unwrap_or_else(|| text.as_bytes().to_vec())
                }
                Input::Paste(text) => engine.term.encode_paste(text),
            };
            (bytes, back)
        };
        if back {
            self.repaint();
        }
        self.write(&bytes);
    }
}

/// Writes what the user sent through the view, then tells the hook.
pub(crate) fn sent(tree: &mut Tree, id: NodeId, s: &Session, bytes: &[u8], input: Input) {
    crate::view::send(tree, id, s, bytes);
    if !bytes.is_empty() {
        s.told(&input);
    }
}

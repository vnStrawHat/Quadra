//! Actions (specs/09): what a command is, as a Rust type. Keys, menus, a command palette and
//! buttons dispatch the same action, so a command has one implementation.

use std::any::{Any, TypeId};
use std::rc::Rc;

use quadra_reactive::batch;

use crate::input::Handler;
use crate::keymap::{Keymap, Keystroke, Match};
use crate::node::NodeId;
use crate::tree::Tree;

/// A command. It is handled by type; `NAME` is what a keymap file and a palette call it. An
/// action may carry data (`SelectTab(3)`).
pub trait Action: Any {
    const NAME: &'static str;
}

/// An action of any type, as the keymap keeps it.
pub(crate) trait AnyAction {
    fn as_any(&self) -> &dyn Any;
}

impl<A: Action> AnyAction for A {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Moves the focus to the next node that can have it. Bound to Tab; handled by the tree when
/// no node handles it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FocusNext;

impl Action for FocusNext {
    const NAME: &'static str = "focus::next";
}

/// Moves the focus to the node before. Bound to Shift+Tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FocusPrevious;

impl Action for FocusPrevious {
    const NAME: &'static str = "focus::previous";
}

/// The keymap a tree uses, and the keys of a sequence typed so far.
pub(crate) struct Keys {
    pub map: Rc<Keymap>,
    pub pending: Vec<Keystroke>,
}

impl Default for Keys {
    fn default() -> Keys {
        Keys { map: Rc::new(Keymap::default()), pending: Vec::new() }
    }
}

impl Tree {
    /// Gives the tree the keymap of its app: the windows of an app share one.
    pub fn set_keymap(&mut self, keymap: Rc<Keymap>) {
        self.keys = Keys { map: keymap, pending: Vec::new() };
    }

    /// Names the node as a context of key bindings: a binding made for that context applies
    /// while the focus is on the node or inside it.
    pub fn set_key_context(&mut self, id: NodeId, context: &str) {
        if let Some(node) = self.get_mut(id) {
            node.key_context = Some(Rc::from(context));
        }
    }

    /// Runs `f` when an action of type `A` reaches the node: the focus is on it or inside it,
    /// and no node nearer the focus handled the action.
    pub fn on_action<A: Action>(&mut self, id: NodeId, mut f: impl FnMut(&A) + 'static) {
        let run = move |action: &dyn Any| {
            if let Some(action) = action.downcast_ref::<A>() {
                f(action)
            }
        };
        self.add_handler(id, Handler::Action { of: TypeId::of::<A>(), run: Box::new(run) });
    }

    /// Dispatches an action as a key binding would: from the focused node (or the root) up to
    /// the first node that handles it. Returns whether anything handled it.
    pub fn dispatch<A: Action>(&mut self, action: &A) -> bool {
        batch(|| self.dispatch_any(action))
    }

    pub(crate) fn dispatch_any(&mut self, action: &dyn AnyAction) -> bool {
        let Some(target) = self.focus.filter(|&id| self.is_alive(id)).or(self.root) else { return false };
        let of = action.as_any().type_id();
        for node in self.path(target) {
            for handler in &mut self.nodes[node.index as usize].handlers {
                if let Handler::Action { of: handled, run } = handler
                    && *handled == of
                {
                    run(action.as_any());
                    return true;
                }
            }
        }
        // What the tree does itself when no node wanted the action.
        let own = [(TypeId::of::<FocusNext>(), false), (TypeId::of::<FocusPrevious>(), true)];
        match own.iter().find(|(known, _)| *known == of) {
            Some(&(_, back)) => self.focus_next(back),
            None => return false,
        }
        true
    }

    /// What a key press means in the keymap, after the keys of a sequence typed before it:
    /// an action that something handled (true), the start of a sequence (true: the key is
    /// taken), or nothing (false: the key goes on as a key).
    pub(crate) fn key_binding(&mut self, stroke: Keystroke, target: NodeId) -> bool {
        let contexts: Vec<Rc<str>> = self
            .path(target)
            .into_iter()
            .filter_map(|id| self.nodes[id.index as usize].key_context.clone())
            .collect();
        let found = self.keys.map.find(&self.keys.pending, stroke, &contexts);
        match found {
            Match::Action(action) => {
                self.keys.pending.clear();
                self.dispatch_any(&*action)
            }
            Match::Pending => {
                self.keys.pending.push(stroke);
                true
            }
            Match::None => {
                // A sequence that leads nowhere is dropped; its last key is an ordinary key.
                self.keys.pending.clear();
                false
            }
        }
    }
}

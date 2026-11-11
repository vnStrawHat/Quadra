//! Focus and keys (specs/09, "Focus", "Keys"): one node of a window has the focus; keys go
//! down the path to it and back up; Tab moves the focus in tree order inside the nearest
//! focus scope.

use quadra_layout::Display;
use quadra_platform::{Key, Mods};
use quadra_prim::Rect;
use quadra_reactive::batch;

use crate::input::Handler;
use crate::keymap::Keystroke;
use crate::node::NodeId;
use crate::style::State;
use crate::tree::Tree;

/// What a key handler is given: the key, and the controls of its dispatch.
#[derive(Clone, Debug)]
pub struct KeyEvent {
    pub key: Key,
    /// The OS's own key code, for diagnostics only.
    pub code: u32,
    pub mods: Mods,
    pub down: bool,
    /// The key is held: this press is a repeat.
    pub repeat: bool,
    /// The node that has the focus, or the root when none has.
    pub target: NodeId,
    /// The node the handler is on.
    pub node: NodeId,
    pub(crate) stopped: bool,
}

impl KeyEvent {
    /// Ends the dispatch: no handler after this one runs.
    pub fn stop(&mut self) {
        self.stopped = true;
    }
}

impl Tree {
    /// Lets a node have the focus: a click on it or in it focuses it, and Tab stops at it.
    pub fn set_focusable(&mut self, id: NodeId, on: bool) {
        if let Some(node) = self.get_mut(id) {
            node.focusable = on;
            self.semantics_changed(id);
        }
        if !on && self.focus == Some(id) {
            self.set_focus(None, false);
        }
    }

    /// Makes a node a focus scope: Tab and Shift+Tab stay among what is under it.
    pub fn set_focus_scope(&mut self, id: NodeId, on: bool) {
        if let Some(node) = self.get_mut(id) {
            node.focus_scope = on;
        }
    }

    /// Runs `f` for key presses (`down`) or releases that reach the node on their way up from
    /// the focused node.
    pub fn on_key(&mut self, id: NodeId, down: bool, f: impl FnMut(&mut KeyEvent) + 'static) {
        self.add_handler(id, Handler::Key { down, capture: false, run: Box::new(f) });
    }

    /// As [`Tree::on_key`], on the way down from the root: before every other handler.
    pub fn on_key_capture(&mut self, id: NodeId, down: bool, f: impl FnMut(&mut KeyEvent) + 'static) {
        self.add_handler(id, Handler::Key { down, capture: true, run: Box::new(f) });
    }

    /// Runs `f` with true when the focus comes to the node and with false when it leaves.
    pub fn on_focus(&mut self, id: NodeId, f: impl FnMut(bool) + 'static) {
        self.add_handler(id, Handler::Focus(Box::new(f)));
    }

    /// Runs `f` with the text typed while the node has the focus: committed characters, never
    /// control keys.
    pub fn on_text(&mut self, id: NodeId, f: impl FnMut(&str) + 'static) {
        self.add_handler(id, Handler::Text(Box::new(f)));
    }

    /// Takes text typed in the window: it goes to the focused node, if that takes text.
    pub fn text(&mut self, text: &str) {
        let Some(focus) = self.focus.filter(|&id| self.is_alive(id)) else { return };
        batch(|| {
            for handler in &mut self.nodes[focus.index as usize].handlers {
                if let Handler::Text(run) = handler {
                    run(text);
                }
            }
        });
    }

    /// Runs `f` with the text an input method is composing while the node has the focus, and
    /// the caret in it; with an empty text when the composition is over.
    pub fn on_composition(&mut self, id: NodeId, f: impl FnMut(&str, usize) + 'static) {
        self.add_handler(id, Handler::Composition(Box::new(f)));
    }

    /// Takes what an input method is composing in the window: for the focused node.
    pub fn composition(&mut self, text: &str, caret: usize) {
        let Some(focus) = self.focus.filter(|&id| self.is_alive(id)) else { return };
        batch(|| {
            for handler in &mut self.nodes[focus.index as usize].handlers {
                if let Handler::Composition(run) = handler {
                    run(text, caret);
                }
            }
        });
    }

    /// The caret of the focused node, in the window, if that node takes text: where an
    /// input method opens. `None` turns input methods off.
    pub fn text_caret(&self) -> Option<Rect> {
        let focus = self.focus.filter(|&id| self.is_alive(id))?;
        let caret = self.nodes[focus.index as usize].widget.as_ref()?.caret()?;
        let at = self.window_rect(focus).origin();
        Some(caret.translate(at.x, at.y))
    }

    /// Lets a node have the focus, and gives it a place in the order Tab walks: above zero
    /// before the nodes without one, lowest first; below zero never by Tab, only by a press or
    /// by the program.
    pub fn set_tab_index(&mut self, id: NodeId, index: i16) {
        if let Some(node) = self.get_mut(id) {
            node.tab_index = index;
            self.set_focusable(id, true);
        }
    }

    /// The node that has the focus.
    pub fn focused(&self) -> Option<NodeId> {
        self.focus
    }

    /// Gives a node the focus, as a program does: without the ring of a focus that came by
    /// the keyboard. Nothing happens if it cannot have the focus.
    pub fn focus(&mut self, id: NodeId) {
        if self.get(id).is_some_and(|node| node.focusable) {
            batch(|| self.set_focus(Some(id), false));
        }
    }

    /// Takes the focus from whatever has it.
    pub fn blur(&mut self) {
        batch(|| self.set_focus(None, false));
    }

    /// The window got or lost the keyboard. A window without it shows no focus; the node that
    /// had it has it again when the window is back.
    pub fn set_active(&mut self, active: bool) {
        if std::mem::replace(&mut self.active, active) != active {
            self.focus_states(self.focus, active, active && self.focus_ring);
        }
    }

    /// Moves the focus: the states of the old and the new node and of their ancestors, then
    /// the events, the one that loses it first.
    pub(crate) fn set_focus(&mut self, to: Option<NodeId>, by_keyboard: bool) {
        let from = self.focus;
        self.focus_ring = to.is_some() && (by_keyboard || (from == to && self.focus_ring));
        if from == to {
            if let Some(id) = to {
                self.set_state(id, State::FOCUS_VISIBLE, self.active && self.focus_ring);
            }
            return;
        }
        self.focus_states(from, false, false);
        self.focus = to;
        self.focus_states(to, self.active, self.active && by_keyboard);
        for (node, gained) in [(from, false), (to, true)] {
            let Some(node) = node.filter(|&n| self.is_alive(n)) else { continue };
            for handler in &mut self.nodes[node.index as usize].handlers {
                if let Handler::Focus(run) = handler {
                    run(gained);
                }
            }
        }
    }

    /// Sets the focus states of a node and `FOCUS_WITHIN` on it and its ancestors.
    fn focus_states(&mut self, node: Option<NodeId>, on: bool, visible: bool) {
        let Some(node) = node.filter(|&n| self.is_alive(n)) else { return };
        self.set_state(node, State::FOCUS, on);
        self.set_state(node, State::FOCUS_VISIBLE, visible);
        self.semantics_changed(node); // a listener is told which node has the focus
        for id in self.path(node) {
            self.set_state(id, State::FOCUS_WITHIN, on);
        }
    }

    /// What a press does for the focus: the deepest node of the path that can have it gets
    /// it; a press on nothing focusable takes it away.
    pub(crate) fn focus_on_press(&mut self, path: &[NodeId]) {
        let to = path.iter().copied().find(|&id| self.nodes[id.index as usize].focusable);
        self.set_focus(to, false);
    }

    /// Before a node and what is under it is removed: a focus inside goes to the nearest
    /// ancestor that can have it, or to nothing.
    pub(crate) fn focus_leaves(&mut self, removed: NodeId) {
        let Some(focus) = self.focus else { return };
        let path = self.path(focus);
        let Some(at) = path.iter().position(|&id| id == removed) else { return };
        let to = path[at + 1..].iter().copied().find(|&id| self.nodes[id.index as usize].focusable);
        batch(|| self.set_focus(to, false));
    }

    /// Takes a key event of the window. A press is first looked up in the keymap: an action
    /// that something handles, or the start of a key sequence, is all the key does. Otherwise
    /// the key goes down the path to the focused node (or the root) and back up.
    pub fn key(&mut self, key: Key, code: u32, mods: Mods, down: bool, repeat: bool) {
        let Some(target) = self.focus.filter(|&id| self.is_alive(id)).or(self.root) else { return };
        batch(|| {
            // A modifier pressed on its way to a keystroke is not one itself.
            let modifier = matches!(key, Key::Shift | Key::Ctrl | Key::Alt | Key::Super);
            if down && !modifier && self.key_binding(Keystroke { key, mods }, target) {
                return;
            }
            let mut event = KeyEvent { key, code, mods, down, repeat, target, node: target, stopped: false };
            let path = self.path(target);
            'walk: for capture in [true, false] {
                for i in 0..path.len() {
                    let node = if capture { path[path.len() - 1 - i] } else { path[i] };
                    event.node = node;
                    for handler in &mut self.nodes[node.index as usize].handlers {
                        if let Handler::Key { down: d, capture: c, run } = handler
                            && *d == down
                            && *c == capture
                            && !event.stopped
                        {
                            run(&mut event);
                        }
                    }
                    if event.stopped {
                        break 'walk;
                    }
                }
            }
        });
    }

    /// Moves the focus to the next node that can have it, in tree order, or to the one before
    /// with `back`; after the last comes the first. It stays inside the nearest focus scope
    /// around the focused node. The focus shows its ring: it came by the keyboard.
    pub fn focus_next(&mut self, back: bool) {
        let Some(root) = self.root else { return };
        let scope = self.focus.filter(|&id| self.is_alive(id)).and_then(|focus| {
            self.path(focus).into_iter().find(|&id| self.nodes[id.index as usize].focus_scope)
        });
        let mut stops = Vec::new();
        self.tab_stops(scope.unwrap_or(root), &mut stops);
        // Those with an index above zero come first, lowest first; the rest in tree order.
        stops.sort_by_key(|id| match self.nodes[id.index as usize].tab_index {
            0 => i16::MAX,
            index => index,
        });
        if stops.is_empty() {
            return;
        }
        let at = self.focus.and_then(|focus| stops.iter().position(|&id| id == focus));
        let next = match (at, back) {
            (Some(i), false) => (i + 1) % stops.len(),
            (Some(i), true) => (i + stops.len() - 1) % stops.len(),
            (None, false) => 0,
            (None, true) => stops.len() - 1,
        };
        batch(|| self.set_focus(Some(stops[next]), true));
        self.scroll_into_view(stops[next]); // the keyboard went there: it is shown
    }

    /// The nodes under `id` that can have the focus, in tree order. What is not displayed is
    /// left out with everything under it.
    fn tab_stops(&self, id: NodeId, out: &mut Vec<NodeId>) {
        let node = &self.nodes[id.index as usize];
        if self.layout.style(node.layout).display == Display::None {
            return;
        }
        if node.focusable && node.tab_index >= 0 {
            out.push(id);
        }
        for &child in &node.children {
            self.tab_stops(child, out);
        }
    }
}

//! Walking the focus (specs/09, "Focus"): the order Tab goes by, inside a focus scope, and
//! the first place the focus goes in a part of the tree.

use quadra_layout::Display;
use quadra_reactive::batch;

use crate::node::NodeId;
use crate::style::State;
use crate::tree::Tree;

impl Tree {
    /// Lets a node have the focus, and gives it a place in the order Tab walks: above zero
    /// before the nodes without one, lowest first; below zero never by Tab, only by a press or
    /// by the program.
    pub fn set_tab_index(&mut self, id: NodeId, index: i16) {
        if let Some(node) = self.get_mut(id) {
            node.tab_index = index;
            self.set_focusable(id, true);
        }
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
        // What is not displayed, or is disabled, is left out with everything under it.
        if self.layout.style(node.layout).display == Display::None || node.state.contains(State::DISABLED) {
            return;
        }
        if node.focusable && node.tab_index >= 0 {
            out.push(id);
        }
        for &child in &node.children {
            self.tab_stops(child, out);
        }
    }

    /// Gives the focus to the first node under `id` that Tab would reach, or to `id` itself
    /// when there is none and it can have the focus: where the focus goes when a popup
    /// opens. Returns whether it went anywhere.
    pub fn focus_first(&mut self, id: NodeId) -> bool {
        if !self.is_alive(id) {
            return false;
        }
        let mut stops = Vec::new();
        self.tab_stops(id, &mut stops);
        let to = stops.first().copied().or(self.nodes[id.index as usize].focusable.then_some(id));
        if let Some(to) = to {
            batch(|| self.set_focus(Some(to), false));
        }
        to.is_some()
    }
}

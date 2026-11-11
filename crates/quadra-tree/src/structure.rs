//! Changing the tree's structure: children, removal, the scope a node owns (specs/04,
//! "Mounting and removing").

use quadra_reactive::Scope;

use crate::node::{NodeId, flag};
use crate::tree::Tree;

impl Tree {
    /// Sets the ordered children of `parent`. A child that had another parent leaves it.
    pub fn set_children(&mut self, parent: NodeId, children: &[NodeId]) {
        if self.node(parent).children == children {
            return;
        }
        let old = std::mem::take(&mut self.nodes[parent.index as usize].children);
        for child in old.into_iter().filter(|c| !children.contains(c)) {
            if let Some(node) = self.get_mut(child) {
                node.parent = None;
            }
        }
        for &child in children {
            let was = self.node(child).parent;
            if was == Some(parent) {
                continue;
            }
            if let Some(other) = was {
                self.nodes[other.index as usize].children.retain(|&c| c != child);
                self.mark(other, flag::RELIST);
            }
            // In a new place: positions in a layer are absolute, so all of it is painted.
            self.nodes[child.index as usize].parent = Some(parent);
            if let Some(state) = self.nodes[child.index as usize].layer.as_mut() {
                state.marked = false; // its marker is not in this parent's list yet
            }
            self.mark(child, flag::PAINT | flag::SUBTREE);
        }
        self.nodes[parent.index as usize].children = children.to_vec();
        let layouts: Vec<_> = children.iter().map(|c| self.nodes[c.index as usize].layout).collect();
        self.layout.set_children(self.nodes[parent.index as usize].layout, &layouts);
        self.relayout = true;
        self.mark(parent, flag::RELIST);
        self.semantics_changed(parent);
    }

    /// Removes a node and everything under it. Their scopes end: bindings, handlers and tasks
    /// they owned never run again.
    pub fn remove(&mut self, id: NodeId) {
        let Some((parent, layout)) = self.get(id).map(|n| (n.parent, n.layout)) else { return };
        self.focus_leaves(id);
        if let Some(parent) = parent {
            self.nodes[parent.index as usize].children.retain(|&c| c != id);
            self.mark(parent, flag::RELIST);
            self.semantics_changed(parent);
        }
        if self.root == Some(id) {
            self.root = None;
        }
        self.forget_overlay(id);
        self.layout.remove(layout);
        self.relayout = true;
        self.repaint = true;
        let mut scopes = Vec::new();
        self.free_subtree(id, &mut scopes);
        // After the nodes are gone: a cleanup may use the tree's handle.
        scopes.into_iter().for_each(Scope::dispose);
    }

    fn free_subtree(&mut self, id: NodeId, scopes: &mut Vec<Scope>) {
        let node = &mut self.nodes[id.index as usize];
        node.alive = false;
        node.generation = node.generation.wrapping_add(1);
        node.widget = None;
        node.handlers = Vec::new();
        (node.focusable, node.focus_scope, node.key_context) = (false, false, None);
        node.semantics = None;
        (node.state, node.styled) = (crate::State::NONE, None);
        self.sizes.remove(&id.index);
        if let Some(state) = node.layer.take().filter(|s| s.id != self.root_layer) {
            self.scene.remove_layer(state.id);
        }
        scopes.extend(node.scope.take());
        self.free.push(id.index);
        for child in std::mem::take(&mut node.children) {
            self.free_subtree(child, scopes);
        }
    }

    /// Gives a node the scope that ends with it.
    pub fn set_scope(&mut self, id: NodeId, scope: Scope) {
        self.get_mut(id).expect("a node was used after it was removed").scope = Some(scope);
    }
}

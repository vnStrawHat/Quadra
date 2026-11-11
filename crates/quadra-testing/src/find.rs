//! Finding and driving controls through the semantic tree (specs/12, "The harness"): what a
//! screen reader can reach, a test reaches the same way.

use quadra_a11y::snapshot;
use quadra_tree::{NodeId, Role, SemanticAction, SemanticNode};

use crate::Harness;

/// A node a query found, and the harness to act on it with.
pub struct Found<'a> {
    harness: &'a mut Harness,
    node: NodeId,
}

impl Found<'_> {
    pub fn node(&self) -> NodeId {
        self.node
    }

    /// Asks the node to do something, as assistive technology would. Fails if it does not.
    pub fn act(self, action: SemanticAction) -> Self {
        let done = self.harness.tree.act(self.node, &action);
        assert!(done, "{action:?} did nothing on this node:\n{}", self.harness.semantics());
        self
    }

    /// Clicks without the pointer: the node may be covered or scrolled out of view.
    pub fn click(self) -> Self {
        self.act(SemanticAction::Click)
    }

    pub fn focus(self) -> Self {
        self.act(SemanticAction::Focus)
    }

    /// Replaces what an input holds.
    pub fn set_value(self, value: &str) -> Self {
        self.act(SemanticAction::SetValue(value.to_string()))
    }

    /// Focuses the node and types into it.
    pub fn type_text(self, text: &str) -> Self {
        let found = self.focus();
        found.harness.type_text(text);
        found
    }
}

impl Harness {
    /// The semantic tree as text, one node a line: what a snapshot test compares.
    pub fn semantics(&self) -> String {
        let tree = self.tree.semantic_tree();
        snapshot(&tree.nodes, tree.root)
    }

    /// The nodes of the semantic tree that `matches` accepts.
    pub fn query(&self, matches: impl Fn(&SemanticNode) -> bool) -> Vec<NodeId> {
        let tree = self.tree.semantic_tree();
        tree.nodes.iter().filter(|n| matches(n)).filter_map(|n| self.tree.node_of(n.id)).collect()
    }

    /// The one node that `matches` accepts. None, or more than one, fails with the tree.
    fn one(&mut self, what: &str, matches: impl Fn(&SemanticNode) -> bool) -> Found<'_> {
        let found = self.query(matches);
        assert!(found.len() == 1, "{} nodes are {what}, not one:\n{}", found.len(), self.semantics());
        Found { node: found[0], harness: self }
    }

    /// The node of a role and a name: `get_by_role(Role::Button, "Save")`.
    pub fn get_by_role(&mut self, role: Role, name: &str) -> Found<'_> {
        let what = format!("{role:?} {name:?}");
        self.one(&what, |n| n.semantics.role == role && n.semantics.name == name)
    }

    /// The node whose name, or value, is a text.
    pub fn get_by_text(&mut self, text: &str) -> Found<'_> {
        let what = format!("called {text:?}");
        self.one(&what, |n| n.semantics.name == text || n.semantics.value.as_deref() == Some(text))
    }

    pub fn get_by_test_id(&mut self, id: &str) -> Found<'_> {
        let what = format!("#{id}");
        self.one(&what, |n| n.semantics.test_id.as_deref() == Some(id))
    }

    /// Whether some node's name, or value, is a text.
    pub fn has_text(&self, text: &str) -> bool {
        let named = |n: &SemanticNode| n.semantics.name == text || n.semantics.value.as_deref() == Some(text);
        !self.query(named).is_empty()
    }
}

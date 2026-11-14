//! How nodes stand to each other for a reader (specs/12): named by, described by, controls.

use quadra_a11y::Relation;

use crate::node::NodeId;
use crate::semantics::semantic_id;
use crate::tree::Tree;

impl Tree {
    /// Says how a node stands to another: named by it, described by it, or changing what it
    /// shows. A reader is told the node of the semantic tree the other is, or is part of.
    pub fn relate(&mut self, id: NodeId, relation: Relation, to: NodeId) {
        let to = semantic_id(to);
        self.update_semantics(id, |says| {
            if !says.relations.contains(&(relation, to)) {
                says.relations.push((relation, to));
            }
        });
    }

    /// The relations of a node as a listener is told them: to the semantic nodes the others
    /// are, hold first, or are part of; those gone, or not shown, are left out.
    pub(crate) fn resolve_relations(&self, relations: &mut Vec<(Relation, u64)>) {
        relations.retain_mut(|(_, to)| {
            let node = self.node_of(*to).filter(|&node| self.semantic_around(node, true).is_some());
            let found = node.and_then(|node| self.semantic_within(node).or(self.semantic_around(node, true)));
            found.map(|node| *to = semantic_id(node)).is_some()
        });
    }

    /// The first node of the semantic tree at or under a node, in order, among those shown.
    fn semantic_within(&self, id: NodeId) -> Option<NodeId> {
        if self.hidden(id) {
            return None;
        }
        if self.means(id) {
            return Some(id);
        }
        self.children(id).iter().find_map(|&child| self.semantic_within(child))
    }
}

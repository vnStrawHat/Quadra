//! The layout tree: the layout part of every node, kept across frames (specs/06, "The engine").

use std::rc::Rc;

use quadra_prim::{Rect, Size};

use crate::cache::Answers;
use crate::flex_items::Scratch;
use crate::incremental::{RootSize, Stats};
use crate::style::Style;

/// A node of a [`LayoutTree`]. Using one after it was removed panics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LayoutId(u32);

impl LayoutId {
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }

    pub(crate) fn from_index(index: usize) -> LayoutId {
        LayoutId(index as u32)
    }
}

pub(crate) const NOT_PENDING: u32 = u32::MAX;

#[derive(Clone)]
pub(crate) struct Node {
    pub style: Rc<Style>,
    pub children: Vec<LayoutId>,
    pub parent: Option<LayoutId>,
    /// Sized by the measurer, not by its children.
    pub leaf: bool,
    pub alive: bool,
    /// Border box, relative to the parent's border box.
    pub rect: Rect,
    /// Extent of the children, for scrolling.
    pub content: Size,
    pub answers: Answers,
    /// Index of its old answers in `LayoutTree::saved` while it waits for the next update.
    pub pending: u32,
    /// How it was last laid out as a root.
    pub root: Option<RootSize>,
}

/// The answers a changed node had given, kept until the next update asks it again.
#[derive(Clone)]
pub(crate) struct Saved {
    pub id: LayoutId,
    pub answers: Answers,
    /// A value its parent reads from its style changed: the parent runs whatever it answers.
    pub item_changed: bool,
}

#[derive(Clone, Default)]
pub struct LayoutTree {
    pub(crate) nodes: Vec<Node>,
    free: Vec<u32>,
    pub(crate) live: usize,
    pub(crate) saved: Vec<Option<Saved>>,
    pub(crate) scratch: Vec<Scratch>,
    pub(crate) depth: usize,
    pub(crate) stats: Stats,
}

impl LayoutTree {
    pub fn new() -> LayoutTree {
        LayoutTree::default()
    }

    /// Adds a container: its size comes from its style and its children.
    pub fn insert(&mut self, style: Style) -> LayoutId {
        self.add(style, false)
    }

    /// Adds a leaf: its content size comes from the measurer given to the layout calls.
    pub fn insert_leaf(&mut self, style: Style) -> LayoutId {
        self.add(style, true)
    }

    fn add(&mut self, style: Style, leaf: bool) -> LayoutId {
        let node = Node {
            style: Rc::new(style),
            children: Vec::new(),
            parent: None,
            leaf,
            alive: true,
            rect: Rect::default(),
            content: Size::ZERO,
            answers: Answers::default(),
            pending: NOT_PENDING,
            root: None,
        };
        self.live += 1;
        if let Some(index) = self.free.pop() {
            self.nodes[index as usize] = node;
            return LayoutId(index);
        }
        self.nodes.push(node);
        LayoutId(self.nodes.len() as u32 - 1)
    }

    /// Removes a node and everything under it.
    pub fn remove(&mut self, id: LayoutId) {
        if let Some(parent) = self.node(id).parent {
            self.nodes[parent.index()].children.retain(|&c| c != id);
            self.mark(parent, false);
        }
        self.free_subtree(id);
    }

    fn free_subtree(&mut self, id: LayoutId) {
        let node = &mut self.nodes[id.index()];
        let children = std::mem::take(&mut node.children);
        node.alive = false;
        node.parent = None;
        if node.pending != NOT_PENDING {
            self.saved[node.pending as usize] = None;
            node.pending = NOT_PENDING;
        }
        self.free.push(id.0);
        self.live -= 1;
        for child in children {
            self.free_subtree(child);
        }
    }

    /// Sets the ordered children of `parent`. A child that had another parent leaves it.
    pub fn set_children(&mut self, parent: LayoutId, children: &[LayoutId]) {
        if self.node(parent).children == children {
            return;
        }
        let old = std::mem::take(&mut self.nodes[parent.index()].children);
        for child in old {
            if !children.contains(&child) {
                self.nodes[child.index()].parent = None;
            }
        }
        for &child in children {
            let was = self.node(child).parent;
            if let Some(other) = was
                && other != parent
            {
                self.nodes[other.index()].children.retain(|&c| c != child);
                self.mark(other, false);
            }
            self.nodes[child.index()].parent = Some(parent);
        }
        self.nodes[parent.index()].children = children.to_vec();
        self.mark(parent, false);
    }

    /// Replaces a node's style. Nothing happens when it is equal.
    pub fn set_style(&mut self, id: LayoutId, style: Style) {
        let old = &self.node(id).style;
        if **old == style {
            return;
        }
        let item_changed = !old.same_as_item(&style);
        self.nodes[id.index()].style = Rc::new(style);
        self.mark(id, item_changed);
    }

    /// Says that what a leaf measures changed (its text, its image).
    pub fn mark_dirty(&mut self, id: LayoutId) {
        self.node(id);
        self.mark(id, false);
    }

    /// Forgets a node's answers and keeps them for the next update to compare with.
    fn mark(&mut self, id: LayoutId, item_changed: bool) {
        let node = &mut self.nodes[id.index()];
        if node.pending != NOT_PENDING {
            if item_changed && let Some(saved) = &mut self.saved[node.pending as usize] {
                saved.item_changed = true;
            }
            return;
        }
        // Never asked: no answer above depends on it, unless its parent must now count it.
        if node.answers.is_empty() && !item_changed {
            return;
        }
        node.pending = self.saved.len() as u32;
        self.saved.push(Some(Saved { id, answers: node.answers, item_changed }));
        node.answers = Answers::default();
    }

    pub(crate) fn node(&self, id: LayoutId) -> &Node {
        let node = &self.nodes[id.index()];
        assert!(node.alive, "a layout node was used after it was removed");
        node
    }

    pub fn style(&self, id: LayoutId) -> &Style {
        &self.node(id).style
    }

    pub fn children(&self, id: LayoutId) -> &[LayoutId] {
        &self.node(id).children
    }

    pub fn parent(&self, id: LayoutId) -> Option<LayoutId> {
        self.node(id).parent
    }

    /// The node's border box, relative to its parent's border box.
    pub fn rect(&self, id: LayoutId) -> Rect {
        self.node(id).rect
    }

    /// The extent of the node's children, for scrolling.
    pub fn content_size(&self, id: LayoutId) -> Size {
        self.node(id).content
    }

    /// The number of nodes in the tree.
    pub fn len(&self) -> usize {
        self.live
    }

    pub fn is_empty(&self) -> bool {
        self.live == 0
    }
}

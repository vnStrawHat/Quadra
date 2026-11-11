//! What is asked of a node after layout: its rect, its place in the window, its size as a
//! signal, its styles.

use quadra_prim::{Point, Rect, Size};
use quadra_reactive::{Signal, signal};

use crate::node::{NodeId, PaintStyle};
use crate::tree::Tree;

impl Tree {
    /// A signal of the node's size, for state that depends on it (how many rows of a list
    /// fit). It is written after the layout pass, and the frame then runs its bindings and
    /// lays out again: one more round, not a loop inside the pass (specs/04).
    pub fn size_signal(&mut self, id: NodeId) -> Signal<Size> {
        let size = self.rect(id).size();
        *self.sizes.entry(id.index).or_insert_with(|| signal(size))
    }

    /// The node's border box, relative to its parent's.
    pub fn rect(&self, id: NodeId) -> Rect {
        self.layout.rect(self.node(id).layout)
    }

    /// The node's border box in the window, in logical pixels: where it is drawn, with the
    /// offsets of the layers around it (scrolling) counted in.
    pub fn window_rect(&self, id: NodeId) -> Rect {
        let mut origin = Point::ZERO;
        let mut up = Some(id);
        while let Some(at) = up {
            let node = &self.nodes[at.index as usize];
            let offset = node.layer.as_ref().map_or(Point::ZERO, |layer| layer.offset);
            let place = if at == id { Point::ZERO } else { self.layout.rect(node.layout).origin() };
            origin = Point::new(origin.x + place.x + offset.x, origin.y + place.y + offset.y);
            up = node.parent;
        }
        self.rect(id).translate(origin.x, origin.y)
    }

    pub fn layout_style(&self, id: NodeId) -> &quadra_layout::Style {
        self.layout.style(self.node(id).layout)
    }

    pub fn paint_style(&self, id: NodeId) -> PaintStyle {
        self.node(id).paint
    }
}

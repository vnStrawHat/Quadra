//! What is asked of a node after layout: its rect, its place in the window, its size as a
//! signal, its styles.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_prim::{Point, Rect, Size};
use quadra_reactive::{Signal, batch, signal};

use crate::fonts::Fonts;
use crate::input::Handler;
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

    /// The fonts and the text system the tree lays text out with.
    pub fn fonts(&self) -> Rc<RefCell<Fonts>> {
        self.fonts.clone()
    }

    /// Runs `f` once, after the next layout pass: for what must see where nodes are before
    /// it can say where others go (a list that measures its rows). State it writes is seen
    /// by another round of the same frame.
    pub fn after_layout(&mut self, f: impl FnOnce(&mut Tree) + 'static) {
        self.after_layout.push(Box::new(f));
        self.shared.wake.set(true);
    }

    /// Makes a node that scrolls what it holds answer [`Tree::scroll_into_view`]: `f` is given
    /// a rect in the node's own box, as it is scrolled now, scrolls to show it, and returns
    /// how far it scrolled, sideways and down.
    pub fn on_reveal(&mut self, id: NodeId, f: impl FnMut(Rect) -> Point + 'static) {
        self.add_handler(id, Handler::Reveal(Box::new(f)));
    }

    /// Scrolls whatever scrolls around a node, the nearest first, so that the node shows: as
    /// little as that takes. The scrolling is seen in the next frame.
    pub fn scroll_into_view(&mut self, id: NodeId) {
        let whole = Rect::new(0.0, 0.0, f32::INFINITY, f32::INFINITY);
        self.scroll_rect_into_view(id, whole);
    }

    /// As [`Tree::scroll_into_view`], for a part of the node, given from its corner: the
    /// caret of a text of many lines.
    pub fn scroll_rect_into_view(&mut self, id: NodeId, part: Rect) {
        if !self.is_alive(id) {
            return;
        }
        let all = self.window_rect(id);
        let (width, height) = (part.width.min(all.width - part.x), part.height.min(all.height - part.y));
        let mut rect = Rect::new(all.x + part.x, all.y + part.y, width, height);
        batch(|| {
            for around in self.path(id).into_iter().skip(1) {
                let port = self.window_rect(around);
                let within = rect.translate(-port.x, -port.y);
                for handler in &mut self.nodes[around.index as usize].handlers {
                    if let Handler::Reveal(run) = handler {
                        let by = run(within);
                        rect = rect.translate(-by.x, -by.y); // it is further up and left now
                    }
                }
            }
        });
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

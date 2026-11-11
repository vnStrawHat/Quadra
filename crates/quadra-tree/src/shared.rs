//! What bindings and handlers hold of a tree: a queue of bindings that are due, and a queue
//! of writes (specs/03, "Batches"; specs/04, "Passes").
//!
//! A closure cannot hold the tree. It holds a [`Handle`]: writing a property through it queues
//! the write, and the tree applies the queue in its update pass.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use quadra_reactive::{Deferred, deferred};

use crate::node::{NodeId, PaintStyle};
use crate::tree::Tree;

pub(crate) enum Op {
    Text(NodeId, String),
    Layout(NodeId, Box<quadra_layout::Style>),
    Paint(NodeId, PaintStyle),
    /// Anything else: mounting, removing, a widget of the caller's own.
    With(Box<dyn FnOnce(&mut Tree)>),
}

#[derive(Default)]
pub(crate) struct Shared {
    pub due: RefCell<Vec<Deferred>>,
    pub ops: RefCell<Vec<Op>>,
    /// Set when something was queued: the window asks for a frame.
    pub wake: Cell<bool>,
}

/// A cheap, clonable way to reach a tree from a closure.
#[derive(Clone)]
pub struct Handle(pub(crate) Rc<Shared>);

impl Handle {
    fn push(&self, op: Op) {
        self.0.ops.borrow_mut().push(op);
        self.0.wake.set(true);
    }

    /// Sets the text of a text node. Nothing happens if it is equal, or the node is gone.
    pub fn set_text(&self, node: NodeId, text: String) {
        self.push(Op::Text(node, text));
    }

    pub fn set_layout_style(&self, node: NodeId, style: quadra_layout::Style) {
        self.push(Op::Layout(node, Box::new(style)));
    }

    pub fn set_paint_style(&self, node: NodeId, style: PaintStyle) {
        self.push(Op::Paint(node, style));
    }

    /// Runs `f` on the tree in its next update pass.
    pub fn with(&self, f: impl FnOnce(&mut Tree) + 'static) {
        self.push(Op::With(Box::new(f)));
    }

    /// Makes a binding: `f` runs now, and again in the tree's update pass after a signal it
    /// read changed. It belongs to the current scope and never runs once that is disposed.
    /// `f` reads signals and writes one property through a handle.
    pub fn bind(&self, f: impl FnMut() + 'static) -> Deferred {
        let me: Rc<Cell<Option<Deferred>>> = Rc::default();
        let (slot, shared) = (me.clone(), self.0.clone());
        let binding = deferred(
            move || {
                if let Some(binding) = slot.get() {
                    shared.due.borrow_mut().push(binding);
                    shared.wake.set(true);
                }
            },
            f,
        );
        me.set(Some(binding));
        binding
    }
}

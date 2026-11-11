//! What bindings and handlers hold of a tree: a queue of bindings that are due, and a queue
//! of writes (specs/03, "Batches"; specs/04, "Passes").
//!
//! A closure cannot hold the tree. It holds a [`Handle`]: writing a property through it queues
//! the write, and the tree applies the queue in its update pass.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

use quadra_reactive::{Deferred, deferred};

use crate::action::Action;
use crate::node::{NodeId, PaintStyle};
use crate::patch::Patch;
use crate::tree::Tree;

/// Something to do on the tree, once.
pub(crate) type OnTree = Box<dyn FnOnce(&mut Tree)>;

pub(crate) enum Op {
    Text(NodeId, String),
    Layout(NodeId, Box<quadra_layout::Style>),
    Paint(NodeId, PaintStyle),
    /// One value of a node's own style: what a reactive property writes.
    Patch(NodeId, Patch),
    /// Anything else: mounting, removing, a widget of the caller's own.
    With(OnTree),
}

#[derive(Default)]
pub(crate) struct Shared {
    pub due: RefCell<Vec<Deferred>>,
    pub ops: RefCell<Vec<Op>>,
    /// Run at the start of every frame, each until it returns false (specs/05, "Frame
    /// callbacks").
    pub frames: RefCell<Vec<Box<dyn FnMut() -> bool>>>,
    /// Set when something was queued: the window asks for a frame.
    pub wake: Cell<bool>,
    /// What was asked of the window, until the app takes it.
    pub asked: RefCell<Vec<crate::window::WindowRequest>>,
    /// The clock of the window the tree is in, when it was given one.
    pub clock: RefCell<Option<Rc<dyn Fn() -> Instant>>>,
}

impl Tree {
    /// Gives the tree the clock of its window: what animates goes by it, and a test moves
    /// it. Without one the tree goes by the machine's.
    pub fn set_clock(&mut self, now: impl Fn() -> Instant + 'static) {
        *self.shared.clock.borrow_mut() = Some(Rc::new(now));
    }
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

    /// Changes one value of a node's own style.
    pub fn patch_style(&self, node: NodeId, patch: Patch) {
        self.push(Op::Patch(node, patch));
    }

    /// The time on the clock of the tree's window.
    pub fn now(&self) -> Instant {
        let clock = self.0.clock.borrow().clone();
        clock.map_or_else(Instant::now, |now| now())
    }

    /// Runs `f` at the start of each frame, in the frame's batch, until it returns false.
    /// While one is registered the window draws every frame the display can take.
    pub fn each_frame(&self, f: impl FnMut() -> bool + 'static) {
        self.0.frames.borrow_mut().push(Box::new(f));
        self.0.wake.set(true);
    }

    /// Dispatches an action in the tree's next update pass, as a key binding would: what a
    /// button or a menu item does for its command.
    pub fn dispatch<A: Action>(&self, action: A) {
        self.with(move |tree| {
            tree.dispatch(&action);
        });
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

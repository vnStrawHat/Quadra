//! Effects: code that runs again when a signal it read changes.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::arena::Id;
use crate::graph;
use crate::runtime::{Compute, Kind, Loc, Node, State, Value, rt};

fn create(kind: Kind, mut f: impl FnMut() + 'static, created: Loc) -> Id {
    let compute: Compute = Rc::new(RefCell::new(move || {
        f();
        false
    }));
    let value: Value = Rc::new(RefCell::new(()));
    let id = rt(|r| r.add_node(Node::new(kind, State::Dirty, value, Some(compute), created)));
    graph::update(id);
    graph::flush(); // what the first run wrote
    id
}

/// Runs `f` now, and again when the outermost batch ends after a source changed. Owned by the
/// current scope: it never runs once that scope is disposed.
#[track_caller]
pub fn effect(f: impl FnMut() + 'static) {
    create(Kind::Effect, f, std::panic::Location::caller());
}

/// An effect its owner runs. This is what a binding is made of: the tree queues the effect when
/// it is told, and runs it in the next frame's update pass (specs/03, "Batches").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Deferred {
    id: Id,
    marker: PhantomData<*const ()>,
}

/// Runs `f` now. When a source changes, `notify` is called once; `f` runs again only when the
/// owner calls [`Deferred::run`], and only if a source really changed.
#[track_caller]
pub fn deferred(notify: impl Fn() + 'static, f: impl FnMut() + 'static) -> Deferred {
    let id = create(Kind::Deferred(Rc::new(notify)), f, std::panic::Location::caller());
    Deferred { id, marker: PhantomData }
}

impl Deferred {
    /// Brings the effect up to date. Does nothing if no source changed or it was disposed.
    pub fn run(self) {
        graph::update(self.id);
        graph::flush();
    }

    pub fn is_alive(self) -> bool {
        rt(|r| r.nodes.get(self.id).is_some())
    }
}

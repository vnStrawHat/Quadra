//! Memos: values computed from signals, cached, recomputed on demand.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::panic::Location;
use std::rc::Rc;

use crate::arena::Id;
use crate::graph;
use crate::runtime::{Compute, Kind, Node, State, disposed, rt};

/// A handle to a derived value. It is computed at its first read and again only when a source
/// changed; its own readers hear of it only if the value differs.
pub struct Memo<T> {
    id: Id,
    marker: PhantomData<*const T>,
}

impl<T> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Memo<T> {}

/// Creates a memo in the current scope. `f` must only read: it runs whenever a reader asks and
/// a source has changed.
#[track_caller]
pub fn memo<T: PartialEq + 'static>(f: impl Fn() -> T + 'static) -> Memo<T> {
    let slot: Rc<RefCell<Option<T>>> = Rc::new(RefCell::new(None));
    let out = slot.clone();
    let compute: Compute = Rc::new(RefCell::new(move || {
        let new = f();
        let mut current = out.borrow_mut();
        let differs = current.as_ref() != Some(&new);
        if differs {
            *current = Some(new);
        }
        differs
    }));
    let node = Node::new(Kind::Memo, State::Dirty, slot, Some(compute), Location::caller());
    Memo { id: rt(|r| r.add_node(node)), marker: PhantomData }
}

impl<T: 'static> Memo<T> {
    /// False once the scope that owns the memo was disposed.
    pub fn is_alive(self) -> bool {
        rt(|r| r.nodes.get(self.id).is_some())
    }

    /// Reads the value, computing it first if a source changed; `None` on a disposed memo.
    pub fn try_with<R>(self, f: impl FnOnce(&T) -> R) -> Option<R> {
        let cell = rt(|r| {
            let cell = r.nodes.get(self.id)?.value.clone();
            assert!(r.observer != Some(self.id), "a memo read itself");
            r.track(self.id);
            Some(cell)
        })?;
        graph::update(self.id);
        let value = cell.borrow();
        let value = value.downcast_ref::<Option<T>>().expect("a memo holds a value of its type");
        Some(f(value.as_ref().expect("a memo was read while it was being computed: it depends on itself")))
    }

    /// Reads the value. Panics on a disposed memo.
    #[track_caller]
    pub fn with<R>(self, f: impl FnOnce(&T) -> R) -> R {
        let at = Location::caller();
        self.try_with(f).unwrap_or_else(|| disposed("a memo", self.id, at))
    }

    #[track_caller]
    pub fn get(self) -> T
    where
        T: Clone,
    {
        self.with(T::clone)
    }

    pub fn try_get(self) -> Option<T>
    where
        T: Clone,
    {
        self.try_with(T::clone)
    }
}

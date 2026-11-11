//! Signals: values that record who reads them and tell them when they change.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::panic::Location;
use std::rc::Rc;

use crate::arena::Id;
use crate::graph;
use crate::runtime::{Kind, Loc, Node, State, Value, disposed, rt};

/// A handle to a value owned by the scope it was created in. `Copy`; not `Send`.
pub struct Signal<T> {
    id: Id,
    marker: PhantomData<*const T>,
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Signal<T> {}

/// Creates a signal in the current scope.
#[track_caller]
pub fn signal<T: 'static>(value: T) -> Signal<T> {
    let value: Value = Rc::new(RefCell::new(value));
    let node = Node::new(Kind::Signal, State::Clean, value, None, Location::caller());
    Signal { id: rt(|r| r.add_node(node)), marker: PhantomData }
}

fn read<T: 'static, R>(cell: &Value, f: impl FnOnce(&T) -> R) -> R {
    let Ok(value) = cell.try_borrow() else { panic!("a signal was read inside its own update") };
    f(value.downcast_ref::<T>().expect("a signal holds a value of its type"))
}

impl<T: 'static> Signal<T> {
    /// False once the scope that owns the signal was disposed.
    pub fn is_alive(self) -> bool {
        rt(|r| r.nodes.get(self.id).is_some())
    }

    fn cell(self, track: bool) -> Option<Value> {
        rt(|r| {
            let cell = r.nodes.get(self.id)?.value.clone();
            if track {
                r.track(self.id);
            }
            Some(cell)
        })
    }

    /// Reads the value; `None` on a disposed signal.
    pub fn try_with<R>(self, f: impl FnOnce(&T) -> R) -> Option<R> {
        Some(read(&self.cell(true)?, f))
    }

    /// Reads the value. Panics on a disposed signal.
    #[track_caller]
    pub fn with<R>(self, f: impl FnOnce(&T) -> R) -> R {
        let at = Location::caller();
        self.try_with(f).unwrap_or_else(|| disposed("a signal", self.id, at))
    }

    /// Reads without becoming a reader.
    #[track_caller]
    pub fn with_untracked<R>(self, f: impl FnOnce(&T) -> R) -> R {
        match self.cell(false) {
            Some(cell) => read(&cell, f),
            None => disposed("a signal", self.id, Location::caller()),
        }
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

    #[track_caller]
    pub fn get_untracked(self) -> T
    where
        T: Clone,
    {
        self.with_untracked(T::clone)
    }

    /// Applies `f`; readers are told when it returns true. False on a disposed signal.
    fn write(self, at: Loc, f: impl FnOnce(&mut T) -> bool) -> bool {
        let Some(cell) = self.cell(false) else {
            return false;
        };
        let changed = {
            let Ok(mut value) = cell.try_borrow_mut() else {
                panic!("a signal was written at {at} inside its own read or update")
            };
            f(value.downcast_mut::<T>().expect("a signal holds a value of its type"))
        };
        if changed {
            graph::changed(self.id, at);
        }
        true
    }

    /// Sets the value; nothing happens when it is equal to the current one.
    #[track_caller]
    pub fn set(self, value: T)
    where
        T: PartialEq,
    {
        if !self.try_set(value) {
            disposed("a signal", self.id, Location::caller());
        }
    }

    /// `set` that reports a disposed signal with `false` instead of a panic.
    #[track_caller]
    pub fn try_set(self, value: T) -> bool
    where
        T: PartialEq,
    {
        self.write(Location::caller(), |current| {
            let differs = *current != value;
            if differs {
                *current = value;
            }
            differs
        })
    }

    /// Sets the value and always tells readers: for types without equality.
    #[track_caller]
    pub fn set_always(self, value: T) {
        self.update(|current| *current = value);
    }

    /// Mutates in place and always tells readers: for values too large to compare.
    #[track_caller]
    pub fn update(self, f: impl FnOnce(&mut T)) {
        self.update_if(|value| {
            f(value);
            true
        });
    }

    /// Mutates in place; readers are told when `f` returns true.
    #[track_caller]
    pub fn update_if(self, f: impl FnOnce(&mut T) -> bool) {
        if !self.write(Location::caller(), f) {
            disposed("a signal", self.id, Location::caller());
        }
    }
}

/// A signal with no value: something to depend on and to notify.
#[derive(Clone, Copy)]
pub struct Trigger(Signal<()>);

#[track_caller]
pub fn trigger() -> Trigger {
    Trigger(signal(()))
}

impl Trigger {
    /// Makes the running memo or effect depend on this trigger.
    pub fn track(self) {
        self.0.try_with(|_| ());
    }

    #[track_caller]
    pub fn notify(self) {
        self.0.set_always(());
    }
}

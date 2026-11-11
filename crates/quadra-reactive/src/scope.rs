//! Scopes own what was created while they were current, and end as a unit. Context values are
//! found by walking the scope chain.

use std::any::{Any, TypeId, type_name};
use std::marker::PhantomData;
use std::rc::Rc;

use crate::arena::Id;
use crate::runtime::{ScopeData, rt};

/// A handle to a scope. The view layer makes one per mount boundary (specs/05).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Scope {
    id: Id,
    marker: PhantomData<*const ()>,
}

impl Scope {
    /// The scope new signals, effects and scopes are created in right now.
    pub fn current() -> Scope {
        Scope {
            id: rt(|r| r.scope),
            marker: PhantomData,
        }
    }

    /// Creates a scope owned by the current one. It becomes current only inside [`Scope::run`].
    pub fn child() -> Scope {
        let id = rt(|r| {
            r.scope_mut();
            let parent = r.scope;
            let id = r.scopes.insert(ScopeData {
                parent: Some(parent),
                ..ScopeData::default()
            });
            r.scope_mut().children.push(id);
            id
        });
        Scope {
            id,
            marker: PhantomData,
        }
    }

    /// Runs `f` with this scope current: what `f` creates belongs to it.
    pub fn run<R>(self, f: impl FnOnce() -> R) -> R {
        let outer = rt(|r| std::mem::replace(&mut r.scope, self.id));
        let out = f();
        rt(|r| r.scope = outer);
        out
    }

    pub fn is_alive(self) -> bool {
        rt(|r| r.scopes.get(self.id).is_some())
    }

    /// Ends the scope now: its child scopes first, then its cleanups, newest first, then
    /// everything it owns. Nothing it owned runs afterwards. Disposing twice does nothing.
    pub fn dispose(self) {
        dispose(self.id);
    }
}

fn dispose(id: Id) {
    // A cleanup may create a child or register another cleanup: repeat until none is left.
    loop {
        let work = rt(|r| {
            assert!(id != r.root, "the root scope cannot be disposed");
            let scope = r.scopes.get_mut(id)?;
            Some((
                std::mem::take(&mut scope.children),
                std::mem::take(&mut scope.cleanups),
            ))
        });
        let Some((children, cleanups)) = work else {
            return;
        };
        if children.is_empty() && cleanups.is_empty() {
            break;
        }
        for child in children.into_iter().rev() {
            dispose(child);
        }
        for cleanup in cleanups.into_iter().rev() {
            cleanup();
        }
    }
    // Dropped here, outside the runtime borrow: a value's Drop may use the runtime.
    drop(rt(|r| r.remove_scope(id)));
}

/// Runs `f` when the current scope is disposed.
pub fn on_cleanup(f: impl FnOnce() + 'static) {
    rt(|r| r.scope_mut().cleanups.push(Box::new(f)));
}

/// Makes `value` available to the current scope and everything below it. A second value of the
/// same type on the same scope replaces the first.
pub fn provide<T: 'static>(value: T) {
    let key = TypeId::of::<T>();
    let value: Rc<dyn Any> = Rc::new(value);
    let replaced = rt(|r| {
        let contexts = &mut r.scope_mut().contexts;
        match contexts.iter_mut().find(|c| c.0 == key) {
            Some(entry) => Some(std::mem::replace(&mut entry.1, value)),
            None => {
                contexts.push((key, value));
                None
            }
        }
    });
    drop(replaced);
}

/// The value of type `T` provided by the nearest scope, from the current one upwards.
pub fn use_context<T: Clone + 'static>() -> Option<T> {
    let key = TypeId::of::<T>();
    let found = rt(|r| {
        let mut next = Some(r.scope);
        while let Some(id) = next {
            let scope = r.scopes.get(id)?;
            if let Some(entry) = scope.contexts.iter().find(|c| c.0 == key) {
                return Some(entry.1.clone());
            }
            next = scope.parent;
        }
        None
    })?;
    found.downcast_ref::<T>().cloned()
}

/// [`use_context`] for a value that must be there.
#[track_caller]
pub fn expect<T: Clone + 'static>() -> T {
    match use_context::<T>() {
        Some(value) => value,
        None => panic!(
            "no {} was provided to this scope or one above it",
            type_name::<T>()
        ),
    }
}

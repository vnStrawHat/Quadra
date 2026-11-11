//! Events: for things that happen rather than values that are (specs/03, "Events").

use std::any::Any;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::arena::Id;
use crate::graph;
use crate::runtime::{EventData, Handler, rt};
use crate::scope::on_cleanup;

/// A handle to an event owned by the scope it was created in. `Copy`; not `Send`.
pub struct Event<E> {
    id: Id,
    marker: PhantomData<*const E>,
}

impl<E> Clone for Event<E> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<E> Copy for Event<E> {}

/// Creates an event in the current scope.
pub fn event<E: 'static>() -> Event<E> {
    let id = rt(|r| {
        r.scope_mut();
        let id = r.events.insert(EventData { subs: Vec::new() });
        r.scope_mut().events.push(id);
        id
    });
    Event {
        id,
        marker: PhantomData,
    }
}

impl<E: 'static> Event<E> {
    /// Calls `f` for every value emitted from now on, until the current scope is disposed.
    pub fn subscribe(self, mut f: impl FnMut(&E) + 'static) {
        let handler: Handler = Rc::new(RefCell::new(move |e: &dyn Any| {
            f(e.downcast_ref::<E>().expect("an event carries its type"))
        }));
        let id = self.id;
        let key = rt(|r| {
            r.seq += 1;
            if let Some(event) = r.events.get_mut(id) {
                event.subs.push((r.seq, handler));
            }
            r.seq
        });
        on_cleanup(move || {
            let gone = rt(|r| {
                let subs = &mut r.events.get_mut(id)?.subs;
                let at = subs.iter().position(|s| s.0 == key)?;
                Some(subs.remove(at))
            });
            drop(gone);
        });
    }

    /// Queues `value` for the subscribers. They are called when the outermost batch ends, in
    /// emission order, before effects run.
    pub fn emit(self, value: E) {
        let id = self.id;
        rt(|r| {
            r.deliveries
                .push_back(Box::new(move || deliver(id, &value)))
        });
        graph::flush();
    }
}

fn deliver(id: Id, value: &dyn Any) {
    let keys: Vec<u64> = rt(|r| {
        r.events
            .get(id)
            .map(|e| e.subs.iter().map(|s| s.0).collect())
            .unwrap_or_default()
    });
    for key in keys {
        // Looked up again each time: a handler may have ended a later subscriber's scope.
        let handler = rt(|r| {
            r.events
                .get(id)?
                .subs
                .iter()
                .find(|s| s.0 == key)
                .map(|s| s.1.clone())
        });
        if let Some(handler) = handler {
            (handler.borrow_mut())(value);
        }
    }
}

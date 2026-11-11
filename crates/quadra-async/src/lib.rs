//! Work over time (specs/11). So far: timers. They run on the platform's clock, which a test
//! moves by hand, and belong to the scope that made them.
//!
//! Local tasks, the background pool and inboxes join when the first thing needs them.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::time::Duration;

use quadra_platform::{Platform, TimerId};
use quadra_reactive::{Scope, batch, on_cleanup, use_context};

struct Entry {
    scope: Scope,
    repeat: bool,
    run: Box<dyn FnMut()>,
}

/// The timers of an app: what the platform's timers mean. An app has one, provides it as
/// context, and gives it the timers the platform fires.
pub struct Timers {
    platform: Rc<dyn Platform>,
    entries: RefCell<HashMap<TimerId, Entry>>,
    /// The timer whose callback is running, and whether that callback cancelled it.
    firing: Cell<Option<TimerId>>,
    cancelled: Cell<bool>,
}

/// A timer that was set. It ends with the scope it was set in; `cancel` ends it sooner.
#[derive(Clone)]
pub struct Timer {
    id: TimerId,
    timers: Weak<Timers>,
}

impl Timer {
    /// Stops the timer: its callback does not run again. Doing it twice does nothing.
    pub fn cancel(&self) {
        if let Some(timers) = self.timers.upgrade() {
            timers.cancel(self.id);
        }
    }
}

impl Timers {
    pub fn new(platform: Rc<dyn Platform>) -> Rc<Timers> {
        let (firing, cancelled) = (Cell::new(None), Cell::new(false));
        Rc::new(Timers { platform, entries: RefCell::new(HashMap::new()), firing, cancelled })
    }

    fn cancel(&self, id: TimerId) {
        // The entry is dropped after the borrow ends: what it owns may set or cancel timers.
        let entry = self.entries.borrow_mut().remove(&id);
        let running = self.firing.get() == Some(id) && !self.cancelled.replace(true);
        if entry.is_some() || running {
            self.platform.cancel_timer(id);
        }
    }

    fn set(self: &Rc<Timers>, after: Duration, repeat: bool, run: Box<dyn FnMut()>) -> Timer {
        let id = self.platform.set_timer(after, repeat);
        self.entries.borrow_mut().insert(id, Entry { scope: Scope::current(), repeat, run });
        let timer = Timer { id, timers: Rc::downgrade(self) };
        let ended = timer.clone();
        on_cleanup(move || ended.cancel());
        timer
    }

    /// A timer of the platform fired: runs what was set for it, in the scope that set it and
    /// in one batch. A timer of someone else's is not known here, and nothing happens.
    pub fn fire(&self, id: TimerId) {
        // Taken out while it runs: the callback may set or cancel timers, this one too.
        let Some(mut entry) = self.entries.borrow_mut().remove(&id) else { return };
        let outer = (self.firing.replace(Some(id)), self.cancelled.replace(false));
        batch(|| entry.scope.run(&mut entry.run));
        let cancelled = self.cancelled.replace(outer.1);
        self.firing.set(outer.0);
        // Still wanted: it repeats, and neither the callback nor its scope cancelled it.
        if entry.repeat && !cancelled && entry.scope.is_alive() {
            self.entries.borrow_mut().entry(id).or_insert(entry);
        }
    }
}

fn timers() -> Rc<Timers> {
    use_context::<Rc<Timers>>().expect("timers are set where an app, or a test harness, provides them")
}

/// Runs `f` once, after `after`, unless the scope it is set in ends first.
#[track_caller]
pub fn timeout(after: Duration, f: impl FnOnce() + 'static) -> Timer {
    let mut f = Some(f);
    timers().set(
        after,
        false,
        Box::new(move || {
            if let Some(f) = f.take() {
                f()
            }
        }),
    )
}

/// Runs `f` every `every`, until it is cancelled or the scope it is set in ends.
#[track_caller]
pub fn interval(every: Duration, f: impl FnMut() + 'static) -> Timer {
    timers().set(every, true, Box::new(f))
}

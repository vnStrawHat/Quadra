//! Notifications (specs/17, "The components"): what a window is told to show, in the order it
//! was told, each until it is closed or for as long as it lasts. The look gives the card of
//! each and the stacks they stand in.

use std::time::Duration;

use quadra_async::{Timer, timeout};
use quadra_reactive::{Scope, Signal, signal};

/// How long a toast shows.
pub const TOAST: Duration = Duration::from_secs(10);

/// What shows, each with the id it was given. Copies are the same one.
pub struct Queue<T: 'static> {
    shown: Signal<Vec<(u64, T)>>,
    last: Signal<u64>,
    owner: Scope,
    /// What lasts a while: how long, and the timer that closes it while nothing holds it.
    timed: Signal<Vec<(u64, Duration, Option<Timer>)>>,
}

impl<T> Clone for Queue<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Queue<T> {}

impl<T: Clone + 'static> Queue<T> {
    /// A queue of the scope it is made in: make it where the window's view is built.
    pub fn new() -> Queue<T> {
        let timed = signal(Vec::new());
        Queue { shown: signal(Vec::new()), last: signal(0), owner: Scope::current(), timed }
    }

    /// Shows `item` after those that show already, for `lasts` if it says, and returns what
    /// [`Queue::close`] takes.
    pub fn push(&self, item: T, lasts: Option<Duration>) -> u64 {
        let (id, me) = (self.last.get_untracked() + 1, *self);
        self.last.set(id);
        if let Some(time) = lasts.filter(|_| self.owner.is_alive()) {
            let timer = self.owner.run(|| timeout(time, move || me.close(id)));
            self.timed.update(|all| all.push((id, time, Some(timer))));
        }
        self.shown.update(|all| all.push((id, item)));
        id
    }

    /// Holds what lasts a while, or lets it go: while the pointer is on it, it waits, and
    /// when the pointer leaves it lasts as long again.
    pub fn hold(&self, id: u64, held: bool) {
        let me = *self;
        self.timed.update(|all| {
            let Some((_, time, timer)) = all.iter_mut().find(|(at, ..)| *at == id) else { return };
            if let Some(timer) = timer.take() {
                timer.cancel();
            }
            if !held && me.owner.is_alive() {
                let time = *time;
                *timer = Some(me.owner.run(|| timeout(time, move || me.close(id))));
            }
        });
    }

    pub fn close(&self, id: u64) {
        if self.shown.with_untracked(|all| all.iter().any(|(at, _)| *at == id)) {
            self.shown.update(|all| all.retain(|(at, _)| *at != id));
        }
        if self.timed.with_untracked(|all| all.iter().any(|(at, ..)| *at == id)) {
            self.timed.update(|all| all.retain(|(at, ..)| *at != id));
        }
    }

    /// What `id` stands for, while it shows.
    pub fn find(&self, id: u64) -> Option<T> {
        self.shown.with_untracked(|all| all.iter().find(|(at, _)| *at == id).map(|(_, item)| item.clone()))
    }

    /// The ids of what shows and `which` keeps, the oldest first: a signal's read.
    pub fn ids(&self, which: impl Fn(&T) -> bool) -> Vec<u64> {
        self.shown.with(|all| all.iter().filter(|(_, item)| which(item)).map(|(id, _)| *id).collect())
    }
}

impl<T: Clone + 'static> Default for Queue<T> {
    fn default() -> Self {
        Queue::new()
    }
}

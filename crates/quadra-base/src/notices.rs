//! Notifications (specs/17, "The components"): what a window is told to show, in the order it
//! was told, each until it is closed or for as long as it lasts. The look gives the card of
//! each and the stacks they stand in.

use std::time::Duration;

use quadra_async::timeout;
use quadra_reactive::{Scope, Signal, signal};

/// How long a toast shows.
pub const TOAST: Duration = Duration::from_secs(10);

/// What shows, each with the id it was given. Copies are the same one.
pub struct Queue<T: 'static> {
    shown: Signal<Vec<(u64, T)>>,
    last: Signal<u64>,
    owner: Scope,
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
        Queue { shown: signal(Vec::new()), last: signal(0), owner: Scope::current() }
    }

    /// Shows `item` after those that show already, for `lasts` if it says, and returns what
    /// [`Queue::close`] takes.
    pub fn push(&self, item: T, lasts: Option<Duration>) -> u64 {
        let (id, me) = (self.last.get_untracked() + 1, *self);
        self.last.set(id);
        if let Some(time) = lasts.filter(|_| self.owner.is_alive()) {
            self.owner.run(|| timeout(time, move || me.close(id)));
        }
        self.shown.update(|all| all.push((id, item)));
        id
    }

    pub fn close(&self, id: u64) {
        if self.shown.with_untracked(|all| all.iter().any(|(at, _)| *at == id)) {
            self.shown.update(|all| all.retain(|(at, _)| *at != id));
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

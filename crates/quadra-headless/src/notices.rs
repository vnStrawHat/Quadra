//! Notifications (specs/17, "The components"): what a window is told to show, in the order it
//! was told, each until it is closed or for as long as it lasts. The look gives the card of
//! each and the stacks they stand in.

use std::rc::Rc;
use std::time::Duration;

use quadra_async::{Timer, timeout};
use quadra_reactive::{Scope, Signal, signal};

/// How long a toast shows.
pub const TOAST: Duration = Duration::from_secs(10);

/// What is told of what went.
type Gone<T> = Rc<dyn Fn(T)>;

/// What shows, each with the id it was given. Copies are the same one.
pub struct Queue<T: 'static> {
    shown: Signal<Vec<(u64, T)>>,
    last: Signal<u64>,
    owner: Scope,
    /// What lasts a while: how long, and the timer that closes it while nothing holds it.
    timed: Signal<Vec<(u64, Duration, Option<Timer>)>>,
    /// Told what went, however it went (closed, timed out); not what was replaced.
    gone: Signal<Option<Gone<T>>>,
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
        let (shown, last, gone) = (signal(Vec::new()), signal(0), signal(None));
        Queue { shown, last, owner: Scope::current(), timed, gone }
    }

    /// Tells `f` what goes, however it goes, but for what [`Queue::replace`] puts another in
    /// the place of.
    pub fn on_gone(&self, f: impl Fn(T) + 'static) {
        self.gone.update(|it| *it = Some(Rc::new(f)));
    }

    /// Shows `item` after those that show already, for `lasts` if it says, and returns what
    /// [`Queue::close`] takes.
    pub fn push(&self, item: T, lasts: Option<Duration>) -> u64 {
        let id = self.timed_id(lasts);
        self.shown.update(|all| all.push((id, item)));
        id
    }

    /// Puts `item` in the place of `id`, as a new one, for `lasts` if it says, and returns its
    /// id; what was there goes untold. When `id` no longer shows, `item` comes after the rest.
    pub fn replace(&self, id: u64, item: T, lasts: Option<Duration>) -> u64 {
        let Some(at) = self.shown.with_untracked(|all| all.iter().position(|(it, _)| *it == id)) else {
            return self.push(item, lasts);
        };
        self.untime(id);
        let new = self.timed_id(lasts);
        self.shown.update(|all| all[at] = (new, item));
        new
    }

    /// A new id, timed to close after `lasts`.
    fn timed_id(&self, lasts: Option<Duration>) -> u64 {
        let (id, me) = (self.last.get_untracked() + 1, *self);
        self.last.set(id);
        if let Some(time) = lasts.filter(|_| self.owner.is_alive()) {
            let timer = self.owner.run(|| timeout(time, move || _ = me.close(id)));
            self.timed.update(|all| all.push((id, time, Some(timer))));
        }
        id
    }

    fn untime(&self, id: u64) {
        if self.timed.with_untracked(|all| all.iter().any(|(at, ..)| *at == id)) {
            self.timed.update(|all| {
                let at = all.iter().position(|(at, ..)| *at == id).unwrap();
                if let Some(timer) = all.remove(at).2 {
                    timer.cancel();
                }
            });
        }
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
                *timer = Some(me.owner.run(|| timeout(time, move || _ = me.close(id))));
            }
        });
    }

    /// Closes `id` and returns what it was, if it showed.
    pub fn close(&self, id: u64) -> Option<T> {
        self.untime(id);
        let at = self.shown.with_untracked(|all| all.iter().position(|(it, _)| *it == id))?;
        let mut gone = None;
        self.shown.update(|all| gone = Some(all.remove(at).1));
        let gone = gone?;
        if let Some(tell) = self.gone.with_untracked(Clone::clone) {
            tell(gone.clone());
        }
        Some(gone)
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

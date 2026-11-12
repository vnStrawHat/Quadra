//! Local tasks (specs/11): futures run on the thread of the windows, between events. A task
//! may hold signals, because it never leaves that thread; what wakes it may be anywhere.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Wake, Waker};

use quadra_platform::Platform;
use quadra_reactive::{Scope, batch, on_cleanup, use_context};

/// The tasks that were woken, filled from any thread.
struct Ready {
    ids: Mutex<Vec<u64>>,
    wake: quadra_platform::Waker,
}

/// What a future is given to wake its task with: a number and where to put it, so it can
/// go to any thread.
struct TaskWaker {
    id: u64,
    /// In the list already: a hundred wakes before the task runs are one run.
    queued: AtomicBool,
    ready: Arc<Ready>,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref()
    }

    fn wake_by_ref(self: &Arc<Self>) {
        if self.queued.swap(true, Ordering::AcqRel) {
            return;
        }
        let first = {
            let mut ids = self.ready.ids.lock().unwrap_or_else(PoisonError::into_inner);
            ids.push(self.id);
            ids.len() == 1
        };
        // The loop is woken for the first of a turn; the rest ride with it.
        if first {
            self.ready.wake.wake();
        }
    }
}

struct Entry {
    future: Pin<Box<dyn Future<Output = ()>>>,
    scope: Scope,
    waker: Arc<TaskWaker>,
}

/// The tasks of an app. An app has one, provides it as context (as its timers), and runs
/// what is ready after each event and when the platform says it was woken.
pub struct Executor {
    tasks: RefCell<HashMap<u64, Entry>>,
    /// The scopes that own a task, or did: one cleanup each.
    owners: RefCell<HashSet<Scope>>,
    next: Cell<u64>,
    ready: Arc<Ready>,
    /// The task that is running, and whether it was cancelled while it ran.
    running: Cell<Option<u64>>,
    cancelled: Cell<bool>,
}

impl Executor {
    pub fn new(platform: &dyn Platform) -> Rc<Executor> {
        Rc::new(Executor {
            tasks: RefCell::new(HashMap::new()),
            owners: RefCell::new(HashSet::new()),
            next: Cell::new(0),
            ready: Arc::new(Ready { ids: Mutex::new(Vec::new()), wake: platform.waker() }),
            running: Cell::new(None),
            cancelled: Cell::new(false),
        })
    }

    /// Runs the tasks that were woken, once each, in its scope and in a batch. What they
    /// wake waits for the next call, so events are never starved. Says whether any ran.
    pub fn run_ready(&self) -> bool {
        let ids = std::mem::take(&mut *self.ready.ids.lock().unwrap_or_else(PoisonError::into_inner));
        let mut ran = false;
        for id in ids {
            // Taken out while it runs: it may spawn and cancel tasks, itself among them.
            let Some(mut entry) = self.tasks.borrow_mut().remove(&id) else { continue };
            entry.waker.queued.store(false, Ordering::Release);
            let waker = Waker::from(entry.waker.clone());
            let outer = (self.running.replace(Some(id)), self.cancelled.replace(false));
            let scope = entry.scope;
            let done = batch(|| {
                scope.run(|| entry.future.as_mut().poll(&mut Context::from_waker(&waker)).is_ready())
            });
            let cancelled = self.cancelled.replace(outer.1);
            self.running.set(outer.0);
            ran = true;
            if !done && !cancelled && scope.is_alive() {
                self.tasks.borrow_mut().insert(id, entry);
            }
        }
        ran
    }

    /// How many tasks are neither done nor cancelled.
    pub fn len(&self) -> usize {
        self.tasks.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn add(self: &Rc<Executor>, future: Pin<Box<dyn Future<Output = ()>>>) -> u64 {
        let (id, scope) = (self.next.replace(self.next.get() + 1), Scope::current());
        let waker = Arc::new(TaskWaker { id, queued: AtomicBool::new(false), ready: self.ready.clone() });
        self.tasks.borrow_mut().insert(id, Entry { future, scope, waker: waker.clone() });
        if self.owners.borrow_mut().insert(scope) {
            let tasks = Rc::downgrade(self);
            on_cleanup(move || {
                if let Some(tasks) = tasks.upgrade() {
                    tasks.end(scope)
                }
            });
        }
        waker.wake(); // it starts between events, not inside the call that spawned it
        id
    }

    fn cancel(&self, id: u64) {
        // Dropped after the borrow ends: what a future owns may cancel other tasks.
        let entry = self.tasks.borrow_mut().remove(&id);
        if entry.is_none() && self.running.get() == Some(id) {
            self.cancelled.set(true);
        }
    }

    /// A scope ended: its tasks are dropped where they wait.
    fn end(&self, scope: Scope) {
        self.owners.borrow_mut().remove(&scope);
        let ids: Vec<u64> =
            self.tasks.borrow().iter().filter(|(_, e)| e.scope == scope).map(|(&id, _)| id).collect();
        for id in ids {
            self.cancel(id);
        }
    }
}

/// What a task leaves for who waits for it.
struct Slot<T> {
    value: RefCell<Option<T>>,
    waiter: RefCell<Option<Waker>>,
}

/// A task that was spawned. **Dropping it cancels the task**: its future is dropped where it
/// waits, with what it owns. [`Task::detach`] lets go of it without that. Awaiting it gives
/// what the future gave.
#[must_use = "dropping a task cancels it; `detach` leaves it to the scope it was spawned in"]
pub struct Task<T> {
    id: u64,
    executor: Weak<Executor>,
    slot: Rc<Slot<T>>,
    kept: bool,
}

impl<T> Task<T> {
    /// Leaves the task to the scope it was spawned in: it runs until it is done or that
    /// scope ends.
    pub fn detach(mut self) {
        self.kept = true;
    }

    /// Whether the future ran to its end and its value was not taken yet.
    pub fn is_done(&self) -> bool {
        self.slot.value.borrow().is_some()
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        if !self.kept
            && let Some(executor) = self.executor.upgrade()
        {
            executor.cancel(self.id);
        }
    }
}

impl<T> Future for Task<T> {
    type Output = T;

    /// A task whose scope ended is never done: who waits for it shares its scope, or its
    /// fate.
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        if let Some(value) = self.slot.value.borrow_mut().take() {
            return Poll::Ready(value);
        }
        self.slot.waiter.replace(Some(cx.waker().clone()));
        Poll::Pending
    }
}

/// Runs a future on this thread, between events, in the scope it is spawned in: when that
/// scope ends the future is dropped, so nothing of it runs afterwards.
#[track_caller]
pub fn spawn<T: 'static>(future: impl Future<Output = T> + 'static) -> Task<T> {
    let executor =
        use_context::<Rc<Executor>>().expect("tasks are spawned where an app, or a test harness, runs them");
    let slot = Rc::new(Slot { value: RefCell::new(None), waiter: RefCell::new(None) });
    let out = slot.clone();
    let id = executor.add(Box::pin(async move {
        let value = future.await;
        out.value.replace(Some(value));
        if let Some(waiter) = out.waiter.take() {
            waiter.wake();
        }
    }));
    Task { id, executor: Rc::downgrade(&executor), slot, kept: false }
}

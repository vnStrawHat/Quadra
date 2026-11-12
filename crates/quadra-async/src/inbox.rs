//! Inboxes (specs/11): what a thread produces as a stream (a pseudo terminal, a channel of
//! the network, the tail of a log) arrives on the thread of the windows in bursts, however
//! many pushes a burst was.

use std::future::poll_fn;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::task::{Poll, Waker};

use crate::task::spawn;

/// What a producer does when the inbox holds as much as it may.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Full {
    /// It waits until what is there was delivered.
    Block,
    /// What does not fit is lost.
    Drop,
}

struct State<T> {
    items: Vec<T>,
    finished: Option<i32>,
    /// The receiving end is gone.
    closed: bool,
    /// Who to wake for a delivery. None once it was woken: a burst is one wake.
    waker: Option<Waker>,
}

struct Shared<T> {
    state: Mutex<State<T>>,
    room: Condvar,
    limit: usize,
    full: Full,
}

fn lock<T>(shared: &Shared<T>) -> MutexGuard<'_, State<T>> {
    shared.state.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The receiving end, on the thread of the windows.
pub struct Inbox<T> {
    shared: Arc<Shared<T>>,
    finish: Option<Box<dyn FnOnce(i32)>>,
}

/// The producing end: it goes to any thread, and to several.
pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Sender<T> {
        Sender { shared: self.shared.clone() }
    }
}

/// An inbox that holds whatever is pushed.
pub fn inbox<T: Send + 'static>() -> Inbox<T> {
    inbox_with(usize::MAX, Full::Drop)
}

/// An inbox that holds `limit` items at most between two deliveries.
pub fn inbox_with<T: Send + 'static>(limit: usize, full: Full) -> Inbox<T> {
    let state = State { items: Vec::new(), finished: None, closed: false, waker: None };
    let shared = Shared { state: Mutex::new(state), room: Condvar::new(), limit: limit.max(1), full };
    Inbox { shared: Arc::new(shared), finish: None }
}

impl<T: Send + 'static> Sender<T> {
    /// Adds an item. False when it will never be delivered: the receiving end is gone, or
    /// the stream was finished.
    pub fn push(&self, item: T) -> bool {
        self.put(|items, room| {
            if room > 0 {
                items.push(item);
            }
        })
    }

    /// Adds a run of items (the bytes a read gave) with one lock.
    pub fn extend(&self, mut run: &[T]) -> bool
    where
        T: Clone,
    {
        while !run.is_empty() {
            let open = self.put(|items, room| {
                let take = room.min(run.len());
                items.extend_from_slice(&run[..take]);
                run = &run[take..];
            });
            if !open {
                return false;
            }
            if self.shared.full == Full::Drop {
                break; // what did not fit is lost
            }
        }
        true
    }

    /// Ends the stream: `code` is delivered after the last data, and nothing after it.
    pub fn finish(&self, code: i32) {
        let waker = {
            let mut state = lock(&self.shared);
            if state.finished.is_some() || state.closed {
                return;
            }
            state.finished = Some(code);
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }

    /// Whether a push would be delivered no more.
    pub fn is_closed(&self) -> bool {
        let state = lock(&self.shared);
        state.closed || state.finished.is_some()
    }

    /// Runs `add` with the items and the room left, under the lock, waiting for room first
    /// if the inbox blocks; then wakes the receiving end if no wake is on its way.
    fn put(&self, add: impl FnOnce(&mut Vec<T>, usize)) -> bool {
        let shared = &*self.shared;
        let waker = {
            let mut state = lock(shared);
            while shared.full == Full::Block && state.items.len() >= shared.limit && !state.closed {
                state = shared.room.wait(state).unwrap_or_else(PoisonError::into_inner);
            }
            if state.closed || state.finished.is_some() {
                return false;
            }
            let room = shared.limit.saturating_sub(state.items.len());
            add(&mut state.items, room);
            state.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
        true
    }
}

impl<T: Send + 'static> Inbox<T> {
    pub fn sender(&self) -> Sender<T> {
        Sender { shared: self.shared.clone() }
    }

    /// What to do with the code the producer finished with, after the last data.
    pub fn on_finish(mut self, f: impl FnOnce(i32) + 'static) -> Inbox<T> {
        self.finish = Some(Box::new(f));
        self
    }

    /// Delivers what is pushed to `f`, a burst at a time, in the scope this is called in and
    /// until that scope ends. `f` may take the items out of the vector; what it leaves is
    /// cleared, and the vector goes back to be filled, so a steady stream allocates nothing.
    #[track_caller]
    pub fn on_data(mut self, mut f: impl FnMut(&mut Vec<T>) + 'static) {
        spawn(async move {
            let mut burst = Vec::new();
            loop {
                let end = poll_fn(|cx| {
                    let mut state = lock(&self.shared);
                    if !state.items.is_empty() {
                        std::mem::swap(&mut state.items, &mut burst);
                        self.shared.room.notify_all();
                        return Poll::Ready(None);
                    }
                    if let Some(code) = state.finished {
                        return Poll::Ready(Some(code));
                    }
                    state.waker = Some(cx.waker().clone());
                    Poll::Pending
                })
                .await;
                match end {
                    None => {
                        f(&mut burst);
                        burst.clear();
                    }
                    Some(code) => {
                        if let Some(finish) = self.finish.take() {
                            finish(code);
                        }
                        return;
                    }
                }
            }
        })
        .detach();
    }
}

impl<T> Drop for Inbox<T> {
    /// The receiving end is gone, with its scope or unused: producers are told, and those
    /// that wait for room go on.
    fn drop(&mut self) {
        let mut state = lock(&self.shared);
        state.closed = true;
        state.items.clear();
        self.shared.room.notify_all();
    }
}

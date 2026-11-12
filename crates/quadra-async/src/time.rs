//! Time for tasks and signals (specs/11): `sleep` in a task, and signals that follow another
//! at a slower pace.

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use quadra_reactive::{Scope, Signal, effect, signal, untrack};

use crate::{Timer, interval, timeout};

#[derive(Default)]
struct Woken {
    done: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

/// A wait in a task. It starts at its first poll and its timer ends with it.
#[must_use = "a sleep waits where it is awaited"]
pub struct Sleep {
    after: Duration,
    set: Option<(Timer, Rc<Woken>)>,
}

/// `sleep(duration).await` goes on after `duration` of the platform's clock.
pub fn sleep(after: Duration) -> Sleep {
    Sleep { after, set: None }
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if let Some((_, woken)) = &self.set {
            if woken.done.get() {
                return Poll::Ready(());
            }
            woken.waker.replace(Some(cx.waker().clone()));
            return Poll::Pending;
        }
        let woken = Rc::new(Woken { done: Cell::new(false), waker: RefCell::new(Some(cx.waker().clone())) });
        let fired = woken.clone();
        let timer = timeout(self.after, move || {
            fired.done.set(true);
            if let Some(waker) = fired.waker.take() {
                waker.wake();
            }
        });
        self.set = Some((timer, woken));
        Poll::Pending
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        if let Some((timer, _)) = &self.set {
            timer.cancel();
        }
    }
}

/// A signal that takes the value of `source` once it has stood still for `wait`: what a
/// search does with what is typed.
#[track_caller]
pub fn debounce<T: Clone + PartialEq + 'static>(source: Signal<T>, wait: Duration) -> Signal<T> {
    let (out, scope) = (signal(source.get_untracked()), Scope::current());
    let pending: Rc<RefCell<Option<Timer>>> = Rc::default();
    effect(move || {
        let value = source.get();
        if let Some(timer) = pending.take() {
            timer.cancel();
        }
        if out.with_untracked(|o| *o == value) {
            return;
        }
        let fired = pending.clone();
        let timer = scope.run(|| {
            timeout(wait, move || {
                fired.take();
                out.set(value);
            })
        });
        pending.replace(Some(timer));
    });
    out
}

/// A signal that follows `source` at once and then no more often than every `every`, with
/// the latest value at each step: what a view of fast data does.
#[track_caller]
pub fn throttle<T: Clone + PartialEq + 'static>(source: Signal<T>, every: Duration) -> Signal<T> {
    let (out, scope) = (signal(source.get_untracked()), Scope::current());
    let running: Rc<RefCell<Option<Timer>>> = Rc::default();
    effect(move || {
        let value = source.get();
        // While the timer runs, its tick takes the latest.
        if running.borrow().is_some() || out.with_untracked(|o| *o == value) {
            return;
        }
        untrack(|| out.set(value));
        let ticking = running.clone();
        let timer = scope.run(|| {
            interval(every, move || {
                let latest = source.get_untracked();
                if out.with_untracked(|o| *o == latest) {
                    if let Some(timer) = ticking.take() {
                        timer.cancel();
                    }
                } else {
                    out.set(latest);
                }
            })
        });
        running.replace(Some(timer));
    });
    out
}

//! Timers in virtual time (specs/11, Tests): they fire in order on the platform's clock, end
//! with their scope, and can be cancelled, from inside their own callback too.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use quadra_async::{Timers, interval, timeout};
use quadra_platform::{HeadlessPlatform, PlatformEvent, PlatformHandler, TimerId, WindowId};
use quadra_reactive::{Scope, provide, signal};

/// What an app does with the platform's timers: gives them to its `Timers`.
struct App(Rc<Timers>);

impl PlatformHandler for App {
    fn event(&mut self, _: WindowId, _: PlatformEvent) {}
    fn frame(&mut self, _: WindowId) {}
    fn timer(&mut self, id: TimerId) {
        self.0.fire(id);
    }
    fn wake(&mut self) {}
}

fn setup() -> (HeadlessPlatform, App, Scope) {
    let platform = HeadlessPlatform::new();
    let timers = Timers::new(Rc::new(platform.clone()));
    let scope = Scope::child();
    scope.run(|| provide(timers.clone()));
    (platform, App(timers), scope)
}

const MS: Duration = Duration::from_millis(1);

#[test]
fn timers_fire_in_order_once_or_again_and_again() {
    let (platform, mut app, scope) = setup();
    let log: Rc<RefCell<Vec<&str>>> = Rc::default();
    let (a, b, c) = (log.clone(), log.clone(), log.clone());
    scope.run(|| {
        timeout(30 * MS, move || a.borrow_mut().push("late"));
        timeout(10 * MS, move || b.borrow_mut().push("soon"));
        interval(20 * MS, move || c.borrow_mut().push("tick"));
    });
    platform.advance(5 * MS, &mut app);
    assert!(log.borrow().is_empty());
    platform.advance(60 * MS, &mut app);
    assert_eq!(*log.borrow(), ["soon", "tick", "late", "tick", "tick"]);
}

#[test]
fn a_timer_ends_with_its_scope_or_when_it_is_cancelled() {
    let (platform, mut app, scope) = setup();
    let count = signal(0);
    let inner = scope.run(Scope::child);
    let ticking = inner.run(|| interval(10 * MS, move || count.update(|n| *n += 1)));
    let once = scope.run(|| timeout(15 * MS, move || count.update(|n| *n += 100)));
    platform.advance(25 * MS, &mut app);
    assert_eq!(count.get_untracked(), 102);
    inner.dispose(); // the interval was its
    platform.advance(50 * MS, &mut app);
    assert_eq!(count.get_untracked(), 102);
    ticking.cancel(); // again, and of a timer that is over: nothing
    once.cancel();

    let cancelled = scope.run(|| timeout(10 * MS, move || count.set(-1)));
    cancelled.cancel();
    platform.advance(50 * MS, &mut app);
    assert_eq!(count.get_untracked(), 102);
}

#[test]
fn a_callback_may_cancel_its_own_timer_and_set_others() {
    let (platform, mut app, scope) = setup();
    let count = signal(0);
    let own: Rc<RefCell<Option<quadra_async::Timer>>> = Rc::default();
    let handle = own.clone();
    let timer = scope.run(|| {
        interval(10 * MS, move || {
            count.update(|n| *n += 1);
            if count.get_untracked() == 3 {
                handle.borrow().as_ref().unwrap().cancel(); // stops itself
                timeout(5 * MS, move || count.update(|n| *n += 10)); // and sets another
            }
        })
    });
    *own.borrow_mut() = Some(timer);
    platform.advance(100 * MS, &mut app);
    assert_eq!(count.get_untracked(), 13);
}

//! Propagation: what runs, how often, and in what state it finds the world (specs/03, Tests).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use quadra_reactive::{Memo, batch, effect, memo, signal, trigger, untrack};

/// A counter and a closure that bumps it.
fn counter() -> (Rc<Cell<u32>>, impl Fn() + Clone + 'static) {
    let count = Rc::new(Cell::new(0));
    let bump = count.clone();
    (count, move || bump.set(bump.get() + 1))
}

#[test]
fn an_effect_runs_at_creation_and_once_per_change() {
    let (runs, ran) = counter();
    let n = signal(1);
    let seen = Rc::new(Cell::new(0));
    let out = seen.clone();
    effect(move || {
        ran();
        out.set(n.get());
    });
    assert_eq!((runs.get(), seen.get()), (1, 1));
    n.set(5);
    assert_eq!((runs.get(), seen.get()), (2, 5));
}

#[test]
fn equal_writes_run_nothing() {
    let (runs, ran) = counter();
    let n = signal(7);
    effect(move || {
        n.get();
        ran();
    });
    n.set(7);
    n.update_if(|_| false);
    assert!(n.try_set(7));
    assert_eq!(runs.get(), 1);
    n.set_always(7); // for types without equality: always tells
    n.update(|_| {});
    assert_eq!(runs.get(), 3);
}

#[test]
fn a_diamond_runs_each_memo_once_and_shows_no_glitch() {
    let (b_runs, b_ran) = counter();
    let (c_runs, c_ran) = counter();
    let a = signal(1);
    let b = memo(move || {
        b_ran();
        a.get() + 1
    });
    let c = memo(move || {
        c_ran();
        a.get() * 10
    });
    let seen = Rc::new(RefCell::new(Vec::new()));
    let out = seen.clone();
    effect(move || out.borrow_mut().push((b.get(), c.get())));
    batch(|| {
        a.set(2);
        a.set(3);
    });
    // Never (3, 30) with a half-updated pair, and one run for two writes.
    assert_eq!(*seen.borrow(), [(2, 10), (4, 30)]);
    assert_eq!((b_runs.get(), c_runs.get()), (2, 2));
}

#[test]
fn an_unchanged_memo_stops_the_wave() {
    let (memo_runs, memo_ran) = counter();
    let (effect_runs, effect_ran) = counter();
    let n = signal(1);
    let odd = memo(move || {
        memo_ran();
        n.get() % 2 == 1
    });
    let outer = memo(move || !odd.get());
    effect(move || {
        outer.get();
        effect_ran();
    });
    n.set(3);
    assert_eq!((memo_runs.get(), effect_runs.get()), (2, 1));
    n.set(4);
    assert_eq!((memo_runs.get(), effect_runs.get()), (3, 2));
}

#[test]
fn memos_are_lazy() {
    let (runs, ran) = counter();
    let n = signal(1);
    let double = memo(move || {
        ran();
        n.get() * 2
    });
    assert_eq!(runs.get(), 0);
    n.set(2);
    n.set(3);
    assert_eq!(runs.get(), 0);
    assert_eq!((double.get(), double.get(), runs.get()), (6, 6, 1));
}

#[test]
fn dependencies_follow_branches() {
    let (runs, ran) = counter();
    let (flag, a, b) = (signal(true), signal(1), signal(10));
    effect(move || {
        ran();
        if flag.get() {
            a.get()
        } else {
            b.get()
        };
    });
    b.set(11); // not read on this branch
    assert_eq!(runs.get(), 1);
    flag.set(false);
    assert_eq!(runs.get(), 2);
    a.set(2); // no longer read
    assert_eq!(runs.get(), 2);
    b.set(12);
    assert_eq!(runs.get(), 3);
}

#[test]
fn a_source_read_twice_is_one_dependency() {
    let (runs, ran) = counter();
    let (a, b) = (signal(1), signal(1));
    effect(move || {
        ran();
        a.get();
        b.get();
        a.get();
    });
    a.set(2);
    b.set(2);
    a.set(3);
    assert_eq!(runs.get(), 4);
}

#[test]
fn untracked_reads_make_no_dependency() {
    let (runs, ran) = counter();
    let (a, b, c) = (signal(1), signal(1), signal(1));
    effect(move || {
        ran();
        a.get();
        b.get_untracked();
        untrack(|| c.get());
    });
    b.set(2);
    c.set(2);
    assert_eq!(runs.get(), 1);
    a.set(2);
    assert_eq!(runs.get(), 2);
}

#[test]
fn an_effect_that_writes_what_it_reads_runs_again() {
    let (runs, ran) = counter();
    let n = signal(0);
    effect(move || {
        ran();
        if n.get() < 3 {
            n.update(|v| *v += 1);
        }
    });
    assert_eq!((n.get(), runs.get()), (3, 4));
}

#[test]
fn a_write_during_a_run_reaches_a_source_first_read_in_that_run() {
    let (go, base) = (signal(false), signal(1));
    let double = memo(move || base.get() * 2);
    let seen = Rc::new(Cell::new(0));
    let out = seen.clone();
    effect(move || {
        if go.get() {
            out.set(double.get()); // first read of the memo
            base.set(5); // equal on the second run, so it settles
        }
    });
    go.set(true);
    assert_eq!(seen.get(), 10);
}

#[test]
fn a_trigger_wakes_its_readers() {
    let (runs, ran) = counter();
    let changed = trigger();
    effect(move || {
        changed.track();
        ran();
    });
    changed.notify();
    changed.notify();
    assert_eq!(runs.get(), 3);
}

#[test]
#[should_panic(expected = "inside its own read or update")]
fn writing_a_signal_inside_its_own_read_panics() {
    let n = signal(1);
    n.with(|_| n.set(2));
}

#[test]
#[should_panic(expected = "a memo read itself")]
fn a_memo_reading_itself_panics() {
    let me: Rc<Cell<Option<Memo<i32>>>> = Rc::new(Cell::new(None));
    let inner = me.clone();
    let m = memo(move || inner.get().map_or(0, |m| m.get()) + 1);
    me.set(Some(m));
    m.get();
}

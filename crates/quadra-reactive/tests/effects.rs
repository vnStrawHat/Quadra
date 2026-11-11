//! Batches, cycles, deferred effects and events (specs/03, "Batches", "Events").

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use quadra_reactive::{ReactiveError, Scope, batch, deferred, effect, event, set_error_hook, signal};

fn log() -> (Rc<RefCell<Vec<String>>>, impl Fn(&str) + Clone + 'static) {
    let lines = Rc::new(RefCell::new(Vec::new()));
    let out = lines.clone();
    (lines, move |s: &str| out.borrow_mut().push(s.to_string()))
}

#[test]
fn a_batch_runs_effects_once_when_it_ends() {
    let (lines, say) = log();
    let (a, b) = (signal(1), signal(1));
    effect(move || say(&format!("{}+{}", a.get(), b.get())));
    batch(|| {
        a.set(2);
        batch(|| b.set(2)); // the outermost batch does the work
        assert_eq!(lines.borrow().len(), 1);
    });
    assert_eq!(*lines.borrow(), ["1+1", "2+2"]);
}

#[test]
fn effects_run_in_creation_order() {
    let (lines, say) = log();
    let n = signal(0);
    for name in ["first", "second", "third"] {
        let say = say.clone();
        effect(move || {
            n.get();
            say(name);
        });
    }
    lines.borrow_mut().clear();
    n.set(1);
    assert_eq!(*lines.borrow(), ["first", "second", "third"]);
}

#[test]
fn a_cycle_is_reported_and_stopped() {
    let errors = Rc::new(RefCell::new(Vec::new()));
    let out = errors.clone();
    set_error_hook(move |e| out.borrow_mut().push(e.to_string()));
    let n = signal(0u32);
    effect(move || n.set(n.get() + 1));
    assert_eq!(errors.borrow().len(), 1);
    assert!(errors.borrow()[0].contains("effects.rs"), "{}", errors.borrow()[0]);
    let stopped_at = n.get();
    assert!(stopped_at > 60 && stopped_at < 70, "{stopped_at}");
    // The runtime still works, and the effect can be woken again in a later batch.
    let other = signal(1);
    other.set(2);
    assert_eq!(other.get(), 2);
    let _: fn(&ReactiveError) = |_| {};
}

#[test]
fn a_deferred_effect_is_told_once_and_runs_when_asked() {
    let told = Rc::new(Cell::new(0));
    let ran = Rc::new(Cell::new(0));
    let (t, r) = (told.clone(), ran.clone());
    let n = signal(1);
    let binding = deferred(
        move || t.set(t.get() + 1),
        move || {
            n.get();
            r.set(r.get() + 1);
        },
    );
    assert_eq!((told.get(), ran.get()), (0, 1)); // the body ran at creation
    n.set(2);
    n.set(3);
    assert_eq!((told.get(), ran.get()), (1, 1)); // told once, not run
    binding.run();
    binding.run();
    assert_eq!((told.get(), ran.get()), (1, 2));
    n.set(4);
    assert_eq!(told.get(), 2);
}

#[test]
fn a_deferred_effect_behind_an_unchanged_memo_does_not_run() {
    let ran = Rc::new(Cell::new(0));
    let r = ran.clone();
    let n = signal(1);
    let sign = quadra_reactive::memo(move || n.get() > 0);
    let binding = deferred(
        || {},
        move || {
            sign.get();
            r.set(r.get() + 1);
        },
    );
    n.set(2);
    binding.run();
    assert_eq!(ran.get(), 1);
}

#[test]
fn events_are_delivered_when_the_batch_ends_in_order() {
    let (lines, say) = log();
    let named = event::<&'static str>();
    let n = signal(0);
    let on_event = say.clone();
    named.subscribe(move |name| on_event(name));
    effect(move || say(&format!("n={}", n.get())));
    lines.borrow_mut().clear();
    batch(|| {
        named.emit("a");
        n.set(1);
        named.emit("b");
        assert!(lines.borrow().is_empty());
    });
    assert_eq!(*lines.borrow(), ["a", "b", "n=1"]); // events first, then effects
}

#[test]
fn an_event_emitted_by_a_handler_is_delivered_after_it() {
    let (lines, say) = log();
    let ping = event::<u32>();
    ping.subscribe(move |n| {
        say(&format!("got {n}"));
        if *n < 2 {
            ping.emit(n + 1);
        }
        say(&format!("done {n}"));
    });
    ping.emit(0);
    assert_eq!(*lines.borrow(), ["got 0", "done 0", "got 1", "done 1", "got 2", "done 2"]);
}

#[test]
fn a_subscriber_whose_scope_ended_during_delivery_is_not_called() {
    let (lines, say) = log();
    let fired = event::<()>();
    let later = Scope::child();
    let first = say.clone();
    fired.subscribe(move |_| {
        first("first");
        later.dispose();
    });
    later.run(|| fired.subscribe(move |_| say("second")));
    fired.emit(());
    fired.emit(());
    assert_eq!(*lines.borrow(), ["first", "first"]);
}

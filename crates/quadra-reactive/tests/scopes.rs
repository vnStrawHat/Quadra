//! Scopes, disposal and context (specs/03, "Scopes", "Context").

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use quadra_reactive::{
    Scope, deferred, effect, event, expect, memo, on_cleanup, provide, signal, stats, use_context,
};

#[test]
fn disposing_a_scope_frees_everything_it_owns() {
    let before = stats();
    let outside = signal(1);
    let scope = Scope::child();
    scope.run(|| {
        let local = signal(1);
        let sum = memo(move || outside.get() + local.get());
        effect(move || {
            sum.get();
        });
        event::<u32>().subscribe(|_| {});
        Scope::child().run(|| {
            effect(move || {
                outside.get();
            });
        });
    });
    assert!(stats().nodes > before.nodes + 1 && stats().scopes == before.scopes + 2);
    scope.dispose();
    scope.dispose(); // twice does nothing
    let after = stats();
    assert_eq!((after.nodes, after.scopes, after.events), (before.nodes + 1, before.scopes, before.events));
    assert!(!scope.is_alive() && outside.is_alive());
    outside.set(2); // its dead readers are gone: nothing runs, nothing panics
}

#[test]
fn nothing_a_disposed_scope_owned_runs_again() {
    let runs = Rc::new(Cell::new(0));
    let n = signal(1);
    let ping = event::<()>();
    let scope = Scope::child();
    let binding = scope.run(|| {
        let (a, b, c) = (runs.clone(), runs.clone(), runs.clone());
        effect(move || {
            n.get();
            a.set(a.get() + 1);
        });
        ping.subscribe(move |_| b.set(b.get() + 1));
        deferred(
            || {},
            move || {
                n.get();
                c.set(c.get() + 1);
            },
        )
    });
    assert_eq!(runs.get(), 2);
    scope.dispose();
    n.set(2);
    ping.emit(());
    binding.run();
    assert_eq!(runs.get(), 2);
    assert!(!binding.is_alive());
}

#[test]
fn cleanups_run_children_first_then_newest_first() {
    let order = Rc::new(RefCell::new(Vec::new()));
    let push = |name: &'static str| {
        let order = order.clone();
        on_cleanup(move || order.borrow_mut().push(name));
    };
    let scope = Scope::child();
    scope.run(|| {
        push("outer 1");
        Scope::child().run(|| push("inner"));
        push("outer 2");
    });
    scope.dispose();
    assert_eq!(*order.borrow(), ["inner", "outer 2", "outer 1"]);
}

#[test]
fn a_value_dropped_by_disposal_may_use_the_runtime() {
    struct Noisy(quadra_reactive::Signal<u32>);
    impl Drop for Noisy {
        fn drop(&mut self) {
            self.0.update(|n| *n += 1);
        }
    }
    let drops = signal(0);
    let scope = Scope::child();
    scope.run(|| {
        signal(Noisy(drops));
        provide(Rc::new(Noisy(drops)));
    });
    scope.dispose();
    assert_eq!(drops.get(), 2);
}

#[test]
fn a_disposed_handle_reports_instead_of_reaching_another_value() {
    let scope = Scope::child();
    let (n, double) = scope.run(|| {
        let n = signal(1);
        (n, memo(move || n.get() * 2))
    });
    scope.dispose();
    let reuse = signal(99); // takes a freed slot
    assert_eq!((n.try_get(), double.try_get(), n.try_set(5)), (None, None, false));
    assert!(!n.is_alive() && !double.is_alive() && reuse.get() == 99);
}

#[test]
#[should_panic(expected = "after it was disposed (created at")]
fn reading_a_disposed_signal_panics_naming_where_it_was_created() {
    let scope = Scope::child();
    let n = scope.run(|| signal(1));
    scope.dispose();
    n.get();
}

#[test]
#[should_panic(expected = "the current scope was disposed")]
fn nothing_can_be_created_in_a_disposed_scope() {
    let scope = Scope::child();
    scope.run(|| {
        scope.dispose();
        signal(1);
    });
}

#[test]
fn context_comes_from_the_nearest_scope() {
    #[derive(Clone, PartialEq, Debug)]
    struct Theme(&'static str);
    assert_eq!(use_context::<Theme>(), None);
    provide(Theme("light"));
    provide(Theme("dark")); // replaces
    let inner = Scope::child();
    inner.run(|| {
        assert_eq!(expect::<Theme>(), Theme("dark"));
        provide(Theme("panel"));
        Scope::child().run(|| assert_eq!(expect::<Theme>(), Theme("panel")));
        assert_eq!(use_context::<u32>(), None);
    });
    assert_eq!(expect::<Theme>(), Theme("dark"));
    assert_eq!(Scope::current(), Scope::current());
}

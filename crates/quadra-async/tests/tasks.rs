//! Tasks, the pool and inboxes (specs/11, Tests): a task ends when it is dropped or its scope
//! is, and nothing of it runs afterwards; the pool has its bound and turns a panic into an
//! error; an inbox delivers a burst once and keeps to its limit; `sleep` goes by the
//! platform's clock.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use quadra_async::{
    Executor, Full, Timers, background, background_with, inbox, inbox_with, pool_threads, sleep, spawn,
    wait_idle,
};
use quadra_platform::{HeadlessPlatform, PlatformEvent, PlatformHandler, TimerId, WindowId};
use quadra_reactive::{Scope, provide};

/// What an app does: timers to its `Timers`, and its tasks run after each thing delivered.
struct App(Rc<Timers>, Rc<Executor>);

impl PlatformHandler for App {
    fn event(&mut self, _: WindowId, _: PlatformEvent) {}
    fn frame(&mut self, _: WindowId) {}
    fn timer(&mut self, id: TimerId) {
        self.0.fire(id);
        self.1.run_ready();
    }
    fn wake(&mut self) {
        self.1.run_ready();
    }
}

fn setup() -> (HeadlessPlatform, App, Scope) {
    let platform = HeadlessPlatform::new();
    let timers = Timers::new(Rc::new(platform.clone()));
    let tasks = Executor::new(&platform);
    let scope = Scope::child();
    scope.run(|| {
        provide(timers.clone());
        provide(tasks.clone());
    });
    (platform, App(timers, tasks), scope)
}

/// Runs tasks until none is ready and the pool has nothing left to bring.
fn settle(app: &App) {
    loop {
        while app.1.run_ready() {}
        assert!(wait_idle(Duration::from_secs(20)), "the pool is stuck");
        if !app.1.run_ready() {
            return;
        }
    }
}

const MS: Duration = Duration::from_millis(1);

#[test]
fn a_task_runs_between_events_and_ends_with_its_handle_or_its_scope() {
    let (platform, mut app, scope) = setup();
    let log: Rc<RefCell<Vec<&str>>> = Rc::default();
    let (a, b, c) = (log.clone(), log.clone(), log.clone());

    // It does not start inside the call that spawns it, and its value is what it gives.
    let kept = scope.run(|| {
        spawn(async move {
            a.borrow_mut().push("started");
            sleep(10 * MS).await;
            a.borrow_mut().push("done");
            7
        })
    });
    assert!(log.borrow().is_empty() && !kept.is_done());
    settle(&app);
    assert_eq!(*log.borrow(), ["started"]);
    platform.advance(10 * MS, &mut app);
    assert_eq!(*log.borrow(), ["started", "done"]);
    assert!(kept.is_done() && app.1.is_empty());

    // Another task waits for it and gets its value.
    let got = Rc::new(Cell::new(0));
    let seen = got.clone();
    scope.run(|| spawn(async move { seen.set(kept.await) }).detach());
    settle(&app);
    assert_eq!(got.get(), 7);

    // Dropped where it waits: what it owns is released and the rest never runs.
    let owned = Rc::new(());
    let held = owned.clone();
    let dropped = scope.run(|| {
        spawn(async move {
            let _held = held;
            sleep(10 * MS).await;
            b.borrow_mut().push("after the drop");
        })
    });
    settle(&app);
    assert_eq!(Rc::strong_count(&owned), 2);
    drop(dropped);
    assert_eq!(Rc::strong_count(&owned), 1);

    // Left to its scope, it ends with it: a completion never runs after disposal.
    let inner = scope.run(Scope::child);
    inner.run(|| {
        spawn(async move {
            sleep(10 * MS).await;
            c.borrow_mut().push("after the scope");
        })
        .detach()
    });
    settle(&app);
    assert_eq!(app.1.len(), 1);
    inner.dispose();
    assert!(app.1.is_empty());
    platform.advance(50 * MS, &mut app);
    assert_eq!(*log.borrow(), ["started", "done"]);

    // A task that ends its own scope while it runs is not kept.
    let own = scope.run(Scope::child);
    own.run(|| {
        spawn(async move {
            own.dispose();
            std::future::pending::<()>().await
        })
        .detach()
    });
    settle(&app);
    assert!(app.1.is_empty());
    scope.dispose();
}

#[test]
fn work_on_the_pool_comes_back_as_a_value_and_a_panic_as_an_error() {
    let (_platform, app, scope) = setup();
    let out: Rc<RefCell<Vec<String>>> = Rc::default();
    let (a, b) = (out.clone(), out.clone());
    scope.run(|| {
        spawn(async move {
            let sum = background(|| (1..=100u32).sum::<u32>()).await;
            a.borrow_mut().push(format!("{sum:?}"));
        })
        .detach();
        spawn(async move {
            let failed = background(|| -> u32 { panic!("no such file") }).await;
            b.borrow_mut().push(failed.unwrap_err().to_string());
        })
        .detach();
    });
    settle(&app);
    out.borrow_mut().sort();
    assert_eq!(*out.borrow(), ["Ok(5050)", "background work panicked: no such file"]);

    // No more of them run at once than the pool has threads, and it has four at most.
    assert!((1..=4).contains(&pool_threads()));
    let (now, most) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let done = Rc::new(Cell::new(0));
    scope.run(|| {
        for _ in 0..16 {
            let (now, most, done) = (now.clone(), most.clone(), done.clone());
            spawn(async move {
                let work = background(move || {
                    most.fetch_max(now.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(10));
                    now.fetch_sub(1, Ordering::SeqCst);
                });
                work.await.unwrap();
                done.set(done.get() + 1);
            })
            .detach();
        }
    });
    settle(&app);
    assert_eq!(done.get(), 16);
    assert!(most.load(Ordering::SeqCst) <= pool_threads());

    // Work whose future is dropped is told, and gives up.
    let (started, gave_up) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let (s, g) = (started.clone(), gave_up.clone());
    let long = scope.run(|| {
        spawn(async move {
            let work = background_with(move |token| {
                s.store(1, Ordering::SeqCst);
                while !token.is_cancelled() {
                    std::thread::sleep(Duration::from_millis(1));
                }
                g.store(1, Ordering::SeqCst);
            });
            work.await.unwrap();
        })
    });
    while app.1.run_ready() {}
    while started.load(Ordering::SeqCst) == 0 {
        std::thread::sleep(Duration::from_millis(1));
    }
    drop(long);
    assert!(wait_idle(Duration::from_secs(20)));
    assert_eq!(gave_up.load(Ordering::SeqCst), 1);
    scope.dispose();
}

#[test]
fn an_inbox_delivers_a_burst_once_keeps_its_limit_and_ends_with_its_scope() {
    let (platform, mut app, scope) = setup();
    let got: Rc<RefCell<Vec<Vec<u8>>>> = Rc::default();
    let code = Rc::new(Cell::new(None));
    let (seen, ended) = (got.clone(), code.clone());
    let tx = scope.run(|| {
        let inbox = inbox::<u8>();
        let tx = inbox.sender();
        inbox
            .on_finish(move |c| ended.set(Some(c)))
            .on_data(move |bytes| seen.borrow_mut().push(bytes.clone()));
        tx
    });
    settle(&app);

    // A hundred pushes from another thread before the loop turns: one wake, one delivery.
    let producer = tx.clone();
    std::thread::spawn(move || {
        for n in 0..100u8 {
            assert!(producer.extend(&[n, n]));
        }
    })
    .join()
    .unwrap();
    assert!(platform.step(&mut app), "the loop was woken");
    assert!(!platform.step(&mut app), "once");
    assert_eq!(got.borrow().len(), 1);
    assert_eq!(got.borrow()[0].len(), 200);

    // The code comes after the last data, and nothing is taken after it.
    assert!(tx.push(7));
    tx.finish(3);
    assert!(!tx.push(8) && tx.is_closed());
    assert_eq!(code.get(), None);
    settle(&app);
    assert_eq!(got.borrow().last().unwrap(), &[7]);
    settle(&app);
    assert_eq!(code.get(), Some(3));
    assert!(app.1.is_empty());

    // Past its limit an inbox drops, or its producer waits for the delivery.
    let sizes: Rc<RefCell<Vec<usize>>> = Rc::default();
    let (d, b) = (sizes.clone(), sizes.clone());
    let (drops, blocks) = scope.run(|| {
        let (drops, blocks) = (inbox_with::<u8>(4, Full::Drop), inbox_with::<u8>(4, Full::Block));
        let senders = (drops.sender(), blocks.sender());
        drops.on_data(move |bytes| d.borrow_mut().push(bytes.len()));
        blocks.on_data(move |bytes| b.borrow_mut().push(bytes.len() + 100));
        senders
    });
    assert!(drops.extend(&[0; 10]));
    settle(&app);
    assert_eq!(*sizes.borrow(), [4]);
    let producer = std::thread::spawn(move || blocks.extend(&[0; 10]));
    while !producer.is_finished() {
        settle(&app);
        std::thread::yield_now();
    }
    settle(&app);
    assert!(producer.join().unwrap());
    assert_eq!(*sizes.borrow(), [4, 104, 104, 102]);

    // The scope ends: pushes are dropped and the sender says closed.
    let inner = scope.run(Scope::child);
    let orphan = inner.run(|| {
        let inbox = inbox::<u8>();
        let tx = inbox.sender();
        inbox.on_data(|_| panic!("delivered after its scope ended"));
        tx
    });
    settle(&app);
    inner.dispose();
    assert!(!orphan.push(1) && orphan.is_closed());
    settle(&app);
    scope.dispose();
}

//! The background pool (specs/11): blocking work runs on a few threads and comes back as a
//! value. The work sees no signal and no node: it is `Send`, and they are not.

use std::any::Any;
use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

type Job = Box<dyn FnOnce() + Send>;

#[derive(Default)]
struct Queue {
    jobs: VecDeque<Job>,
    /// Jobs waiting and jobs running.
    pending: usize,
}

struct Pool {
    queue: Mutex<Queue>,
    work: Condvar,
    idle: Condvar,
    threads: usize,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The pool of the process, started by the first work given to it.
fn pool() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    static STARTED: AtomicBool = AtomicBool::new(false);
    let pool = POOL.get_or_init(|| Pool {
        queue: Mutex::default(),
        work: Condvar::new(),
        idle: Condvar::new(),
        threads: std::thread::available_parallelism().map_or(1, usize::from).min(4),
    });
    if !STARTED.swap(true, Ordering::AcqRel) {
        for n in 0..pool.threads {
            let spawned =
                std::thread::Builder::new().name(format!("quadra-pool-{n}")).spawn(move || worker(pool));
            // The OS gave fewer threads than asked: the work queues for those there are, and
            // with none it never runs, which the future's owner sees as a wait without end.
            drop(spawned);
        }
    }
    pool
}

fn worker(pool: &'static Pool) {
    loop {
        let job = {
            let mut queue = lock(&pool.queue);
            loop {
                match queue.jobs.pop_front() {
                    Some(job) => break job,
                    None => queue = pool.work.wait(queue).unwrap_or_else(PoisonError::into_inner),
                }
            }
        };
        job(); // it catches its own panic
        let mut queue = lock(&pool.queue);
        queue.pending -= 1;
        if queue.pending == 0 {
            pool.idle.notify_all();
        }
    }
}

/// How many threads the pool has: the cores of the machine, four at most.
pub fn pool_threads() -> usize {
    pool().threads
}

/// Waits until the pool has nothing to do, `at_most` so long. Says whether it has nothing.
/// A test harness settles with it; an app has no reason to wait.
pub fn wait_idle(at_most: Duration) -> bool {
    let (pool, end) = (pool(), Instant::now() + at_most);
    let mut queue = lock(&pool.queue);
    while queue.pending > 0 {
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return false;
        }
        queue = pool.idle.wait_timeout(queue, left).unwrap_or_else(PoisonError::into_inner).0;
    }
    true
}

/// The work panicked; this is what it said.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Panicked(pub String);

impl fmt::Display for Panicked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "background work panicked: {}", self.0)
    }
}

impl std::error::Error for Panicked {}

fn said(panic: Box<dyn Any + Send>) -> String {
    let text = panic.downcast_ref::<&str>().map(|s| s.to_string());
    text.or_else(|| panic.downcast_ref::<String>().cloned()).unwrap_or_else(|| "no message".into())
}

/// Set when the future of the work is dropped: long work looks at it and gives up.
#[derive(Clone)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

struct Shared<T> {
    result: Option<Result<T, Panicked>>,
    waker: Option<Waker>,
}

/// The result of work on the pool, when it is done. Dropping it sets the work's token; work
/// that has not started is then never run.
#[must_use = "the work's result comes through this future; dropping it cancels the work"]
pub struct Background<T> {
    shared: Arc<Mutex<Shared<T>>>,
    token: CancelToken,
}

impl<T> Drop for Background<T> {
    fn drop(&mut self) {
        self.token.0.store(true, Ordering::Release);
    }
}

impl<T> Future for Background<T> {
    type Output = Result<T, Panicked>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut shared = lock(&self.shared);
        match shared.result.take() {
            Some(result) => Poll::Ready(result),
            None => {
                shared.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// Runs blocking work on the pool. Its panic is the error of the future.
pub fn background<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> Background<T> {
    background_with(move |_| work())
}

/// [`background`] for work long enough to ask now and then whether it is still wanted.
pub fn background_with<T: Send + 'static>(
    work: impl FnOnce(&CancelToken) -> T + Send + 'static,
) -> Background<T> {
    let shared = Arc::new(Mutex::new(Shared { result: None, waker: None }));
    let token = CancelToken(Arc::new(AtomicBool::new(false)));
    let (out, asked) = (shared.clone(), token.clone());
    let job = move || {
        if asked.is_cancelled() {
            return;
        }
        let result = catch_unwind(AssertUnwindSafe(|| work(&asked))).map_err(|panic| Panicked(said(panic)));
        let waker = {
            let mut out = lock(&out);
            out.result = Some(result);
            out.waker.take()
        };
        // Outside the lock: no lock is held across a call into someone else's code.
        if let Some(waker) = waker {
            waker.wake();
        }
    };
    let pool = pool();
    let mut queue = lock(&pool.queue);
    queue.jobs.push_back(Box::new(job));
    queue.pending += 1;
    pool.work.notify_one();
    Background { shared, token }
}

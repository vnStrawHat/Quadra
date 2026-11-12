# 11 — Async and Background Work

The UI thread owns all UI state. Work that waits or takes long runs elsewhere and comes back as
a value.

## Local tasks

```rust
let load = spawn(async move {
    match background(move || std::fs::read_to_string(path)).await {
        Ok(Ok(text)) => document.set(parse(&text)),
        failed => error.set(Some(format!("{failed:?}"))),
    }
});
```

- `spawn(future)` runs a future on the UI thread, between events: it starts after the call
  that spawned it returns, not inside it. The future may hold signal handles, because it
  never leaves the thread. It runs in the scope it was spawned in, inside a batch.
- It returns a `Task<T>`. **Dropping a task cancels it**: the future is dropped where it
  waits and whatever it owns is released. `task.await` gives what the future gave.
- **A task belongs to the scope it was spawned in**: when the scope is disposed its future
  is dropped. `task.detach()` lets go of the handle without cancelling, which leaves the
  task to that scope; a task for the whole run of the app is spawned in the app's scope.
  (A handle that is neither kept nor detached cancels at once; the compiler warns.)
- A task that finishes after its scope was disposed cannot happen: its future was dropped.
  This is the property that makes reads of dead signals rare (03). A task that awaits one
  whose scope ended waits for ever, so the two share a scope.
- The executor (`Executor`) is Quadra's own (01, R15): futures live in a map on the UI
  thread; a waker holds only a task id and the list of ready ids, so it is `Send` and needs
  no `unsafe`. A task woken a hundred times before it runs runs once. The ready tasks are
  run once each after every entry point (an event, a timer, a frame, the platform's `wake`);
  what they wake waits for the next, so a task that wakes itself cannot starve events.
  The first id of a turn wakes the loop through the platform's `Waker` (10).
- An app has one executor, provided as context by its root scope beside its timers; the
  test harness provides its own to views mounted with `Harness::mount`, and
  `Harness::settle()` runs tasks and frames until none is ready and the pool is idle.
- A scope pays one cleanup for its tasks, and one for its timers, however many it starts
  while it lives: a task that sleeps in a loop leaves nothing behind.

## The background pool

- `background(|| work)` runs a blocking closure on the pool and returns a future of its result.
  The closure and its result must be `Send`; it can capture no signal and no node.
- The pool has a fixed number of threads: the number of cores, at most four. Work queues when
  all are busy. It is the pool of the process, started by the first work given to it.
- Dropping the future does not stop a closure that already started; one that has not
  started is never run. Long work takes a `CancelToken` and checks it:
  `background_with(|token| ..)`. The token is set when the future is dropped.
- A panic in the closure is caught at the pool boundary and returned as the error of the
  future (`Panicked`, with what the panic said), so the future gives `Result<T, Panicked>`.

The V design started one OS thread per task. A fixed pool bounds threads and makes start-up of
a task cheap.

## Inboxes

For a producer thread that delivers a stream: a pseudo terminal, an SSH channel, a log tail.

```rust
let inbox = inbox_with::<u8>(1 << 20, Full::Block);
let tx = inbox.sender();                         // Send + Clone
std::thread::spawn(move || {
    while let Ok(n) = pty.read(&mut buf) {
        if !tx.extend(&buf[..n]) { return }      // the view is gone
    }
    tx.finish(0);
});
inbox.on_finish(move |code| exited.set(Some(code))).on_data(move |bytes| screen.update(|s| s.feed(bytes)));
```

- An inbox holds items of one type (`Inbox<T>`; bytes are `Inbox<u8>`). `push(item)` and
  `extend(&items)` append under a lock and wake the UI thread only if no wake is pending.
  A burst of pushes is one delivery.
- Delivery swaps the filled vector with the one delivered before, emptied, so a steady
  stream allocates nothing. The callback is given the vector and may take what is in it.
- `inbox_with(limit, full)` holds `limit` items at most between two deliveries; a producer
  past it waits for the delivery (`Full::Block`) or loses what does not fit (`Full::Drop`).
  `inbox()` has no limit. The terminal's limits in behaviour/17 use this.
- `finish(code)` is delivered to `on_finish` after the last data; nothing is taken after it.
- The receiving end is a task of the scope `on_data` is called in; after its disposal,
  pushes are dropped, a producer that waits goes on, and `push` and `extend` say false.

## Timers

- `sleep(duration).await` in a task.
- `timeout(duration, || ..)` and `interval(duration, || ..)` return handles owned by the scope.
- `debounce(signal, duration)` and `throttle(signal, duration)` give derived signals.
- All use the platform clock, which is virtual under test: `harness.advance(ms)` fires them in
  order.
- A callback runs in the scope the timer was set in, inside a batch. It may cancel its own
  timer and set others.
- An app has one `Timers`, which gives the platform's timers their meaning. It is provided as
  context by the app's root scope (as the clipboard is, 10), so `timeout` and `interval` work
  wherever a view of the app is built; the test harness provides its own to views mounted
  with `Harness::mount`.

Frame-driven animation does not use timers: transitions and animations are advanced by the
frame (04, 05).

## Apps that need a full async runtime

The framework depends on none (01, R13). An app that needs one, for a network library built on
it, runs it on its own threads and talks to the UI through inboxes and `background`-style
futures. That is a decision of the app, recorded in its own spec.

## Images and other loaders

Decoding runs on the pool, through `ImageDecoder` (10): the system codecs on Windows, a decoder
of flat test images in the harness. The size limits of behaviour/08 are checked from the header
before any pixel is decoded. `image(source)` shows a placeholder, decodes in the background, and
sets the node's image when done; the node is flagged for layout if its intrinsic size was
unknown, otherwise for paint. Decoded images are cached at device size under a byte budget.

## Rules

- No blocking call on the UI thread: file reads beyond small config files, network, process
  spawning and waits go to the pool or a reader thread.
- No lock is held across a call into app code.
- Shared state between threads is a channel or an inbox, never a signal behind a mutex.

## Tests

Cancellation by drop and by scope disposal; a completion never runs after disposal; the pool
bound; a panic becoming an error; inbox coalescing (many pushes, one delivery) and its limit;
timers in virtual time and their order.

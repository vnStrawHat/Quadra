# 11 — Async and Background Work

The UI thread owns all UI state. Work that waits or takes long runs elsewhere and comes back as
a value.

## Local tasks

```rust
let load = spawn(async move {
    let text = background(move || std::fs::read_to_string(path)).await?;
    document.set(parse(&text));
    Ok(())
});
```

- `spawn(future)` runs a future on the UI thread, between events. The future may hold signal
  handles, because it never leaves the thread.
- It returns a `Task<T>`. **Dropping a task cancels it**: the future is dropped at its next
  await point and whatever it owns is released.
- A task spawned inside a scope and not kept is owned by that scope: it is cancelled when the
  scope is disposed. `task.detach()` gives it to the app's root scope instead.
- A task that finishes after its scope was disposed cannot happen: its future was dropped.
  This is the property that makes reads of dead signals rare (03).
- The executor is Quadra's own (01, R15): futures live in a slab on the UI thread; a waker
  holds only a task id and a sender, so it is `Send` and needs no `unsafe`. Runnable ids are
  drained in the platform's `wake` and after each event. Waking from another thread goes
  through the platform's `Waker` (10).

## The background pool

- `background(|| work)` runs a blocking closure on the pool and returns a future of its result.
  The closure and its result must be `Send`; it can capture no signal and no node.
- The pool has a fixed number of threads: the number of cores, at most four. Work queues when
  all are busy.
- Dropping the future does not stop a closure that already started. Long work takes a
  `CancelToken` and checks it: `background_with(|token| ..)`. The token is set when the future
  is dropped.
- A panic in the closure is caught at the pool boundary and returned as an error of the future.

The V design started one OS thread per task. A fixed pool bounds threads and makes start-up of
a task cheap.

## Inboxes

For a producer thread that delivers a stream: a pseudo terminal, an SSH channel, a log tail.

```rust
let inbox = inbox::<Vec<u8>>();
let tx = inbox.sender();                         // Send + Clone
std::thread::spawn(move || while let Ok(n) = pty.read(&mut buf) { tx.push(&buf[..n]) });
inbox.on_data(move |bytes| screen.update(|s| s.feed(bytes)));
```

- `push` appends under a lock and wakes the UI thread only if no wake is pending. A burst of
  pushes is one delivery.
- Delivery swaps the filled buffer with an empty one, so a steady stream allocates nothing.
- The byte inbox has a size limit; a producer past it blocks or drops by the policy the owner
  chose. The terminal's limits in behaviour/17 use this.
- `finish(code)` is delivered after the last data.
- The receiving end is owned by a scope; after disposal, pushes are dropped and the sender
  reports closed.

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

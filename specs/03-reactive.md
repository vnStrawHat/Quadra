# 03 — Reactive Runtime

`quadra-reactive` holds all state and the graph of who depends on what. It knows nothing about
UI and is tested alone.

## Primitives

```rust
let count = signal(0);                       // Signal<i32>: Copy, 'static
let double = memo(move || count.get() * 2);  // Memo<i32>: cached, lazy
effect(move || println!("count is {}", count.get()));
count.set(1);                                // the effect runs when the batch ends
```

| Primitive | Read | Write | Notes |
|---|---|---|---|
| `Signal<T>` | `get()` (clones), `with(\|v\| ..)`, `get_untracked()` | `set`, `update`, `update_if` | Holds a value of any size |
| `Memo<T: PartialEq>` | `get`, `with` | none | Recomputed on demand; dependents hear of it only if the value changed |
| `Effect` | | | Runs once when created, then when a source changed |
| `Trigger` | `track()` | `notify()` | A signal with no value |

A handle is a small `Copy` value: an index and a generation into the runtime's arenas. Closures
capture handles freely; no reference counting, no borrow across calls.

### Change detection is the default

- `set(v)` requires `T: PartialEq` and does nothing when the value is equal.
- `update(|v| ..)` mutates in place and always notifies. It is for values too large to compare
  (a terminal screen, a document).
- `update_if(|v| -> bool)` mutates in place and notifies when the closure says it changed.
- `set_always(v)` is for types without equality.

The V design notified on every update and on every discrete event. Here writing a value a
signal already holds runs nothing.

## The graph

- Reading a signal or memo inside a memo, an effect or a binding records an edge. Edges are
  rebuilt at every run: dependencies are exactly what the last run read.
- Edges are stored on both ends, so finding the readers of a signal costs the number of
  readers, and removing a node costs its own edges. Nothing scans a global list.
- **Propagation is two-phase.** A write marks direct readers dirty and their descendants
  "to check", and stops there. When an effect or binding is about to run, it asks its sources;
  a memo marked "to check" recomputes only if one of its own sources really changed. So a memo
  runs at most once per batch, an effect never sees a half-updated state, and a memo whose
  value did not change stops the wave.
- `untrack(|| ..)` reads without recording edges.

## Batches

Every entry point runs in a batch: an OS event, a timer, a task completion, a frame. Inside a
batch, writes mark the graph; nothing runs. When the outermost batch ends:

1. Events emitted during the batch are delivered in emission order (see Events).
2. Effects run, sources before dependents, then in creation order.
3. Bindings are not run. They are queued on their window and run in the next frame's update
   pass (02, "The frame").

Steps 1 and 2 repeat while they produce more work. An effect that runs more than 64 times in
one batch is a cycle: it is stopped for that batch and reported through the error hook with
the chain of sources that kept waking it.

`batch(|| ..)` opens one explicitly, for code that runs outside the framework's entry points
(tests).

## Scopes

A scope owns what was created while it was current: signals, memos, effects, bindings, event
subscriptions, tasks, timers, context values, cleanup callbacks, child scopes.

- The view layer makes one scope per mount boundary: a window root, a `dynamic` region, a list
  row, an overlay (05). Nodes mounted in a scope are removed with it.
- Disposing a scope disposes its children first, runs cleanups in reverse order, cancels its
  tasks and timers, and frees its slots. It is immediate and deterministic.
- `on_cleanup(|| ..)` registers a callback. Resources that need releasing implement `Drop` and
  are simply owned by a signal or a closure of the scope.
- App-level state that must outlive every window is created in the app's root scope.

### Disposed handles

A handle can outlive its scope only by being stored somewhere owned by an older scope.

- `try_get`, `try_with`, `try_set` return `None` or `false` on a disposed handle.
- `get`, `with`, `set` on a disposed handle are programming errors and panic, naming where the
  signal was created (`#[track_caller]`).
- Callbacks owned by a scope (effects, bindings, handlers, task completions, timers,
  subscriptions) never run after it is disposed. This removes the usual way to reach a dead
  handle: a late completion.

## Events

For things that happen rather than values that are: "a tab asked to close", "output arrived".

```rust
let closed = event::<TabId>();               // Event<TabId>: Copy
closed.subscribe(move |id| tabs.update(|t| t.remove(*id)));   // owned by the current scope
closed.emit(id);                             // delivered when the batch ends
```

Subscribers are stored on the event, so emitting costs the number of subscribers.

## Context

Values shared down the tree without passing them: theme, settings, the locale, the window.

```rust
provide(Theme::dark());            // on the current scope
let theme = expect::<Theme>();     // nearest enclosing scope that provides one
```

Lookup walks the scope chain by `TypeId`. A context value is usually a signal or a struct of
signals, so readers update when it changes.

## Structured state

There is no model type. Domain state is plain structs whose changing parts are signals:

```rust
struct Session { title: Signal<String>, busy: Signal<bool>, screen: Signal<Screen> }
```

Granularity is chosen by where the signals are: one per field that changes alone, one around a
large object that is mutated in place and repainted as a whole.

Collections: a `Signal<Vec<T>>` with a keyed `each` (05) is enough for lists of hundreds, and
costs the list length per change. Large collections are shown through a virtual list, which
asks for a count and builds only visible rows. A collection that reports its own edits
(inserted, removed, moved) is not built until a measured screen needs it.

## Rules the runtime enforces

- One runtime per UI thread; handles are neither `Send` nor `Sync`.
- A signal's value is borrowed only for the duration of `with` or `update`. Writing the same
  signal inside its own `with` panics with both locations.
- Effects must not be relied on for ordering between each other beyond "sources first".
- Bindings never write signals. A binding that needs to (a measured size feeding state) goes
  through `layout_signal` (04), which is delivered after the layout pass in its own batch.

## Implementation

How the crate is built, so that it needs no `unsafe` and no user code ever runs while the
runtime is borrowed.

- **Storage.** One thread-local runtime holds generational arenas of nodes (signals, memos,
  effects), scopes and events. A node's value is an `Rc<RefCell<dyn Any>>`: a read clones the
  `Rc` out of the arena in a short borrow, then borrows the value while the caller's closure
  runs. The runtime's own `RefCell` is never held across a call into user code. For the same
  reason, what a disposed scope owned is moved out of the arenas and dropped afterwards: a
  value's `Drop` may use the runtime.
- **States.** A node is clean, to check, or dirty. A write marks direct readers dirty and, the
  first time a node leaves clean, marks the readers of a memo "to check", queues an effect, or
  calls a deferred effect's notify. A node that was already marked is not walked again.
- **Updating a node.** "To check": update each source in the order it was read, and stop as
  soon as one makes this node dirty. Dirty: run it. The node is set clean *before* it runs, so
  a write during its own run to something it reads marks it again and it runs once more.
- **Edges.** While a node runs, each read is compared with the source it read at the same
  position last time. As long as they match nothing is touched, so a node with stable
  dependencies costs no edge work. From the first difference on, new sources are collected, and
  the edge from a new source is added at once (not at the end of the run), so a write later in
  the same run still reaches the running node. At the end, sources no longer read lose their
  edge. A source read twice is recorded once.
- **A memo that changed** raises its "to check" readers to dirty. A memo reading itself panics.
- **Bindings** are deferred effects: created with a notify callback and a body. The body runs
  once at creation. When a source changes, notify is called once; the owner (a window's update
  pass) later calls `run`, which checks the sources and runs the body only if one really
  changed.
- **Effects in a flush** run in creation order. Events queued by `emit` are delivered before
  effects, one at a time, to the subscribers still subscribed at that moment.
- **Counts for tests**: the number of live nodes, scopes and events is public (`stats`).

## Tests

The crate's own tests cover: each memo runs at most once per batch; an unchanged memo stops
propagation; effects see consistent state (no glitch on a diamond); dependencies follow
branches; scopes free every slot (the arenas return to their starting size); disposed callbacks
never run; a cycle is reported and stopped; equal writes run nothing.

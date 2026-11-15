# 01 — Architecture Decisions

Each entry: choice, rationale, what was rejected, cost of reversing. "The V design" means the
implementation described by specs 00–14 of the V repository.

## R1. The UI is a retained tree of nodes
- **Choice**: each window owns one tree of nodes that lives across frames. A node holds its
  style, layout result, state, handlers and painted output. It is the single source of truth
  (04-tree).
- **Rationale**: the V design rebuilt elements at every render and every frame table (hitboxes,
  handlers, key nodes, semantics) at every prepaint, so the cheapest frame still walked the
  window. A tree that persists can be changed where it changed.
- **Rejected**: elements rebuilt per render with caches added per phase. Every cache needs its
  own invalidation, and identity stays positional.
- **Reversal cost**: very high. Proven or dropped in phase R1.

## R2. State is signals; views are built once
- **Choice**: state lives in signals. A view function runs once, when it is mounted, and binds
  node properties to the signals they read. A change runs only the bindings that read it
  (03-reactive, 05-views).
- **Rationale**: with views re-rendered as a whole, one row changing among 3,000 cost a render
  and a layout walk of all of them. A binding costs one closure call and one property write.
- **Rejected**: re-rendering a view and diffing the result against the tree. It keeps "UI is a
  function of state", but costs the view's size per change and allocates a description each
  time. It stays available on purpose as `dynamic` (05), the coarse tool.
- **Fallback if R1 fails**: keep the retained tree and R3–R7, replace build-once views with
  re-rendered views diffed against the tree. The reactive crate and the view layer change; the
  tree, passes and renderer do not.
- **Reversal cost**: high for the kit and apps, low for the layers below.
- **Status**: accepted by the user on 2026-11-11; phase R1 measures it.

## R3. A frame is a fixed sequence of passes over dirty nodes
- **Choice**: update, style, layout, compose, paint, accessibility, draw. Each pass visits only
  nodes flagged for it. A property knows which pass it dirties. No pass means no frame.
- **Rationale**: this replaces the four hand-chosen invalidation levels. The level is a
  consequence of which property changed, not a call the framework's code must remember to make.
- **Reversal cost**: high.

## R4. Painted output is kept per layer, with subtree-relative ranges
- **Choice**: a layer is a subtree with its own display list, transform, clip and opacity.
  Scrolling and transform or opacity animation change only the layer's constants. Inside a
  layer each node records its instances as a range relative to its parent's, so a clean subtree
  is copied without being visited (07-render).
- **Rationale**: the V design baked the scroll offset into every bound, so one pixel of
  scrolling meant prepaint and paint of the window.
- **Rejected**: textures per layer (memory, blurry text while scrolling); one flat list for the
  window (any insertion shifts everything).
- **Reversal cost**: medium, inside `render` and the paint pass.

## R5. Atlas pages are slices of one texture array
- **Choice**: an instance names its slice. A draw call never breaks on a page change.
- **Rationale**: it removes the overlap reordering, the spatial hash, and the rule that a
  reordered scene cannot be patched.
- **Reversal cost**: low.

## R6. Hit testing walks the tree; handlers live on nodes
- **Choice**: a pointer event finds its target by walking layers and nodes from the top.
  Handlers, focus and key contexts are node data (09-input).
- **Rationale**: events are rare and frames are frequent. The V design paid for dispatch tables
  at every prepaint.
- **Reversal cost**: low.

## R7. Identity is the node
- **Choice**: scroll position, focus, selection and animation state live on the node. Nodes are
  created and removed only at explicit boundaries: a mount, a `dynamic` region, a keyed list.
- **Rationale**: the V design keyed element state by a hash of the element's position, so a
  sibling inserted without a key moved the state of everything after it.
- **Reversal cost**: follows R1.

## R8. Layout: Quadra's own engine, incremental by answers
- **Choice**: flex and grid are written in `quadra-layout`. A node keeps the answers it gave its
  parent; a changed node is asked again, and the layout climbs only until an ancestor answers
  as before (06-layout).
- **Rationale**: incremental layout is the point of Quadra. The stop at "the first ancestor that
  answers the same" needs the engine and its caches to be designed together.
- **Rejected**: a layout crate. The widely used one caches per node, so clean subtrees are
  skipped, but a dirty node clears every ancestor up to the root and layout runs again from the
  root. It has no early stop, and it would be a dependency at the centre of the framework.
- **What carries from the V engine**: requests, answers and the climb. What does not: recycling
  and replacing subtrees, which existed because every render built new nodes.
- **Reversal cost**: low, behind `quadra-layout`.

## R9. GPU: D3D11 directly, behind `Renderer`
- **Choice**: unchanged from the V design: flip-model swap chain, frame-latency waitable,
  precompiled shaders, software device first at start-up.
- **Rejected**: a portable GPU abstraction crate. It brings every platform at once, at the cost
  of a multi-megabyte binary and a slower start. The renderer draws one kind of instanced
  primitive, so a second backend is small when it is needed.
- **Reversal cost**: low, behind the trait.

## R10. Text: native backend, retained paragraphs, counted glyph tiles
- **Choice**: DirectWrite shapes and rasterizes behind `TextBackend`. A text node owns its
  shaped and wrapped paragraph until its text, style or width changes. A glyph tile lives while
  something on screen refers to it, then waits in a least-recently-used list (08-text).
- **Rationale**: the V design looked text up by its content every frame and never evicted a
  glyph. A retained tree makes ownership the natural lifetime.
- **Reversal cost**: medium.

## R11. One semantic tree for accessibility and for tests; the OS adapter from `accesskit`
- **Choice**: nodes describe themselves as semantic nodes in Quadra's own tree
  (`quadra-a11y`); updates are sent incrementally and only while assistive technology listens;
  actions arrive as semantic actions. Tests find and drive controls through that same tree,
  headless, with no desktop session. The bridge to the OS is `accesskit` with its Windows
  adapter, used only by the native crate (12-a11y-testing).
- **Rationale**: the V design sent the whole tree after each prepaint and performed every action
  as a mouse click at the node's centre. One tree for both uses keeps accessibility tested by
  every test. The OS protocol itself (UI Automation providers, their patterns and events) is
  hard to get right and gains nothing from being written again; the same crate family has
  adapters for the other desktop systems.
- **Rejected**: an own provider, as in the V code (about 1,100 lines of COM that covered the
  basic patterns).
- **Reversal cost**: low; only `quadra-native-win` names the crate.

## R12. Windowing: own Win32 layer; OS couplings live in the per-OS native crate
- **Choice**: the platform and renderer traits name no OS concept. Where the two must know each
  other (the swap chain's waitable handle in the message loop), the per-OS crate that builds
  both wires them (10-platform).
- **Rationale**: IME control, per-monitor DPI, the modal resize loop and snap layouts on a
  custom titlebar need Win32 directly. The V traits leaked handles and host names.
- **Reversal cost**: low.

## R13. Async: a local executor and a small pool; dropping a task cancels it
- **Choice**: futures run on the UI thread between events; blocking work runs on a fixed pool;
  the executor is Quadra's own, written on the standard library's `Wake`, with no `unsafe`
  (11-async).
- **Rejected**: one OS thread per task, as in the V design; a full runtime as a dependency of
  every app.
- **Reversal cost**: low.

## R14. Lifetime is scope ownership and `Drop`
- **Choice**: signals, effects, tasks, timers, subscriptions and nodes belong to a scope; a scope
  ends when its region of the tree is removed. OS and GPU resources are released by `Drop`.
- **Rationale**: this is the V ownership tree with the language doing the work.

## R15. Dependencies: few, each justified and listed here
- **Rule**: a crate is admitted when all of these hold. It implements an external standard or
  an OS protocol that Quadra gains nothing by writing again. It is maintained and widely used.
  Its own dependency tree is small, with no async runtime and no C code built at compile time.
  It sits behind one Quadra module. It is not at the centre of what Quadra is: the reactive
  runtime, the tree and passes, layout, the renderer, text layout, the kit are always Quadra's.
- **Admitted**:

  | Crate | For | Behind |
  |---|---|---|
  | `windows` | Win32, D3D11, DXGI, DirectWrite, the system image codecs | `quadra-native-win`, `quadra-shell` |
  | `accesskit`, `accesskit_windows` | The accessibility bridge to the OS (R11) | `quadra-native-win` |
  | `serde_json` (with `serde`, no derive) | Reading and writing themes, settings and keymaps | `quadra-app`, `quadra-design` (theme files) |
  | `oneterm-vt` | The terminal engine: VT parser, grid and scrollback, reflow, selection, damage, key and mouse encoders, the pseudo console. The user's own crate, pinned by tag; it replaces the engine the V tree wrote (behaviour/17). Its tree: `bitflags`, `log`, `memchr`, `rustc-hash`, `unicode-segmentation`, `unicode-width`, and with `pty` `polling` and `windows-sys`. Removing it costs the terminal | `quadra-term` (no features), `quadra-shell` (`pty`) |
  | `polling` | The poller the pseudo console of `oneterm-vt` is waited on with: its types are in that API | `quadra-shell` (`pty`) |
  | `russh` (features `ring`, `rsa`), `tokio` (`rt-multi-thread`, `net`, `time`, `sync`, `io-util`) | 1Term's SSH and SFTP (specs/19, P2): the protocol, its crypto and the agent. **Two exceptions to the rule, the user's choice (2026-11-15)**: `tokio` is an async runtime, and `russh` must have a crypto backend that builds C and assembly (`ring`, by `cc` with the MSVC the target needs anyway). The tree is about 215 crates, none in the framework. Removing them costs SSH, or an engine of our own | `oneterm-ssh` only |
  | `regex-lite` | The patterns a user types to filter a log (and to find in text, when the editor does): the syntax everyone knows, which Quadra gains nothing by writing again. It has no dependency and is small on purpose (slower than `regex`, which is four crates and far larger in the binary). Removing it costs the filter by pattern, or an engine of our own | `quadra-data` (the log's filter), `quadra-code` (finding in a document) |

  One version of `windows` is used: the one the accessibility adapter depends on. It brings
  three build-time crates of its own macros (`syn`, `quote`, `proc-macro2`); nothing of them
  is in the binary, and the crate with them builds in a few seconds. `windows-core`, a part
  of `windows` that is in its tree anyway, is named as a dependency of `quadra-native-win`
  too: the macro that implements a COM interface (the text source and sink DirectWrite calls
  back, 08) refers to the crate by that name.
- **Kept Quadra's own, though crates exist**: layout (R8, it is the centre); the task executor
  (R13, about 150 lines on the standard library); Unicode tables for graphemes, cell widths
  and normalization, generated by the scripts carried from the V tree, because the text tests
  fix their exact behaviour (the terminal's cell widths are its engine's); logging; the test
  and bench harnesses.
- **Taken from the OS instead of a crate**: image decoding, shaping, line breaking and
  bidirectional analysis.
- **Adding one**: a new entry in the table above, with what it replaces and what removing it
  would cost. The gate compares the workspace's dependency list with this table.
- **Rationale**: the user's requirement is to limit dependencies, not to have none. Every one
  is build time, binary size and behaviour Quadra does not control, so each must pay for that.

## R16. `unsafe` only at the OS boundary
- **Choice**: `#![forbid(unsafe_code)]` in every crate except the Win32 platform crate, the
  D3D11 renderer and the DirectWrite backend. Each `unsafe` block there states its invariant.
- **Rationale**: the V code needed checked holders for type erasure everywhere; Rust needs none.

## R17. Build time is a budget
- **Choice**: no procedural macros in the framework; views are type-erased where they are
  placed, so generic code is not instantiated per call site; public APIs take trait objects or
  small generic shims over non-generic functions; crates are split so that an app edit rebuilds
  only the app (14-performance).

## R18. Errors are values that reach the app
- **Choice**: the OS boundary returns `Result`. A failure the framework cannot handle goes to
  the app's error hook and the log. A read of a disposed signal is a programming error and
  panics; callbacks owned by a scope never run after it ends, which removes the usual cause.
- **Rationale**: the V design printed GPU failures to a console a GUI program does not have.

## R19. Style values may be tokens, resolved late
- **Choice**: a colour, radius or font in a style is a literal or a theme token. Tokens resolve
  in the style pass. Changing the theme restyles the window and builds nothing (13-kit).
- **Rejected**: one binding per themed property, which is one closure per node per property.

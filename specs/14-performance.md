# 14 — Performance

## Principles

1. **Work follows the change.** What a frame costs depends on what changed, not on what the
   window holds.
2. **Structure before speed.** A budget is first a count of things visited, checked by tests.
   Timings confirm it on real hardware.
3. **Idle is free.** Nothing flagged, nothing drawn, no timer running.
4. **Measured, not assumed.** Every claim here has a test or a bench. A number that was not
   measured is marked as a target.

## Structural budgets

Each row is a test on `FrameStats` (12), in a window that also holds 10,000
unrelated nodes. "Path" means the node and its ancestors up to the layout boundary.

| Change | Bindings | Styled | Laid out | Painted | Uploaded | Mounted |
|---|---|---|---|---|---|---|
| Nothing | 0 | 0 | 0 | 0 | 0 instances, no frame | 0 |
| Pointer moves over plain content | 0 | 0 | 0 | 0 | no frame | 0 |
| Pointer enters a button | 0 | 1 | 0 | 1 | that node's instances | 0 |
| A label's text changes, same size | 1 | 0 | path | 1 | that node's instances | 0 |
| A label's text changes size | 1 | 0 | path and affected siblings | those nodes | their layer | 0 |
| One row of a 10,000-row list changes | 1 | 0 | path | 1 | that node's instances | 0 |
| Scroll plain content | 1: the offset | 0 | 0 | 0 | 0 instances, one layer constant | 0 |
| Scroll plain content past what is painted around the window (07) | 1: the offset | 0 | 0 | the nodes of three viewports | that layer | 0 |
| Scroll a virtual list within realized rows | the list's own | 0 | 0 | 0 | 0 instances, one layer constant | 0 |
| Scroll a virtual list by one row | the list's own and the row's | 0 | 1 row | 1 row | that layer | 0 (recycled) |
| A transform or opacity animation frame | 0 | 0 | 0 | 0 | one layer constant | 0 |
| A colour transition frame | 0 | 1 | 0 | 1 | that node's instances | 0 |
| Caret blink | 0 | 0 | 0 | 1 | that node's instances | 0 |
| Theme change, colours only | 0 | all | 0 | changed | changed layers | 0 |
| A character typed in an input | 1–2 | 0 | path | the input | its instances or layer | 0 |
| One terminal row changes | 0 | 0 | 0 | 1 surface | that row's instances | 0 |

The scroll position is a signal, so scrolling runs the binding that turns it into a layer
offset: that is the one binding of the scroll rows. Past what is painted, the pass looks at
the nodes near the old band and the new one and at each child of a long column once, not at
every node of the page (`FrameStats::nodes_walked`, `tests/culling.rs`). The tests are
`crates/quadra-view/tests/budgets.rs` and `tests/list.rs`; the rows about input, style
variants, themes and the terminal are added as those are built.

The same table, read against the V design: scrolling prepainted and painted the window; a text
change rendered and laid out its whole view; hover was a special patch path.

## Time and memory budgets

These budgets are the floor, and Quadra's own last measurements (`docs/benchmarks.md`) the
baseline: a change must not make them slower.

| Scenario | Budget |
|---|---|
| Idle with no caret | 0 frames, 0% CPU |
| Hover across a grid of 1,000 buttons | < 0.3 ms main-thread CPU per frame |
| Typing in an input inside a large window | < 1 ms |
| Scrolling a 1,000,000-row table | < 1.5 ms |
| Window resize of the gallery, full relayout | < 6 ms |
| Full first build of the gallery | < 10 ms |
| Startup to first presented frame | < 150 ms |
| Input to present | ≤ 1 frame plus compositor |
| Memory, gallery working set | < 80 MB |
| Memory, a terminal with one idle session in the gallery (private working set / commit) | < 25 MB / < 80 MB |

Targets, to be confirmed or revised with measurements at the end of phase R1:

| Scenario | Target |
|---|---|
| Hover across a grid of 1,000 buttons | < 0.05 ms |
| Scrolling plain content, any size | < 0.1 ms of CPU outside the driver |
| Memory per plain node (box with a style) | < 300 bytes |
| Memory per binding | < 100 bytes |
| Release binary of the counter | < 3 MB |
| Release binary of the gallery | < 6 MB |

## Terminal throughput

A terminal is measured the way a known fire demo stresses one (its source is read, never
built; the user's choice, 2026-11-15): `bench/doomfire` is that demo written again in Rust.
Each frame is the whole screen at two fire pixels a cell: `▀` with its foreground (the upper
pixel) and background (the lower) as 256-colour SGR written only when they change from the
cell before, a row ended by `CSI 0K CSI 1E CSI 1G`, the frame started by `CSI 1;1H` and the
first colours; about 180 KB at 200 by 50. The fire spreads up from a white last row by the
demo's own rule (a random step of 0 to 3, the heat less 0 or 1). One write a frame through
`WriteConsoleA` on the console's UTF-8 code page, then a status line: `mem: <min> min / <avg>
avg / <max> max [ <fps> fps ]`. Its fps is frames written over seconds since the first: what
the terminal takes in as the writer sees it, the console host between them included. Bands:
5 fps or fewer very poor, 24 or fewer poor, 100 or more great.

Three numbers, each with `doomfire --seconds N --out FILE` (it ends after N seconds and
writes `<cols>x<rows> <frames> frames <fps> fps <bytes> bytes a frame`):

- **Feed**: `doomfire-feed`, the frames fed straight to a `quadra-term` session's `Feeder`
  with no console and no window: the engine's ceiling.
- **1Term**: `tools/doomfire.sh`, the app's release build in a real window running
  `doomfire` in a space through the pseudo console, 1440 by 900.
- **Reference**: the same program in another terminal, by hand.

The goal is the best fps there is: 1Term's is to come as near the feed's as the console host
lets it, and never below the poor band. A fourth number, `doomfire-conpty`, is that host's
own ceiling: the demo through the pseudo console with its output read and dropped. 1Term
ships a newer host than the system's (specs/01, R15): `tools/conpty.sh` puts it beside the
binaries, and every number of 1Term is taken with it.

## Baselines

Quadra's numbers are in `docs/benchmarks.md`, each with its date and commit; a change is
measured against the last of them. The V implementation's, measured 2026-11-11 on the
development machine with the comparison bench (`bench/compare`; the scenarios are defined in
`docs/benchmarks-v.md`), are kept as history:

| Scenario | CPU per frame | Peak memory |
|---|---|---|
| list | 0.35 ms | 20.5 MiB |
| grid (600 cells, each changing every frame) | 1.19 ms | 29.8 MiB |
| document (40 paragraphs wrapped to a width changing every frame) | 0.69 ms | 23.5 MiB |

Counter: 46 ms to the first frame, 10.1 MB idle, 1.6 MB binary. Terminal throughput
(`bench/term`): 65, 102 and 350 MB/s on its three workloads.

Phase R1 ports the four scenarios and the runner unchanged. The Rust numbers are recorded
beside these. A scenario that is slower needs an explanation and a decision before phase R2.

Honest expectations, so a surprise is recognized as one:

- **list** and plain scrolling should improve most: they stop doing per-frame work.
- **grid** changes every cell every frame, the worst case for fine-grained updates: 600
  bindings, 600 repaints. It measures the fixed cost per binding and per node repaint. If the
  design loses anywhere, it is here.
- **document** is dominated by line breaking, which is the same work in both designs.
- **Start-up** adds building the reactive graph to mounting; the first build of a large window
  is the cost to watch.
- **Binary size** will be larger than the V build.

## Build time

Build time is a budget because it is paid on every change (01, R17):

| Measure | Target |
|---|---|
| Rebuild and relink an app after a one-line edit in the app | < 10 s |
| Rebuild after a one-line edit in `quadra-design` | < 30 s |
| Clean debug build of the workspace | measured in R0, then fixed as a budget |
| `cargo test` of the framework crates | < 3 min |

Means: no procedural macros; views boxed where placed; thin generic shims over non-generic
functions; crates split along the layers of 02; few dependencies, each with only the features
used (01, R15); build output shared between checkouts by the build tool (00).

## Shipped builds

What ships and what is measured is the `dist` profile: release with fat LTO and one codegen
unit, so the kit, the tree and the text system are optimized as one program and the hot
paths inline across crates. Measured 2026-11-16 against plain release: document 0.725 to
0.648 ms a frame, grid 1.195 to 1.135, the binary 2.84 to 2.77 MB (docs/benchmarks.md). The
gate builds and tests in plain release, which builds far faster; `tools/bench.sh`,
`tools/doomfire.sh` and `tools/package.sh` build in `dist`.

## Costs and their answers

| Cost | Answer |
|---|---|
| Running app code per frame | None: components run once. Bindings run only when a source changed |
| Layout | Results kept on nodes; boundaries stop propagation; caches skip clean subtrees (06) |
| Text | Paragraphs kept on nodes; a width change breaks lines without shaping (08) |
| Emitting instances | Per node, on change; clean subtrees copied or untouched (07) |
| Uploads | A node's range when the count is unchanged; one layer otherwise |
| Draw calls | One per layer run; no split on atlas pages |
| Scrolling and transform animation | A layer constant |
| Allocation per frame | None in steady state: no view description, recycled rows, reused scratch buffers |
| Memory | Counted glyph tiles and images under budgets; scopes free everything they own |
| Start-up | Precompiled shaders, software device first, lazy font fallback, embedded assets |

## Tooling

- `FrameStats` and pass timings in every build; the frame HUD in debug builds (12).
- Micro-benches per crate (reactive propagation, layout, paragraph, display-list repaint).
- The comparison bench (`bench/compare`) with its outside-the-process runner (`bench/runner`),
  unchanged in method so numbers stay comparable over time and across the frameworks it
  measures (docs/benchmarks.md, "Method").
- A memory probe that reports arena sizes, atlas use and layer buffers.
- Bench results are recorded in `docs/benchmarks.md` with the date and the commit.

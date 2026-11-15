# Benchmarks

Measurements of the Rust implementation, newest first, each with its date and the commit it
was taken on. The V implementation's numbers, the baseline of specs/14, are in
[benchmarks-v.md](benchmarks-v.md).

Machine: the development machine (the one of benchmarks-v.md). Release profile at cargo's
defaults until phase R1 fixes it.

## 2026-11-15 — a new band of a long page (on 57c15af plus this commit)

A page of plain rows (a box of 40 px and a text each) in a `scroll`, not a virtual list,
scrolled 1,300 px a frame so every frame paints a new band; release build, the harness's
platform, microseconds of the frame (all of it the paint pass):

| Rows | Before | Now |
|---|---|---|
| 2,000 | 140 | 55 |
| 20,000 | 1,723 | 529 |
| 100,000 | 12,325 | 4,073 |

A node outside the band that emitted nothing in the last one and has nothing to paint is
skipped with what it holds; before, every node of the layer was walked and its range written.
What is left is a look at each row of the column, some 25 ns: a search among children placed
in order would take it lower, if pages that long are ever scrolled without a virtual list.

## 2026-11-15 — what a number costs to lay out (on 7c16cc6 plus this commit)

`tools/bench.sh grid document`, headless, microseconds a frame, three runs each:

| Scene | Pass | Before | Now | What changed |
|---|---|---|---|---|
| grid | layout | 208 | 133 | A number is composed into a buffer the text system keeps, not into a new word (a vector and a reference count a number); the lines of a wrap are found in a kept buffer |
| document | layout | 44 | 42 | the same, for its wraps |

Shaping and wrapping a number of five characters took 209 ns and takes 109 (measured alone,
600 numbers a frame on DirectWrite). What else was looked at, and left: the update pass is
the scene's own formatting of 600 floats (66 µs measured bare), the reactive runtime (some
57 µs to run 1,200 bindings and their 600 memos, 15 to tell them) and applying 1,200 writes
(56); keeping the queues' buffers between frames and a `const` thread local for the runtime
gained nothing above the noise. A text is measured three times a frame, but the two after the
first are answered from what the paragraph holds: a few microseconds in all, so the layout's
knowing that a height does not depend on the width offered is not worth its bookkeeping. A
measure of a box fixed on both axes that skips its children was tried and changed nothing: the
grid's cells are only performed.

## 2026-11-14 — the grid's update and layout (on a2ea8cd plus this commit)

`tools/bench.sh grid`, headless, microseconds a frame (600 cells whose number and colour
change every frame):

| Pass | Before | Now | What changed |
|---|---|---|---|
| update | 239 | 226 | The bindings that are due run in one batch: the runtime flushes once, not once a binding |
| layout | 263 | 204 | A text that changes keeps the faces and metrics of its style (they were looked up twice a shaping); the places of a shaping's glyphs are a buffer of the text system, not an allocation; the lookups by face use the fast hasher (moved to `quadra-prim`) |
| paint | 180 | 180 | nothing: about 185 ns a text of five glyphs, a lookup and an instance a glyph |

Where the update's time goes (measured by timers in the pass, then removed): 25 µs telling 600
memos and their 1,200 bindings, 155 µs running them (the scene formats 600 floats), 55 µs
applying the 1,200 writes. A text is measured three times a frame (its base, its height at its
width, its place); the first shapes it (300 ns), the two others are 30 ns. Merging the
runtime's borrows in `graph::update` was tried and was slower.

## 2026-11-14 — the code editor (on b0d43da plus this commit)

The editor's scenes of the bench app (`bench/compare/src/editor.rs`), headless with real text
(`BENCH_FRAMES=600 tools/bench.sh editor editor-scroll`), on 100,000 lines of Rust (4.2 MB),
microseconds a frame:

| Scene | Update | Paint | Draw | Instances written |
|---|---|---|---|---|
| `editor`: a letter typed, then taken out, at the middle line | 10 | 367 | 10 | 2 586 |
| `editor-scroll`: 8 px down a frame | 2 | 8 | 0 | 31 |

A keystroke costs the lines the view shows, not the file: the budget is 2 ms (behaviour/18)
and the frame takes 0.39. The paint is the band the editor's layer paints, about twice the
view, painted again whole; painting only the changed line would take it lower, if a scene
ever needs it. Scrolling moves the layer and paints a band only when the view leaves the
last one. Opening is the buffer's (6 ms for ten megabytes, specs/13, "Code") and one frame:
the highlighter reads only as far as the view.

## 2026-11-14 — what a frame costs the GPU (on db5e3f9 plus this commit)

The first measure of the GPU's time (specs/07, "Drawing a frame": timestamps around the draw,
read back a few frames later), in a real window on the machine's RTX 3050, 600 frames of each
scene with `BENCH_PHASES=1`, microseconds a frame:

| Scene | GPU | Draw (CPU) | Present (CPU) | Draws |
|---|---|---|---|---|
| grid | 146 | 118 | 88 | 1 |
| document | 118 | 207 | 111 | 2 |
| list | 41 | 67 | 123 | 3 |

A box that blurs what is behind it (specs/07): a pane of 600 by 400 over a gradient filling
1 200 by 800, offscreen on the same GPU, 300 frames: 59 µs a frame without the blur, 84 with
a blur of 16 and 87 with 32. The copy and the mips cost 25 µs; the blur is taken again every
frame drawn.

So the one shader is not what costs a frame: the GPU's part is about one percent of a frame
at 60 Hz in the heaviest scene, and shaders by class of instance wait until a scene shows otherwise. The
headless paint pass of `grid` is as before the gradients and borders (178 to 180 µs).

## 2026-11-11 — with the accessibility bridge (on fffd71e plus this commit)

The runner, the Rust build alone, medians of three rounds (`.tmp/cmp5.json`): nothing moved
but the binary.

| Measure | Before | With the bridge |
|---|---|---|
| list / grid / document: CPU per frame | 0.36 / 1.29 / 0.70 ms | 0.36 / 1.26 / 0.68 ms |
| counter: startup | 41 ms | 39 ms |
| counter: idle memory | 11.1 MB | 11.1 MB |
| list / grid / document: peak memory | 22.8 / 25.5 / 28.2 MB | 23.2 / 27.8 / 27.5 MB |
| binary size | 0.88 MB | 1.50 MB |

The adapter of the accessibility crate is 0.6 MB of the binary (the budget of specs/14 for
the counter is 3 MB). It is made with the window and costs nothing measurable at start-up.
None of the bench's windows was asked for its tree on this machine, which runs no assistive
technology: the accessibility pass did not run.

With a listener (forced, without a window, `BENCH_FRAMES=3000`), what building the update
of a frame costs: list 12 us (60 nodes), grid 169 us (600 texts change every frame),
document 19 us (38 nodes). The grid's was 700 us before the nodes to send were stamped
instead of searched for in a list.

## 2026-11-11 — the result of phase R1: the comparison bench, three rounds (on f96baf0 plus this commit)

The runner's medians of three rounds, both builds taking turns in one session
(`.tmp/cmp4.json`):

| Measure | V | Rust |
|---|---|---|
| list: CPU per frame | 0.38 ms | 0.36 ms |
| grid: CPU per frame | 1.31 ms | 1.29 ms |
| document: CPU per frame | 0.73 ms | 0.70 ms |
| list / grid / document: p99 frame interval | 13.49 / 13.43 / 13.44 ms | 13.45 / 13.44 / 13.44 ms (75 Hz: 13.33) |
| list / grid / document: peak memory | 21.6 / 31.1 / 24.7 MB | 22.8 / 25.5 / 28.2 MB |
| counter: startup | 44 ms | 41 ms |
| counter: idle memory | 10.0 MB | 11.1 MB |
| counter: idle CPU | 0.84 ms/s | 0.91 ms/s |
| binary size | 1.63 MB | 0.88 MB |

Every time is at the V build's or a little under it; the binary is half. Memory is the one
measure that is not better everywhere: idle 1 MB more, document's peak 3.5 MB more.

What start-up took (189 ms before, of which 136 ms was the hardware device being made before
the first frame):

- The first frames are drawn on the software device and copied to the window, and the
  renderer moves to the hardware device when a worker has made it (specs/07, "Start-up"):
  189 to 51 ms. The move is the app's recovery from a lost device, asked for on purpose.
- The software device is made on that worker too, while the UI thread opens the window (13 ms)
  and builds the view: 51 to 41 ms.
- A face is checked for composing numbers only once it has been asked for 64 of them: the
  counter's "0" no longer shapes 300 pairs in the first frame (1 ms).

The memory that came with it: before the two devices, the peaks were 12.7 / 17.5 / 22.7 MB
and idle 10.4 MB. The software rasterizer's library stays loaded after its device is gone, as
in the V build, which starts the same way. Ten megabytes for 150 ms of start-up is the trade
both builds make.

## 2026-11-11 — the comparison bench after the three fixes (on 7a38ba6 plus f96baf0)

One round of the runner, both builds taking turns:

| Measure | V | Rust before | Rust now |
|---|---|---|---|
| list: CPU per frame | 0.39 ms | 0.37 ms | 0.35 ms |
| grid: CPU per frame | 1.29 ms | 3.22 ms | 1.25 ms |
| document: CPU per frame | 0.73 ms | 1.55 ms | 0.69 ms |
| list / grid / document: peak memory | 23.2 / 31.2 / 24.7 MB | 12.9 / 23.2 / 23.3 MB | 12.7 / 17.5 / 22.7 MB |
| counter: startup | 44 ms | 187 ms | 189 ms (not worked on yet) |

Where a frame's time went was measured first: the app reports each frame's passes
(`FrameReport`), and the bench prints their averages with `BENCH_PHASES=1`. With
`BENCH_HEADLESS=1` it runs the scenes on real text without a window, as fast as they go, which
gives numbers steady enough to compare two builds (`tools/bench.sh grid document`). Microseconds
a frame, headless:

| Scene | Pass | Before | Now | What changed |
|---|---|---|---|---|
| grid | layout | 295 (2,663 in the window) | 247 | Numbers are composed from digit glyphs shaped once, when the face shapes them glyph by glyph (checked once per face), as the V build does |
| grid | update, paint | 241, 172 | 239, 157 | nothing yet: 200 ns a binding, 260 ns a repainted text |
| document | layout | 285 | 42 | Three things, below |
| document | paint, draw | 526, 70 | 134, 22 | A scroll container's content is painted only near what shows (specs/07): 4,250 glyphs instead of 15,964 |

The document's layout was doing its work six to sixteen times over:

- A paragraph was broken again for every request, though a layout asks at the space it has and
  then at the width it was told, and both give the same lines. A wrap now answers for every
  width from its widest line up to the width it was broken at.
- After a change, a node was asked again everything it had answered before, including
  requests made with sizes it no longer has, each a layout of the whole page at an old width.
  A replay now stops at the first answer that differs, asks in the order the parent had
  asked, and is skipped when a value the parent reads itself has changed.
- A perform nobody had asked for as a measure stayed in the cache as if it were one.

In the window a frame costs two to three times its headless time, in both builds: the thread
sleeps 12 ms between frames and the processor slows down. The runner's CPU time is what counts.

## 2026-11-11 — the comparison bench, first run in a real window (on af3edbe plus the uncommitted bench)

`bench/compare` is the V bench's four scenes with the same text, sizes and frame recording,
on the Win32 platform, the D3D11 renderer (hardware device) and DirectWrite. The V build's
runner measures both builds from outside, in one session, taking turns (one round):

```sh
tools/bench.sh                                              # builds target/release/compare.exe
<V repo>/.tmp/cmprun.exe -runs 1 -out .tmp/cmp1.json v=<V repo>/.tmp/cmp.exe rust=target/release/compare.exe
```

| Measure | V | Rust | |
|---|---|---|---|
| list: CPU per frame | 0.38 ms | 0.37 ms | the same |
| grid: CPU per frame | 1.28 ms | 3.22 ms | 2.5 times slower |
| document: CPU per frame | 0.74 ms | 1.55 ms | 2.1 times slower |
| list / grid / document: peak memory | 21.5 / 31.1 / 24.7 MB | 12.9 / 23.2 / 23.3 MB | less |
| counter: idle memory | 10.1 MB | 10.4 MB | the same |
| counter: idle CPU | 0.99 ms/s | 0.84 ms/s | the same |
| counter: startup | 46 ms | 187 ms | 4 times slower |
| binary size | 1.6 MB | 0.7 MB | less than half |
| frame intervals (p50 / p99) | 13.33 / 13.45 ms | 13.33 / 13.44 ms | the display's 75 Hz |

It runs, and the four windows show what the V build's show (`tools/shot.py` saves a window's
pixels from the screen). Three numbers are worse, each with a cause already known and none of
them the paradigm:

- **grid**: every frame has 600 numbers never seen before, and each is shaped by DirectWrite.
  The V build composes a number from digit glyphs shaped once (after checking, once per face,
  that the face shapes numbers glyph by glyph). Without a window the same scene costs 0.44 ms
  (below): binding, layout and paint are not where the time goes.
- **document**: 15,600 glyphs are written and uploaded again every frame, 144 bytes each,
  though the window shows a seventh of the page. The V build skips lines outside the clip.
- **startup**: the hardware device is made before the first frame. The V build draws the
  first frame on the software device and makes the hardware one meanwhile (specs/07,
  "Start-up").

These are the next work, measured with pass timings first.

## 2026-11-11 — the scenes with glyph tiles (on 0e95923 plus the uncommitted glyph tiles)

The same run as below, now that text is painted from atlas tiles: every glyph looks its tile up
by face, glyph, device size and subpixel phase, and the text node keeps the handles.

| Scene | A frame | With the standard hasher |
|---|---|---|
| list | 0.007 ms | 0.008 ms |
| grid | 0.443 ms | 0.509 ms |
| document | 0.663 ms | 0.934 ms |

The lookup is the cost: 15,600 glyphs a frame in `document`. The atlas now hashes its keys with
a multiply-and-rotate hasher instead of the standard one, which is built to resist chosen keys
and took a third of the frame. What is left is about 42 ns a glyph. `document` at 0.66 ms of
CPU before any GPU work is already near the V build's whole frame (0.69 ms): painting only the
lines near the visible part, as the V build does, is the likely answer, to decide with the real
backend.

## 2026-11-11 — the comparison scenes without a window (on c92b93f plus the uncommitted `quadra-view`)

`tools/cargo.sh run --release -p quadra-view --example scenes`

The passes on the CPU only: fake text backend, null renderer, 800 x 600. No glyph is rasterized
and nothing reaches a GPU, so these are not yet comparable with the V numbers, which are whole
frames of a real window (list 0.35 ms, grid 1.19 ms, document 0.69 ms).

| Scene | First frame | A frame | Bindings | Laid out | Painted | Instances written | Uploaded |
|---|---|---|---|---|---|---|---|
| list: 100,000 rows of three texts, scrolling 8 px a frame | 0.47 ms | 0.007 ms | 2 | 4 | 1 | 8 | 404 |
| grid: 600 cells, a number changing in each | 1.76 ms | 0.415 ms | 600 | 1,284 | 600 | 2,465 | 3,064 |
| document: 40 paragraphs wrapped to a new width | 3.50 ms | 0.604 ms | 1 | 285 | 41 | 15,600 | 15,600 |

What they say so far:

- **list** scrolls for 7 µs a frame: most frames change one layer constant and nothing else.
  Every third or fourth frame a row is given to the row entering, which runs its three
  bindings and repaints it (the counts above are averages a frame). The upload is the list's
  layer, about 1,200 instances, when a text changes its length.
- **grid**, the worst case for fine-grained updates, spends 0.7 µs per cell on binding, layout
  and paint together. The whole layer is uploaded because the numbers change their digit
  count, which rebuilds the list.
- **document** is all glyph emission: every glyph of every paragraph is written again, 39 ns
  each, although most of the page is below the window. The V build skips lines outside the
  clip. Painting only what is near the visible part of a layer would cut this to a quarter, at
  the price of painting when scrolling brings new lines in. To decide with the real backend.
- Both will grow with real text: shaping by the OS (the V build composes numbers from digit
  glyphs instead of shaping them) and a tile lookup per glyph.

## 2026-11-11 — incremental layout (on eccd55c plus the uncommitted `quadra-layout`)

`tools/cargo.sh run --release -p quadra-layout --example relayout`

A scrolling column of 10,000 rows, each a container holding one text leaf (20,001 nodes). One
row's text changes and takes more lines.

| Rows | From scratch | One row's text changes | Nodes asked | Ancestors laid out again |
|---|---|---|---|---|
| Fixed height | 5.0 ms | 0.82 µs | 4 | 1 |
| Sized by their text | 7.2 ms | 494 µs | 9 | 2 |

With rows of fixed height the row answers its parent as before, so the change stops there and
costs the same in a list of ten rows. With rows sized by their text the row's height changes,
so the list itself is laid out again: its 10,000 children answer from their caches, at about
50 ns each. That is the known ceiling of a container with thousands of direct children that is
not virtualized (specs/06, "What a change costs"; specs/09 names the same ceiling for hit
testing). If a measured screen needs it, the answer is a pass that only shifts the rows after
the changed one. A virtual list holds some forty rows and does not have the problem.

From scratch is about 80 ns per request worked out (each row and each leaf is asked three
times). Not optimized yet.

## 2026-11-11 — reactive propagation (on 7f18b29 plus the uncommitted `quadra-reactive`)

`tools/cargo.sh run --release -p quadra-reactive --example propagation`

| Shape | Cost |
|---|---|
| 600 signals, one binding each; write all, run all | 93 ns per binding, 56 µs per frame |
| The same through a memo per cell | 121 ns per binding, 73 µs per frame |
| One cell of the 600 changes | 92 ns per change, whatever the other 599 |

A binding here is a deferred effect that reads its source and adds to a counter, so this is
the runtime's own cost: the write, the mark, the notify, the check and the call. It is the
shape of the comparison bench's `grid` (600 cells, all changing every frame), the worst case
for fine-grained updates. For scale, the V implementation spends 1.19 ms of CPU per frame on
`grid` in total, so the reactive layer is about a twentieth of that. What the nodes, layout
and paint add is measured when they exist.

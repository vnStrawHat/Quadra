# Benchmarks

Measurements of the Rust implementation, newest first, each with its date and the commit it
was taken on. The V implementation's numbers, the baseline of specs/14, are in
[benchmarks-v.md](benchmarks-v.md).

Machine: the development machine (the one of benchmarks-v.md). Release profile at cargo's
defaults until phase R1 fixes it.

## 2026-11-11 — the comparison scenes without a window (on c92b93f plus the uncommitted `quadra-ui`)

`tools/cargo.sh run --release -p quadra-ui --example scenes`

The passes on the CPU only: fake text backend, null renderer, 800 x 600. No glyph is rasterized
and nothing reaches a GPU, so these are not yet comparable with the V numbers, which are whole
frames of a real window (grid 1.19 ms, document 0.69 ms).

| Scene | First frame | A frame | Bindings | Laid out | Painted | Instances written | Uploaded |
|---|---|---|---|---|---|---|---|
| grid: 600 cells, a number changing in each | 1.76 ms | 0.415 ms | 600 | 1,284 | 600 | 2,465 | 3,064 |
| document: 40 paragraphs wrapped to a new width | 3.50 ms | 0.604 ms | 1 | 285 | 41 | 15,600 | 15,600 |

What they say so far:

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

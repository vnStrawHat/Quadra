# Benchmarks

Measurements of the Rust implementation, newest first, each with its date and the commit it
was taken on. The V implementation's numbers, the baseline of specs/14, are in
[benchmarks-v.md](benchmarks-v.md).

Machine: the development machine (the one of benchmarks-v.md). Release profile at cargo's
defaults until phase R1 fixes it.

## 2026-11-11 — the comparison bench, first run in a real window (on af3edbe plus the uncommitted bench)

`bench/compare` is the V bench's four scenes with the same text, sizes and frame recording,
on the Win32 platform, the D3D11 renderer (hardware device) and DirectWrite. The V build's
runner measures both builds from outside, in one session, taking turns (one round):

```sh
tools/cargo.sh build --release -p quadra-bench-compare      # then copy compare.exe to .tmp/
<V repo>/.tmp/cmprun.exe -runs 1 -out .tmp/cmp1.json v=<V repo>/.tmp/cmp.exe rust=.tmp/compare.exe
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

## 2026-11-11 — the comparison scenes without a window (on c92b93f plus the uncommitted `quadra-ui`)

`tools/cargo.sh run --release -p quadra-ui --example scenes`

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

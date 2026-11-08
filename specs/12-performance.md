# 12 — Performance

Performance is a feature with budgets, tests and tooling. The design choices behind it are in 01 (D3, D5, D8), 03 (chunks, overlap layers), 04 (word cache), 05 (incremental layout) and 07 (invalidation and view cache).

## Principles
1. **Idle is free.** No frame and no wakeup without a cause. Timers exist only while something needs them, like a visible blinking caret or a running animation.
2. **Work scales with the change, not the window.** A hover repaints one chunk. A keystroke re-renders one view. A scroll recomposes.
3. **No per-frame allocation on hot paths.** Scenes, layout arrays, hitbox lists, spatial hash, batch lists and text caches are reused with retained capacity. Large plain-data buffers (instances, glyph bitmaps, layout arrays) use `malloc_noscan` so the GC never scans them.
4. **Latency over throughput.** Input is handled the moment it arrives, the frame starts when the frame-latency waitable signals, and nothing waits for a fixed tick.
5. **Measure in CI.** Every budget below has a benchmark or a harness test.

## Budgets
Reference machine: 4-core laptop CPU from 2020, integrated GPU, 4K display at 150%.

| Scenario | Budget (main-thread CPU per frame) |
|---|---|
| Idle with no caret | 0 frames, 0% CPU |
| Hover across a grid of 1,000 buttons | < 0.3 ms |
| Typing in an input inside a large window | < 1 ms |
| Scrolling a 1,000,000-row table | < 1.5 ms |
| Window resize of the gallery, full relayout | < 6 ms |
| Full first render of the gallery | < 10 ms |
| Startup to first presented frame | < 150 ms |
| Input to present | ≤ 1 frame plus compositor |
| Memory, gallery working set | < 80 MB |

GPU budget: under 2 ms per frame on the reference integrated GPU, and under 40 draw calls in any gallery page.

## Where the time goes, and the answer to each

| Cost | Answer |
|---|---|
| Re-running `render()` | Dependency tracking marks only readers dirty. Clean views reuse everything. |
| Layout | Persistent tree, relayout boundaries, two-entry measure cache per node. |
| Text shaping | Word cache, line cache, wrapping on cached advances. Clean views never touch text. |
| Emitting primitives | Per-view and per-scroll chunks, copied with an offset when clean. |
| Draw calls | Overlap layers batch primitives across the whole window. |
| Uploads | Dynamic instance buffers with discard, atlas uploads limited to new tiles. |
| Images | Decoding off-thread, cached at device size with a byte budget. |
| GC pauses | Few long-lived small objects, no-scan buffers for bulk data, element trees only for dirty views. |
| Startup | Precompiled shaders, lazy font fallback, assets embedded, first window opened before non-critical services start. |

## Power
- Hidden or minimized windows produce no frames.
- On battery, animations above 60 fps are capped to 60.
- `Present1` dirty rects reduce compositor work for small changes.

## Tooling
- The profiler overlay and paint flashing (11-devtools-testing) make every frame's cost visible.
- `bench/` holds headless benchmarks: layout (flex, grid, incremental), text (shaping, wrapping 100 KB), scene composition and ordering, dispatch, model effects flush. Each prints ns/op and allocations per op.
- Harness tests assert render, layout and paint counts for key scenarios, so a caching regression fails as a test, not as a vague slowdown.

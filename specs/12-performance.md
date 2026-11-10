# 12 — Performance

Performance is a feature with budgets, tests and tooling. The design choices behind it are in 01 (D3, D5, D8), 03 (chunks, overlap layers), 04 (word cache), 05 (incremental layout) and 07 (invalidation and view cache).

## Principles
1. **Idle is free.** No frame and no wakeup without a cause. Timers exist only while something needs them, like a visible blinking caret or a running animation.
2. **Work scales with the change, not the window.** A hover repaints one chunk. A keystroke re-renders one view. A scroll recomposes.
3. **No per-frame allocation on hot paths.** Scenes, layout arrays, hitbox lists, spatial hash, batch lists and text caches are reused with retained capacity. Large plain-data buffers (instances, glyph bitmaps, layout arrays) use `malloc_noscan` so the GC never scans them. Elements built by a view's render come from the view's render arena (specs/07), so a view that renders every frame reuses the elements of its render before last. Element ids in prepaint are hashed without building strings, grapheme stepping decodes UTF-8 without allocating, and `ui.number` formats a number without a string builder.
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
| Memory, OneTerm idle with one tab (private working set / commit) | < 25 MB / < 80 MB |

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
| GC pauses | Few long-lived small objects, no-scan buffers for bulk data, element trees only for dirty views, recycled from render arenas. |
| Startup | Precompiled shaders, lazy font fallback, assets embedded, first window opened before non-critical services start. |

## Power
- Hidden or minimized windows produce no frames.
- On battery, animations above 60 fps are capped to 60.
- `Present1` dirty rects reduce compositor work for small changes.

## Tooling
- The profiler overlay and paint flashing (11-devtools-testing) make every frame's cost visible.
- `bench/` holds headless benchmarks: layout (flex, grid, incremental), text (shaping, wrapping 100 KB), scene composition and ordering, dispatch, model effects flush. Each prints ns/op and allocations per op.
- Harness tests assert render, layout and paint counts for key scenarios, so a caching regression fails as a test, not as a vague slowdown.

## Terminal throughput (OneTerm under load)
The load test is `bench/fire`: a flame of 256-color half blocks, one full screen per frame, as many frames as the terminal takes; its frames per second measure how fast the terminal drains its PTY. The window shows at most one frame per display refresh (vsync), so fire's rate is throughput, not what reaches the screen.

```sh
bench/fire/run.sh                 # bundled ConPTY host, 3 runs of 15 s at 160x45, median
bench/fire/run.sh --host system   # the inbox host (kernel32)
bench/fire/run.sh --stats --no-build --runs 1   # per-second counters in .tmp/fire-run/bundled/data-1/stats.txt
```

The script builds fire and OneTerm with clang `-prod`, gives each run a fresh `--data` folder whose `terminal.json` makes fire the shell's program (`shell.kind` custom, no shell in between) with terminal logging off, and reads fire's `--report` file. The report names the terminal's real size (it must be at least 160x45) and the display's refresh rate. Keep the window visible and on screen while it runs. The reference is the original app on the same machine with the same command: 409.6 fps at 159x45. Results and the profile behind them: docs/progress.md, OneTerm throughput. `--exe` runs another build, for an A/B of two builds; the runs of an A/B are interleaved, since other work on the machine moves the numbers by 10-20 %.

## Memory
Task Manager shows three numbers per process: the private working set (its "Memory" column), the working set (with the shared DLL pages) and the commit size (private bytes: everything committed, touched or not). Every one of them is a budget; the rules that keep them low:
- **The heap goes back after a burst.** The collector is built with `USE_MUNMAP` (`tools/v.sh`; V's bundled build leaves it out), so blocks free for a few collections are decommitted. A quiet app collects only when it allocates, so `quadra/core/trim.v` adds one step: once the mapped heap has grown 8 MiB since the last trim, the next frame arms a 2 s check, and when a whole period allocated under 4 MiB the app runs two full collections that unmap every free block. Apps can call `App.trim_now()` after dropping large data. A heap that stays put, or an app that draws nothing, runs no timer for it.
- **Thread stacks are reserved, not committed.** V passes its thread stack size to `CreateThread` as the size to commit; `tools/v.sh` passes `-thread-stack-size 0`, which gives every spawned thread the executable's 32 MiB reservation, committed a page at a time.
- **The first atlas page is small.** A 2048 RGBA page costs 16 MiB of video memory and as much commit (the driver's backing, never touched); each atlas starts with a 1024 page (`gpu.page_extent`), later pages and bigger tiles take 2048.
- **The working set is not trimmed.** `SetProcessWorkingSetSize(-1, -1)` after a burst took OneTerm from 93 to 13 MB of working set, but the pages only move to the standby list and fault back on the next frames (8.5 MB again 7 s later): the first keystroke after a pause would pay for it, and the commit does not move. The original does not do it either.

The harness is `bench/mem/run.sh` (docs/benchmarks.md, OneTerm memory): OneTerm's `--mem` prints, once a second, the three numbers, the collector's heap (free, unmapped, allocated), the process heaps, the GPU resources the backend holds, the terminals' grid and history bytes and each child process (console hosts, shells); the script runs fixed scenarios and prints medians.

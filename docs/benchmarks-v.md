# Comparison benchmarks

Four small apps that other GPU UI frameworks publish numbers for, written for Quadra with the
same text (the same word generator and cell values), window sizes and frame recording
(`bench/compare`). A runner starts each app and measures the whole process from outside,
driver threads included (`bench/compare/run`).

| Scene | Window | What it does |
|---|---|---|
| `counter` | 360×260 | A number and three buttons, then nothing: startup, memory and CPU at rest |
| `list` | 800×600 | A virtual list of 100 000 rows of three texts, scrolling 8 px a frame |
| `grid` | 800×600 | 600 cells, each with its own background and a number changing every frame |
| `document` | 800×600 | 40 paragraphs (about 20 000 characters) wrapped to a width changing every frame |

```sh
tools/v.sh -cc clang -prod -o .tmp/cmp.exe bench/compare
tools/v.sh -cc clang -prod -o .tmp/cmprun.exe bench/compare/run
.tmp/cmprun.exe -runs 5 -out .tmp/cmp5.json quadra=.tmp/cmp.exe [other=app.exe ...]
```

Each scene runs 5 times and results are medians. The first 120 frames are not measured,
then 600 are. Measured:
- **CPU per frame**: the process's cycle count over the measured frames, converted to time
  with the cycle counter's rate (GetProcessTimes counts whole 15.6 ms ticks, too coarse).
  GPU work is not included.
- **Peak memory**: the highest private working set, sampled every 10 ms (Task Manager's
  "Memory" column).
- **Idle memory and CPU** of the counter, 5 to 10 seconds after its first frame.
- **Startup**: from starting the process until its first frame is presented and its window
  is visible, whichever comes last.
- **Frame intervals** (p50, p99), from the app.

Other apps can be measured on the same machine by passing them to the runner; each takes
the scene as its first argument and prints `ready`, `start` and `done {...}` like
`bench/compare/recorder.v`.

## Results (2026-11-08)

Quadra: i5-12400F, NVIDIA GPU, Windows 11, 75 Hz display, `-prod` with the collector. The
published column is the best median among three other frameworks (a Rust framework, its
fast fork, a Go framework) from their own run on an Apple M5 at 60 Hz. The machines
differ (one core of the M5 is faster), so read CPU and memory as orders of magnitude, not
exact ratios; frame intervals depend on the refresh rate and do not compare.

| Measure | Quadra | Best published |
|---|---|---|
| list: CPU per frame | 0.46 ms | 1.46 ms |
| grid: CPU per frame | 2.91 ms | 5.71 ms |
| document: CPU per frame | 0.97 ms | 3.03 ms |
| list / grid / document: peak memory | 25 / 65 / 29 MB | 94 / 106 / 104 MB |
| list / grid / document: p99 frame interval | 13.47 / 13.49 / 13.46 ms (75 Hz: 13.33) | 16.75 / 16.77 / 16.75 ms (60 Hz: 16.67) |
| counter: idle memory | 10.6 MB | 33.7 MB |
| counter: idle CPU | 0.7–1.2 ms/s, all in the GPU driver | 0.09 ms/s |
| counter: startup | 65 ms (WARP-first, below) | 157 ms |
| binary size | 1.3 MB | 3.7 MB |

Since the element-memory work (below, 2026-11-10) grid takes 1.33 ms and 29 MB, document
0.76 ms and 24 MB, list 0.41 ms and 21 MB.

Medians of 3 runs (`.tmp/cmp_final.json`, the last build of 2026-11-08). Since the run
before it, the device is created with `D3D11_CREATE_DEVICE_PREVENT_INTERNAL_THREADING_OPTIMIZATIONS`:
the driver then does its work on the calling thread instead of a worker thread, which cost
the process less CPU in total (list 0.63 to 0.49 ms, document 1.20 to 1.03 ms in an
interleaved run) and 1.3 MB, with the same frame intervals. Grid's peak memory moves
between 58 and 71 MB from run to run with the collector's timing. Idle CPU varied between
0.7 and 1.2 ms/s from run to run that day, whatever the build. The 2026-11-07 run (5 runs,
before the memory work below) had list, grid and document at 0.61, 3.09 and 1.68 ms and
34, 86 and 68 MB.

Idle CPU: a probe of the counter's threads over 5 idle seconds found Quadra's own threads at
0 cycles; one NVIDIA driver thread (`nvwgf2umx.dll`) used 9 million cycles.

## Startup

140–150 ms of the old 199 ms was `D3D11CreateDevice` on the hardware adapter: loading and
starting the NVIDIA user-mode driver, measured alone in a fresh process (no creation flag
changes it). Every D3D11 app on this machine pays it.

Since 2026-11-08 the first frames draw on WARP, which starts in 10 ms, and the app moves to
the hardware device once it is ready (specs/03, `gpu/d3d11/upgrade_windows.c.v`). Startup
went from 196 to 65 ms; idle memory rose 0.4 MB (WARP's DLL stays loaded); frame CPU and
intervals did not change. What it took:
- One worker thread creates WARP, then the hardware device. Run side by side, the driver
  start held the DLL loader lock and stalled WARP in about half the runs.
- Until the switch a window has no swap chain: WARP draws offscreen and `present` copies the
  image with GDI. A first version made a swap chain on WARP and replaced it at the switch;
  the grid scene then missed about 1% of its frames for the rest of the run (p99 26.6 ms),
  also when the second swap chain was on the same device. Creating the one swap chain
  from the polling timer, between frames, did the same; creating it at the start of the
  next draw does not.
- gdi32 is loaded at run time: `#flag -lgdi32` makes V build a GUI-subsystem program, which
  has no console output.

Whether the switch shows a blank frame only a person at the screen can check
(docs/progress.md, manual check 12).

## What the scenes found (fixed)
- `grid` spent 5 ms of layout shaping new strings and allocated 4 MB a frame. Shaping now
  skips script analysis for ASCII, numbers are composed from cached glyphs, and wrapping
  allocates little (specs/14). Grid went from 8.0 to 3.1 ms of CPU a frame.
- Paint wrapped every text a second time at a slightly different width; text elements now
  keep their measured wrap.
- `document` painted all 40 paragraphs, visible or not; lines outside the clip are skipped
  now (3.6 to 1.7 ms).
- Elements were large (`Div` 1944 bytes) and every render allocates them; they are 912
  and 176 bytes now.

## Memory work (2026-11-08)
Garbage collection cost follows the bytes allocated a frame, so the work went there.
- **Collector setting.** Boehm's free-space divisor sets how far the heap grows between
  collections. Measured on all scenes: 1 (what the build had) is the fastest and largest;
  3 cut peak memory by 13–36% for 4–9% more CPU; 6 halved it for 13–16%; incremental
  collection was worse everywhere (grid 4.3 ms and 218 MB). Apps now default to 3
  (`AppOptions.gc_divisor`; `GC_FREE_SPACE_DIVISOR` still overrides it).
- **Wrapped lines are views.** A paragraph's layout does not depend on the width, so the
  lines of a wrap are now slices of it (shared glyphs and caret stops, `LineLayout.cut`)
  instead of new layouts. Rewrapping 20 000 characters allocated 404 KB a frame, now
  64 KB: document went from 1.68 to 1.19 ms and from 68 to 37 MB.
- **Scratch buffers.** A composed number and line layout's per-byte table reuse buffers
  owned by the text system instead of new arrays (grid layout 1111 to 834 KB a frame).
- **Smaller nodes.** The grid part of a style (184 bytes) is a pointer set only on grid
  containers and items, and layout enums are bytes: `Style` 392 to 184 bytes, `Div` 912
  to 704, a layout node 584 to 376.

## Element memory (2026-11-10)
Grid allocated 1.9 MB a frame: 960 KB in render (631 divs of 728 bytes, 600 text elements,
children arrays, 600 `'${v:.2f}'` strings of about 220 bytes each, a string builder),
834 KB in layout (about 1.1 KB per changed number: a new wrap, line layout and cache
entry, and UTF-8 decoding that allocated a substring and a rune array per character; then
flex items and lines per container) and 78 KB in prepaint (two interpolated strings per
element id). What changed (specs/07 Element memory, specs/14):
- **Render arenas.** `ui.div()` and `ui.text()` during a view's render reuse the elements
  of the view's render before last, reset when the build that replaced them ends (the
  closure lifetimes' moment, with their pins). Elsewhere they allocate as before.
- **Numbers.** `ui.number(v, 2)` formats like `'${v:.2f}'` into a 16-byte string. An arena
  text element lays a number out into a wrap it keeps (`TextSystem.wrap_number`), not cached.
- **Layout** keeps flex items and lines in scratch per nesting depth; grapheme stepping
  decodes UTF-8 in place.
- **Prepaint ids** are hashed from the parent id and the key or position, no strings.

`BENCH_PHASES=1`, clang -prod, medians of 3 interleaved runs. Before is the build of that
morning; grid's after uses `ui.number` (with the old `'${v:.2f}'` cells, after allocates
157 KB a frame, 124 of them those strings, at 2.37 ms a frame in the app's own timing).
Collections are counted from the start (120 warm-up frames included); GC time is over
the 600 measured frames.

| Scene | KB a frame (render / layout / prepaint) | Collections | GC time | GC heap | Frame (render, layout µs) |
|---|---|---|---|---|---|
| grid before | 1902 (960 / 834 / 78) | 62 | 294 ms | 41 MB | 3106 µs (684, 1671) |
| grid after | 42 (9 / 12 / 20) | 16 | 30 ms | 18 MB | 2266 µs (110, 1281) |
| document before | 128 (55 / 62 / 5) | 21 | 63 ms | 18 MB | 1560 µs (76, 796) |
| document after | 47 (0 / 40 / 1) | 16 | 16 ms | 12 MB | 1516 µs (16, 798) |
| list before | 71 (1 / 0 / 69) | 17 | 16 ms | 14 MB | 844 µs |
| list after | 53 (0 / 0 / 52) | 16 | 47 ms | 11 MB | 845 µs |

The runner (`bench/compare/run -runs 3`, medians, both builds in the same rounds):

| Scene | CPU a frame before → after | Peak memory before → after |
|---|---|---|
| grid | 2.29 → 1.33 ms | 60.1 → 28.6 MB |
| document | 0.80 → 0.76 ms | 31.0 → 24.3 MB |
| list | 0.42 → 0.41 ms | 25.6 → 21.3 MB |
| counter | startup 60.6 → 56.1 ms, idle 10.0 → 10.1 MB, 0.84 ms/s both | |

For grid, a port of the same scene in Go measured 2.99 ms a frame allocating like this
did and 1.66 ms reusing its memory (the user's figures, another run); this build is 1.33.
List rows are built in prepaint, outside a render, so they still allocate; their texts keep
using the wrap cache (an earlier version laid list numbers out fresh every frame and cost
0.54 ms a frame). Binary 1.52 → 1.54 MB.

Arenas keep elements alive: each view holds up to two trees' worth (pointers, so scanned).
In the gallery (many views, most of them clean) that did not show: `bench/leak fake 10`,
two alternating runs each, live heap 16.6–18.5 MB before and 15.3–19.3 MB after, working
set 41–43 MB in both, flat over the rounds.

Microbenchmarks (`bench`, fake text), medians of 6 interleaved runs against a baseline
built from the same tree without these changes: flex 10k 3692 → 3196 µs, full build of
the 1000-button grid 2150 → 1913, gallery first render 2599 → 2421, resize 572 → 524,
typing 542 → 492, million-row table scroll 353 → 297, list scroll 34 → 33, text wrap
unchanged. Grid 10k 3673 → 3959 µs: its `compute` alone is unchanged (1.93 ms in both,
a probe), the difference is in that benchmark's tree construction on a stack `Tree`. The
hover change was 12.4 → 18.2 µs with both builds spread over 11–25 µs (a collection
landing in the 200 hovers or not); a hover mostly repaints a patch, which this work
did not touch.

## Incremental layout (2026-11-10)
`bench/statemodel` changes one row's colour among N live (not virtualized) rows and times
the update plus the frame that settles it. Before, layout was 75-95 % of that frame although
only a background changed: every build rebuilt the layout tree and laid it all out. Now a
build for notified views lays out only what their new trees change (specs/05, Incremental
layout; specs/07, Invalidation levels). clang -prod, fake text, p50 over 200 updates,
medians of 3 runs, both builds on the same machine in the same session:

| Rows | One view for all rows | A view per row | Equal write |
|---|---|---|---|
| 1,000 | 0.34 → 0.23 ms | 0.25 → 0.02 ms | 0.24 → 0.00 ms |
| 3,000 | 1.07 → 0.71 ms | 0.75 → 0.03 ms | 0.77 → 0.00 ms |
| 10,000 | 4.5 → 3.0 ms | 2.9 → 0.06 ms | 2.9 → 0.00 ms |

- **A view per row**: the row renders, its two nodes are filled again and keep their layout
  (`layout evaluations 0`), prepaint and paint run. At 10,000 rows what is left is mostly
  prepaint and paint walking the list's children to skip the clipped ones (about 30 µs each).
- **One view for all rows**: render (90 µs at 3,000 rows) and the walk that builds the new
  tree into the old nodes while comparing them (layout 400 µs instead of 810) remain.
- **Equal write**: `cx.update_if(m, data, f)` with `core.assign` skips the notification, so
  no build runs. With a plain `notify()` an equal write costs what a paint-only change does.

`bench` (main.v), 12 interleaved runs of each build (min / median, µs; the machine was
shared, so medians spread by about 5 % even where no code changed, as the text rows show):

| Benchmark | Before | After |
|---|---|---|
| Typing a character in the gallery | 216 / 225 | 124 / 129 |
| Window resize of the gallery, full relayout | 197 / 210 | 196 / 218 |
| Full build, 1000-button grid | 1148 / 1182 | 1127 / 1221 |
| Gallery first render | 1813 / 2198 | 1902 / 2198 |
| Hover change, 1000-button grid | 11.4 / 12.0 | 11.7 / 12.3 |
| Scroll a 2000-row list | 16.9 / 18.0 | 16.6 / 17.5 |
| Scroll the 1,000,000-row table | 193 / 207 | 189 / 202 |
| Flex layout, 10k nodes | 3746 / 3813 | 3633 / 3811 |
| Grid layout, 10k cells | 3923 / 4046 | 3760 / 3930 |
| Text wrap 100 KB, warm (code unchanged) | 135 / 141 | 135 / 141 |

Full layouts cost what they did. A node is 8 bytes smaller than before although it holds
the incremental state (parent, owner, last perform, flags): the cache's valid flags became a
bitmask. Parents are written where layout touches a child anyway, not when a node is made
(10,000 children of one grid made that a cache miss each). Two traps found on the way:
passing the 184-byte `Style` by value to the recycling branch of `Tree.node` made every node
build copy it (400 µs on 10,000 nodes; it takes a pointer now), and a second struct holding
the fixed cache array made V emit it before the array's typedef (a C error that depends on
use order; `answers` returns a copy of the node instead).

## OneTerm idle allocation (2026-11-10)
OneTerm idle in the harness, measured by `apps/oneterm/leak_gate_test.v` (run it with
`tools/v.sh -cc msvc -stats test apps/oneterm/leak_gate_test.v`; it prints the numbers and fails
only when closures leak): the null device, a 1 s clock tick rebuilds the root view, the caret
blinks every 250 ms. Two steps. First the arenas (the root render's own arena was already
used; what was left):
- **Kit builds during layout** (`Lazy.build_tree`: title bar, tab strip, buttons) and the
  views rendered inside them allocated about 55 KB a build: they now use the window's pass
  arena and the view's arena (specs/07). About 6 KB a build remains.
- **Interaction states** (`ui.States`, 2.6 KB) were allocated by every div with a hover or
  focus patch: an arena div keeps its block. Svg icons are pooled with their parsed source,
  hash and texture key; `kit.icon` caches its source string per glyph.
- **Each paint** allocated about 18 KB: the null device copied the scene's arrays; it now
  reuses them (`Harness.scene()` returns a copy). The GPU device never did this.

| | committed | after the arenas |
|---|---|---|
| idle, per second | 236 KB | 125 KB |
| with no task running, per second | 201 KB | 104 KB |
| a root render (build) | 156 KB | 82 KB |
| a caret-blink paint | 18-21 KB | 0-5 KB |

Then the rest, app code in the root render: about 50 KB of eager menu items (the app menu,
the new-tab menu, the theme picker) and closures. The menus now use `kit.menu_button_with`
(items built when a menu opens) and apps make no closures (specs/14, No closures in apps):

| | committed | after both steps |
|---|---|---|
| closures created while idle | 20 in 20 s (60 a minute) | 0 |
| idle, allocated per second | 235 KB | 46 KB |
| a root render (build) | 156 KB | 28 KB |
| closures over 3 rounds of the gate flows (tabs, spaces, menus, settings window, update check, About, lock and unlock) | not measured (no gate then) | +0 |

A 200 s idle run of the MSVC `-prod` build (`--mem`, a fresh `--data` folder): the `closures`
field stays at 1 (the main-queue drain); private working set 24-25 MB after the first 30 s in
both builds, commit 70-71 MB after against 72-74 MB before, the collector's heap 7-10 MB
(collected in between, not growing). Idle never made many closures (60 a minute, small ones):
the leaks that mattered were per event (a closure per task, update, dialog, terminal, palette
opening, settings save) and kept what they captured: `start_vault_job` captured the
`vault.VaultJob`, master password and new password included, in two closures per job.

## OneTerm memory (2026-11-10)
The three numbers Task Manager shows (private working set, working set, commit size), in fixed
scenarios, medians of 3 runs:

```sh
bench/mem/run.sh                      # builds OneTerm with MSVC -prod, all scenarios, 3 runs
bench/mem/run.sh --no-build --exe other.exe --out other idle fire   # a build of your own
```

OneTerm's `--mem` prints a `mem: {json}` line on stderr once a second: the process's counters
(`PROCESS_MEMORY_COUNTERS_EX2`), the collector's heap (size, free, unmapped, allocated so far),
the process heaps (`HeapSummary`), the atlas pages and instance buffers the GPU backend holds,
the terminals' grid and history bytes, and each child process (the console hosts and shells,
found by parent pid); after 5 s, the loaded modules. `--tabs <n>` opens n tabs. The script gives
every run a fresh `--data` folder, reads the sample at 10 s (25 s for fire) and writes
`.tmp/mem-run/<out>/summary.md`. MSVC, because Windows Defender quarantines clang `-prod`
OneTerm builds on this machine. Keep the window visible.

| Scenario | What runs |
|---|---|
| idle | one cmd tab, 10 s |
| tabs5 | five cmd tabs and the right dock (`right_dock.local`), 10 s |
| fire | `bench/fire` for 15 s as the shell's program, then 10 s idle |
| lines10k | `cmd /c type` of 10 000 lines of 70 characters, then idle |
| full | the default 10 000-line history filled with 152-column lines of 19 colors each |

Before and after (MB; i5-12400F, NVIDIA GPU, 1600x1000 window):

| Scenario | Private WS | Working set | Commit | GC heap |
|---|---|---|---|---|
| idle | 21.5 → 20.8 | 89.5 → 89.2 | 115.6 → 67.6 | 8.8 → 5.8 |
| tabs5 | 24.2 → 22.4 | 90.5 → 88.8 | 182.7 → 68.8 | 11.9 → 8.3 |
| fire, 10 s after | 41.3 → 23.3 | 106.9 → 89.0 | 112.1 → 73.0 | 28.7 → 5.6 |
| fire, peak | 52.2 → 52.6 | | | |
| lines10k | 27.2 → 21.1 | 92.8 → 86.7 | 93.9 → 68.4 | 16.4 → 6.1 |
| full | 43.2 → 27.3 | 108.7 → 92.9 | 107.7 → 75.2 | 28.6 → 10.6 |

The console hosts are separate processes: OpenConsole and cmd take 3.7 MB of private working
set per tab (18.5 MB for five), unchanged. Idle numbers move by 1-2 MB with the moment of the
sample: the heap regrows with the clock's garbage until the next collection. The original (its own record, 1280x800, same OS):
49.3 MB private working set and 182 MB commit idle, 26.6 MB more commit per tab.

Where it goes, idle after the changes (an address-space walk by pid, `QueryWorkingSetEx` per
page, plus `--mem`):
- **Private working set, 18-20 MB**: the collector's heap about 5 MB, of which 2.3 MB live
  (after a full collection; the rest is the garbage of a clock render a second, 0.4-0.5 MB/s,
  between collections); the process heaps 7.5 MB allocated, 9.8 MB committed (DirectWrite, the
  D3D runtime and driver, the C runtime, shell and COM); about 4 MB of written DLL data pages;
  the rest driver memory. The hardware driver accounts for 5.5 MB of the total (19.3 MB on the
  hardware device against 13.8 MB forced onto WARP).
- **Working set, about 88 MB**: the private part plus about 70 MB of shared DLL code, 40 MB of it the
  NVIDIA driver (`nvwgf2umx.dll` 31 MB resident, `nvgpucomp64.dll` 5-9 MB). WARP alone: 54 MB.
  WARP's DLL is unloaded after the switch; only `Microsoft.Internal.WarpPal.dll` (0.1 MB) stays.
- **Commit, about 68 MB**: the hardware driver adds about 43 MB (80.7 MB against 37.4 MB forced onto
  WARP, both before the atlas change): its DLLs' writable sections and heaps; 5-6 MB is the
  driver's backing of the atlas pages and instance buffer, never touched; the rest the heaps.
- **Start-up data is small**: the live heap after loading the config, the theme and binding the
  keys is 0.6 MB. The 39 themes stay embedded until listed; the completion catalogs (158 KB of
  JSON) are parsed per command on first use. Lazy loading would save nothing measurable.
- **Terminal storage**: a 176x46 grid is 0.13 MB. History lines are UTF-8 text plus style runs:
  10 000 short lines 1.6 MB, 10 000 full-width lines of 19 colors 5.4 MB (7.3 MB before the runs
  were sized exactly). The original keeps 8-byte cells at full width: 0.75 KB a row.
- **The element tree** does not show in these numbers: the live heap is 2.3 MB in all.

What changed:
- **Thread stacks** (most of the commit): V passes its 8 MiB stack size to `CreateThread` as
  the size to commit, so each terminal's two PTY threads cost 16 MiB and every task 8 MiB while
  it ran. `tools/v.sh` passes `-thread-stack-size 0` (the executable's 32 MiB reservation,
  committed as used).
- **The collector returns memory**: V builds its collector without `USE_MUNMAP`, so the heap a
  burst grew it to stayed committed and resident (`unmapped` was always 0). `tools/v.sh` turns
  it on, and `core` trims after a burst (specs/12, Memory). After fire the heap went from 29 to
  5 MB.
- **The first atlas page is 1024** (`gpu.page_extent`): a 2048 RGBA page was 16 MiB of commit,
  never touched, in the driver's backing.
- **The inbox lets go of burst buffers**: one far bigger than the chunk it carried, and both
  once the program exits (`type` of a 3 MB file kept 4-8 MB).
- **History runs are sized exactly** (they grew by appends to up to twice their length).

Not done, measured: trimming the working set (`SetProcessWorkingSetSize(-1, -1)`) after a burst
took the working set from 93 to 13 MB and the private working set to 8 MB, but the pages fault
back (8.5 MB again 7 s later, commit unchanged) and the first input after a pause would pay for
it; the original does nothing like it. Once all windows are minimized it would cost nothing
visible. `full` ranged from 26 to 32 MB over two sets of runs: the trim sometimes runs while a
stale reference to the last 4 MB read buffer is still on the stack.

`bench/fire/run.sh` (MSVC, bundled host, the build before and after interleaved, one run each
round): 384.3, 402.9, 399.8 fps before against 401.5, 400.4, 402.0 after, medians 399.8 and 401.5.

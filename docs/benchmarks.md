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
tools/v.sh -cc msvc -prod -o .tmp/cmp.exe bench/compare
tools/v.sh -cc msvc -prod -o .tmp/cmprun.exe bench/compare/run
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

Still open: grid allocates about 1.9 MB a frame (a `Div` and a text element per cell, and a
new wrap for each changed number). Fewer collections need fewer of those.

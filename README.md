# Quadra

A native, GPU-rendered UI framework in Rust for desktop applications. Windows first (Win32,
D3D11, DirectWrite). Its point is that work follows the change: a retained tree, signals,
incremental layout, and painted output kept per layer, so a frame costs what changed and
nothing else.

Status: early. The design is in [specs/](specs/README.md); what is built is in
[docs/progress.md](docs/progress.md). What anything built with the kit looks like, and how a
view is laid out, is [the design language](specs/design/README.md).

The website (an introduction, a guide, the design language and a page for each component,
with a markdown twin of every page and `llms.txt` for agents) is built from this repository
by `python tools/website.py` into `target/site`, after `tools/shots.sh dark` and
`tools/shots.sh light` have drawn the components.

## Build

Requirements: Git Bash with coreutils, Python 3, rustup, and the Visual Studio Build Tools
(MSVC and the Windows SDK). `tools/setup.sh` checks them.

```sh
tools/setup.sh        # what is missing, if anything
tools/check.sh        # format, lints, tests, repository rules
tools/cargo.sh test   # any cargo command, run from the project root
```

## Performance

Measured on 2026-11-16 (the `dist` profile): i5-12400F, RTX 3050, Windows 11,
a 75 Hz display. Every number, its method and its history are in
[docs/benchmarks.md](docs/benchmarks.md).

### Only what changed is done again

A retained tree, signals bound to the properties they change, incremental layout and display
lists patched in place: a frame lays out, paints and uploads what changed, whatever the size
of the UI. The window below is filled with numbers, every one of them on screen, each in a
box of its own colour; "one number" is the one in the middle, changing every frame, like a
clock on a dashboard (`tools/bench.sh incremental`, one frame, without a window):

| Numbers on screen | A frame changes | Laid out | Painted | Instances to the GPU | Time |
|---|---|---|---|---|---|
| 600 | one number | 3 | 1 | 6 | 2 µs |
| 2,400 | one number | 3 | 1 | 6 | 2 µs |
| 9,600 | one number | 3 | 1 | 6 | 3 µs |
| 600 | every number | 1,800 | 600 | 4,199 | 0.49 ms |
| 9,600 | every number | 28,808 | 9,608 | 67,199 | 11.3 ms |
| 600 | the whole grid, mounted again | 2,464 | 1,232 | 4,136 | 1.34 ms |
| 9,600 | the whole grid, mounted again | 38,644 | 19,322 | 66,146 | 25.3 ms |

Changing one number costs the same with 600 or 9,600 on screen. The last rows are the same
grid removed and mounted again in Quadra every frame: sixteen times the numbers cost 19
times the time, and two to three times what changing every number costs.

The same holds in the other scenes (counted by the frame loop):

| Scene | Laid out | Painted | Instances written |
|---|---|---|---|
| A list of 100,000 rows, scrolling 8 px a frame | 3 | 1 | 9 |
| A code editor on 100,000 lines, scrolling 8 px a frame | 0 | 0 | 32 |
| The same editor, a letter typed | 0 | 1 | 2,587 |

### Four small apps, measured from outside the process

`tools/bench.sh compare`: real windows, medians of three rounds, CPU at full clock since the
runner keeps a core busy.

| Scene | CPU per frame | p99 frame interval | Peak memory |
|---|---|---|---|
| A virtual list of 100,000 rows, scrolling | 0.38 ms | 13.45 ms | 23.9 MB |
| 600 cells, each number changing every frame | 1.10 ms | 13.45 ms | 29.5 MB |
| 40 paragraphs rewrapped to a width changing every frame | 0.65 ms | 13.44 ms | 26.6 MB |

A window with a counter starts in 34 ms (to its first frame on screen) and holds 11.5 MB at
rest. The binary is 2.81 MB. The GPU takes 42 to 151 µs a frame in these scenes.

### A terminal

1Term (`tools/doomfire.sh`): a fire demo that rewrites the whole screen every frame runs at
576 to 627 fps in a 143×39 window (six runs), 62 to 68 MB/s through the Windows console host,
which alone carries 71 MB/s.

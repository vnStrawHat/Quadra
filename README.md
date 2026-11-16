# Quadra

A native, GPU-rendered UI framework in Rust for desktop applications. Windows first (Win32,
D3D11, DirectWrite). Its point is that work follows the change: a retained tree, signals,
incremental layout, and painted output kept per layer, so a frame costs what changed and
nothing else.

Status: early. The design is in [specs/](specs/README.md); what is built is in
[docs/progress.md](docs/progress.md).

## Build

Requirements: Git Bash with coreutils, Python 3, rustup, and the Visual Studio Build Tools
(MSVC and the Windows SDK). `tools/setup.sh` checks them.

```sh
tools/setup.sh        # what is missing, if anything
tools/check.sh        # format, lints, tests, repository rules
tools/cargo.sh test   # any cargo command, run from the project root
```

## Performance

Measured on 2026-11-16 (commit fef3e5c): i5-12400F, RTX 3050, Windows 11, a 75 Hz display.
Every number, its method and its history are in [docs/benchmarks.md](docs/benchmarks.md).

**Work follows the change.** What one frame does, counted by the frame loop:

| Scene | Laid out | Painted | Instances written |
|---|---|---|---|
| A list of 100,000 rows, scrolling 8 px a frame | 3 | 1 | 9 |
| A code editor on 100,000 lines, scrolling 8 px a frame | 0 | 0 | 31 |
| The same editor, a letter typed | 0 | 1 | 2,586 |
| A grid of 600 numbers, all changing every frame | 1,800 | 600 | 3,534 |

**Four small apps measured from outside the process** (`tools/bench.sh compare`: real
windows, medians of three rounds, CPU at full clock since the runner keeps a core busy):

| Scene | CPU per frame | p99 frame interval | Peak memory |
|---|---|---|---|
| A virtual list of 100,000 rows, scrolling | 0.39 ms | 13.45 ms | 23.5 MB |
| 600 cells, each number changing every frame | 1.24 ms | 13.46 ms | 30.4 MB |
| 40 paragraphs rewrapped to a width changing every frame | 0.79 ms | 13.46 ms | 27.6 MB |

A window with a counter starts in 34 ms (to its first frame on screen) and holds 11.4 MB at
rest. The binary is 2.84 MB. The GPU takes 42 to 150 µs a frame in these scenes.

**A terminal** (1Term, `tools/doomfire.sh`): a fire demo that rewrites the whole screen every
frame runs at 588 fps in a 143×39 window, 63 MB/s through the Windows console host, which
alone carries 72 MB/s.

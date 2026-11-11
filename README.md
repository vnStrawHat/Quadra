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

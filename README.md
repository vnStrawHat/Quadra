# Quadra

A GPU-rendered UI framework for V. Windows first (Win32, D3D11, DirectWrite); every layer above the backends is portable. It ships with a component kit, a gallery of every component (`apps/gallery`), a reference notes app (`apps/notes`) and a Markdown reader and editor (`apps/folio`).

The kit's look is the "One" design language (specs/15): flat warm-neutral planes, hairline borders, ghost controls and one blue accent, in One Dark Pro (`kit.dark()`) and One Light (`kit.light()`), both JSON theme files like any other (`kit.load_theme`), with IBM Plex Sans and Lilex bundled (SIL OFL, `quadra/kit/fonts`).

- Design: `specs/` (start with `specs/README.md`); deviations made while building: `specs/14-implementation-notes.md`.
- Status and benchmarks: `docs/progress.md`. V pitfalls: `docs/v-notes.md`.

## Setup

On a fresh clone run `tools/setup.sh` from Git Bash: it downloads V 0.5.2 into `.tools/v` and a portable clang into `.tools/llvm-mingw` (everything stays inside the project, about 850 MB) and checks that a build works. It needs Python 3 and curl. Visual Studio is not needed; `-cc msvc` still works if it is installed.

## Commands

Always build through `tools/v.sh`. It runs the project-local V compiler, keeps caches and temp files inside the project, always builds with the garbage collector and keeps the toolchain's closure fix applied (`docs/v-notes.md`).

Run the commands from Git Bash. From cmd use `tools\v.cmd` in place of `tools/v.sh` (for example `tools\v.cmd run apps/cluster`): a plain `bash` there is usually WSL, whose environment never reaches the Windows compiler, so `v.sh` refuses to run under it.

| Task | Command |
|---|---|
| Run the gallery | `tools/v.sh run apps/gallery` |
| Run Notes with its data in one folder | `tools/v.sh run apps/notes --data .tmp/notes` |
| Read and write Markdown (Folio) | `tools/v.sh run apps/folio apps/folio/samples/welcome.md` |
| CI gate (format, portability, file sizes, tests) | `tools/check.sh` |
| Release gate (also runs the tests as clang `-prod` builds) | `tools/check.sh prod` |
| Memory over a long session (stays flat) | `tools/v.sh -cc clang -prod -o .tmp/leak.exe bench/leak && .tmp/leak.exe real 30 all` |
| Screenshots of every gallery page (design review) | `tools/v.sh -cc clang -prod -o .tmp/shots.exe bench/shots && .tmp/shots.exe .tmp/shots light` |
| Release package with manifest, version and icon | `tools/package.sh apps/notes quadra-notes "Quadra Notes" 1.0.0` |
| API reference (HTML, `docs/api`) | `tools/docs.sh` |
| Benchmarks | `tools/v.sh -cc clang -prod -o .tmp/bench.exe bench && .tmp/bench.exe` |
| Startup and memory | `tools/v.sh -cc clang -prod -o .tmp/g.exe apps/gallery && .tmp/g.exe --measure` |

Tests are headless: no window station, GPU or interactive login. Pixel tests render with WARP.

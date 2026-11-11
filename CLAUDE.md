# Quadra: working notes for Claude

Quadra is a native, GPU-rendered UI framework in Rust (Win32 + D3D11 + DirectWrite first),
with a component kit. It is a redesign of the framework first written in V, which lives frozen
in its own repository (`C:\Users\TrungKFC\Desktop\vGPUi` on the first machine) and is the
reference for behaviour, tests and measured numbers. Never edit that repository from here.

## Hard rules from the user (never break them)

1. **Stay inside the project directory**: scratch files in `.tmp/`, downloads and caches in
   `.cache/`. Never use the system temp dir, the scratchpad, `mktemp` or the home dir.
   **One exception**, granted for the build tool: the Rust toolchain and cargo's build output
   live outside (the cargo home; specs/00, Toolchain).
2. **No PowerShell**: Git Bash + coreutils only, never `.ps1` (check.sh rejects them).
   **One exception**: the cargo fork's own installer, run once per machine.
3. **Conventional Commits** for every commit message (`feat(tree): ...`, `fix(layout): ...`,
   `docs(specs): ...`, `refactor: ...`, `test: ...`, `chore: ...`, `perf: ...`; `!` or a
   `BREAKING CHANGE:` footer for breaking API changes). Scopes are crate names without the
   `quadra-` prefix (prim, reactive, layout, text, render, platform, tree, ui, async, a11y,
   app, native, shell, testing, kit, code, md, data, viz), or specs, tools.
4. **Never test the real clipboard.**
5. **Commit each finished part** once `tools/check.sh prod` exits 0, then go on with the
   next; **push only when the user asks**.

## How to work

- Chat with the user in **Vietnamese**; code, comments, docs, specs and commits in
  **English**.
- **Performance first**, and work proportional to the change: incremental layout and never
  rebuilding the whole UI are the point of Quadra (specs/00, Goals).
- **Specs come before code**: change `specs/` first, then implement, then update
  `docs/progress.md` (the source of truth for what is done).
- **A redesign is a redesign.** When a part needs designing, design it from its goals. Do not
  shape it to fit how the V code did it; what carries over does so on merit.
- **Few dependencies, not zero.** A new crate needs an entry in specs/01 (R15) and in
  `tools/check_deps.py`; the gate fails otherwise.
- **`unsafe` only in `quadra-native-win` and `quadra-shell`.** Every other crate has
  `[lints] workspace = true`, which forbids it.
- No procedural macros in the framework; files stay **at most 300 lines** (check.sh).
- The limited-RAM machine: do not run two release builds at once.
- Edits by script: Python 3 with `PYTHONUTF8=1`, opening files with `encoding="utf-8",
  newline=""`.

## Commands (Git Bash, from the project root)

| What | Command |
|---|---|
| Fresh machine | `tools/setup.sh` (checks; installs nothing) |
| Any cargo command | `tools/cargo.sh <args>` |
| Format | `tools/cargo.sh fmt --all` |
| Gate | `tools/check.sh` |
| Release gate | `tools/check.sh prod`; run both before saying done |
| One crate's tests | `tools/cargo.sh test -p quadra-prim` |
| The bench app | `tools/cargo.sh build --release -p quadra-bench-compare`, then copy `compare.exe` from the build output to `.tmp/` (docs/benchmarks.md has the runner's command) |
| A window's pixels | `python tools/shot.py "Window title" .tmp/shots/name.png` while the app runs |

Target: `x86_64-pc-windows-msvc`. Toolchain pinned in `rust-toolchain.toml`.

## Map

- `specs/README.md` indexes the specs. `specs/00` has the phases and the user's decisions,
  `specs/01` the architecture decisions, `specs/behaviour/` what components do and look like
  (carried from the V repository; their code sections are superseded).
- `crates/quadra-*`: the framework (specs/02, Crates); `crates/quadra` is the facade apps
  depend on. `bench/compare`: the comparison bench's four scenes. `examples/`, `assets/`
  appear with the phases that need them.
- `docs/progress.md`: status per phase. `docs/benchmarks-v.md`: the V implementation's
  measurements, frozen; the baseline of specs/14.

## State at the last handoff (2026-11-11)

There is no remote yet (the user sets it up later). Work goes on from a session opened in the
V repository until the user opens one here; every command starts with `cd /d/quadra`.

Phase R0 is done. Phase R1, the vertical slice that proves the paradigm (specs/00, Phases), is
close to its go/no-go; `docs/progress.md` lists each crate with what it has, what it lacks and
its tests. In short:

- `quadra-reactive`, `quadra-layout`: done.
- `quadra-render`, `quadra-text`, `quadra-tree`, `quadra-ui`, `quadra-testing`: the passes,
  layers, atlases, glyph tiles, views, virtual list, scroll, frame callbacks. Missing: style
  variants and tokens (specs/13), input (specs/09), semantics (specs/12), rows of different
  heights.
- `quadra-platform` (traits, events, the headless platform), `quadra-app` (windows, the frame
  loop, device-loss recovery), `quadra` (the facade, `quadra::app()`).
- `quadra-native-win`: the Win32 platform, the D3D11 renderer with window swap chains, the
  DirectWrite backend. Its shader is `shaders/prims.hlsl`, compiled by
  `tools/build_shaders.sh` (needs the Windows SDK's fxc) into `.cso` files that are committed.
  Its tests use windows that are never shown and the software rasterizer.

The four scenes of the comparison bench run in real windows and look right. Against the V
build (docs/benchmarks.md): list, grid and document cost the same CPU a frame or a little less,
memory is lower, the binary is less than half. **Start-up is 4 times slower (189 ms against
44 ms): that is the work in progress.** The hardware device is made before the first frame;
the V build draws the first frame on the software device and makes the hardware one meanwhile
(specs/07, "Start-up").

Room left that the numbers show (headless, `tools/bench.sh grid`): 200 ns a binding in the grid
(a reactive property boxes a closure on every run, see the `ponytail:` note in
`quadra-ui/src/div.rs`), and a flex item is measured once per height its column offers though
its height does not depend on it.

Then the go/no-go of R1 is written down, and phase R2 (input first) begins.

Habits that cost time when forgotten:

- Read the gate's exit status, not its output through a pipe:
  `tools/check.sh prod > .tmp/check.log 2>&1; echo $?`, and commit only on 0.
- A script with backslashes is written to `.tmp/` as a file, not passed through a here-document
  (the shell collapses the backslashes). `.tmp/edit.py FILE SPEC` applies exact replacements.
- The cargo fork puts binaries under `<cargo home>/targets/quadra-*/<profile>/build/<crate>/<hash>/out/`.
- `tools/bench.sh [scene...]` builds the bench, copies it to `.tmp/compare.exe` and runs the
  named scenes headless with their pass timings.
- The V runner and bench are built in the V repository (`tools/v.sh -cc clang -prod ...`; the
  runner needs `-cflags -Wno-int-conversion`).

`quadra-term` is **on hold** by the user's decision: do not build it until told. The apps come
after Quadra is complete.

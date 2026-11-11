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
6. **Commit or push only when the user asks.**

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

Target: `x86_64-pc-windows-msvc`. Toolchain pinned in `rust-toolchain.toml`.

## Map

- `specs/README.md` indexes the specs. `specs/00` has the phases and the user's decisions,
  `specs/01` the architecture decisions, `specs/behaviour/` what components do and look like
  (carried from the V repository; their code sections are superseded).
- `crates/quadra-*`: the framework (specs/02, Crates). `examples/`, `bench/`, `assets/` appear
  with the phases that need them.
- `docs/progress.md`: status per phase. `docs/benchmarks-v.md`: the V implementation's
  measurements, frozen; the baseline of specs/14.

## State at the last handoff (2026-11-11)

Phase R0 is done and committed: repository, workspace, gate, `quadra-prim`. There is no remote
yet (the user sets it up later).

Phase R1, the vertical slice that proves the paradigm (specs/00, Phases), is in progress;
`docs/progress.md` lists its crates. Done and committed: `quadra-reactive`, and `quadra-layout`
with flex. The user's standing instruction: commit each finished part once the gate is green,
then continue with the next.

Next, in order: grid in `quadra-layout` (the V repository's `quadra/layout/grid*.v` and
`grid_test.v` carry the algorithm and its cases; add `Display::Grid` and the grid style), then
`quadra-tree` (specs/04), which joins the reactive runtime and the layout tree.

`quadra-term` is **on hold** by the user's decision: do not build it until told. The apps come
after Quadra is complete.

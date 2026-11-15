# 00 — Migration to Rust

## Why

The V implementation works and meets its budgets, but the toolchain costs more than the
language saves: six patches to the compiler and standard library, a ban on capturing closures
because none is ever freed, caps on `unsafe`, a page of code generation traps, and standard
library bugs that reached users (an environment walk that lost `PATH`). Roughly a sixth of
`core` exists only to work around the language.

Rust removes that whole class of cost. It is also the one moment when changing the
architecture is free, because every line is rewritten anyway. So the rewrite is a redesign.

## Goals

1. **Work proportional to the change.** A change costs what it touches, never what the window
   holds. Incremental layout and never rebuilding the whole UI are the point of Quadra. This is
   checked with counters, not only with timers (14-performance).
2. **Performance first**: frame time, memory, start-up, binary size, and build time.
3. **Headless testability** of everything above the OS boundary, with no desktop session.
4. **Windows first** (Win32, D3D11, DirectWrite), with boundaries a second OS can implement.
5. **Few dependencies**, each justified (01, R15).
6. **The framework is the product.** Quadra is finished first. The apps (Notes, Folio, Cluster,
   OneTerm) are ported afterwards and are not part of this plan.

Non-goals for the first Rust release: the apps, a second OS, RTL editing, a web target, hot
reload, a render thread, a DSL or macro language for UI.

## What carries over

On merit, not because it exists:

- **Specs of behaviour** (in `specs/behaviour/`): the design language (15), component behaviour (09, 16, 19), the
  terminal with its limits (17), the editor (18), Markdown rendering (21).
- **Test assets**: the terminal corpus and limit tests, fuzz seeds, Markdown fixtures, the
  layout cases, the screenshots used for design review, the bench scenarios and their numbers.
- **Data**: themes as JSON, fonts, icons, the HLSL shaders (edited for the new instance
  format), the Unicode table generators.
- **Design ideas that were right**: backends behind interfaces with fake implementations; one
  instance format and one pipeline; logical pixels become device pixels only at paint; effects
  delivered when the outermost entry point returns; ownership that releases timers, tasks and
  subscriptions together; layout by requests and answers; coalesced delivery from reader
  threads; the device created on a worker thread at start-up with a software device first.

Everything else is designed again in 01–14.

## A new repository

The Rust implementation lives in a new repository. The V repository is frozen at its last
commit and kept as the reference for behaviour, tests and measured numbers.

```
Cargo.toml  rust-toolchain.toml
crates/quadra-*        the framework (02-architecture)
examples/              the gallery (the kit's acceptance test); small demos live with their crates
bench/                 benches, the comparison runner, the tool that draws the gallery to pictures
assets/                themes, fonts, icons, shaders
specs/                 these specs, and the behaviour specs carried from the V repository
docs/                  progress.md, benchmarks.md
tools/                 check.sh, setup.sh, table generators, the shader build
```

The V specs these refer to (02, 04, 05, 08, 09, 10, 13, 15–19, 21) are kept under their
numbers in `specs/behaviour/`. They are written as `behaviour/NN` here. Their API sections
describe the V code and are superseded; their behaviour is the reference.

## Toolchain and rules

The hard rules are in `CLAUDE.md`. They are those of the V repository, with two exceptions
the user granted on 2026-11-11 for the build tool:

- **Cargo is the fork that shares build output between projects.** It keeps intermediate files
  and built dependencies under the user's cargo home, ignores every target-directory setting,
  and puts only final binaries in `./target`. So the Rust toolchain and build output are **not**
  inside the project directory. Everything else still is: scratch files in `.tmp/`, downloads
  and caches in `.cache/`.
- **Its Windows installer is a PowerShell script**, run once. No other PowerShell is used and
  the repository holds no `.ps1`.

Around that:

- Nothing in the repository depends on the fork: standard cargo builds the same workspace. If
  the fork falls behind or misbehaves, the fallback is standard cargo with its target directory
  in `.cache/target`.
- `tools/cargo.sh` runs cargo with the fork's background self-update turned off, so the build
  tool changes only when someone updates it on purpose.
- **Target**: `x86_64-pc-windows-msvc`. Its linker and the Windows SDK come from the Visual
  Studio Build Tools, a system prerequisite like Git Bash; `tools/setup.sh` checks for them and
  says what to install.
- **Toolchain pinned** in `rust-toolchain.toml`; stable only, no nightly features.
- **The API documentation is built by the toolchain's own cargo** into `target/docs`, not by
  the fork, which hangs at the end of a `cargo doc` that had work to do (seen 2026-11-14).
- **Files at most 300 lines**, Conventional Commits, the clipboard
  rule: unchanged.

Gate: `tools/check.sh` runs `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`,
the unsafe policy (01, R16), the dependency list against 01 (R15) and the 300-line rule.
`tools/check.sh prod` adds the release build, the release tests and the
binary-size check.

## Phases

Each phase ends with its gate green and `docs/progress.md` updated.

| Phase | Builds | Exit |
|---|---|---|
| **R0** Repository and toolchain | The new repository, the workspace, `tools/`, the gate, `quadra-prim` | The gate runs on the empty workspace |
| **R1** Vertical slice | `reactive`, `layout`, `tree`, `text` with the fake backend, `render` with the null backend, `testing`; then D3D11, DirectWrite, a Win32 window | Counter, a 10,000-row list, a 1,000-button grid and a long document run. Every structural budget of 14 passes. The comparison bench beats the V baseline or the gap is explained. **This is the go/no-go for decisions R1–R5** |
| **R2** Interaction | Input, focus, keymap, IME, scrolling, overlays, virtual list, text input, accessibility | The interaction tests of the V suite, rewritten against the semantic tree, pass |
| **R3** Kit | Core components and the gallery | The gallery matches the V screenshots within the snapshot tolerance |
| **R4** Domain crates (done 2026-11-14) | `quadra-code`, `quadra-md`, `quadra-data`, `quadra-viz` | Their tests and fuzzers pass; their gallery pages match |
| **R5** Release | App services (settings, i18n, assets, logging), shell services, API documentation, packaging of the gallery, the manual checks on a real desktop | Quadra is complete; the apps can start |
| **R6** Base components (done 2026-11-14) | Components without a look (17), open tokens, and the paints design systems use; see "R6" below | `quadra-design` is built on the base alone and the gallery looks as before; a second kit of another look is made from the base without touching it |

### R6: base components (planned 2026-11-14, started 2026-11-14, done 2026-11-14)

The user's long-term plan: components without a look, so that a user builds a kit of their
own to the design system they want (one like a vendor's, or a brand's own). Started by the
user on 2026-11-14; specs/17 is the design of the base and of the open tokens.

- **A crate of base components** (`quadra-headless`): each component is what it does, and none
  of how it looks: its state (open, selected, value), its keys (actions in the keymap), its
  focus and focus scope, its semantics, how a popup of it closes (Escape, a press outside).
  It gives its **parts** (a trigger, the content, an item, a thumb, a track) to the kit to
  draw, each with the states a style follows (`State::SELECTED`, expanded, disabled). What
  is hard in the kit today (popups, menus, the selection controls, the table, the tree, the
  slider, the combobox, the calendar) moves there; it is not written again.
- **`quadra-design` becomes one look over the base**: the proof that a user can make a kit is
  that ours is made so.
- **Open tokens**: the roles, radii, spacings and type of a theme are the kit's own, named by
  it; the base names none. `Theme` and `theme_from_json` lose what binds them to the kit's
  names.
- **Tests**: the base through the semantic tree, with no pixel; each look by its pictures
  (`tools/shots.sh`).
- **The renderer**, for what design systems draw, in this order: gradients of several stops
  (linear, radial, conic, mixed in Oklab); dashed and dotted borders; a measure of the GPU's
  time a frame (D3D11 timestamp queries) before what costs it; shaders by class of instance
  when the measure says the one shader has grown too costly; a blur of what is behind a box
  (passes of the backdrop, kept to what changed); clips by any shape (masks in the glyph
  atlas). Later, only when asked for: perspective, images drawn as pixel art. Shaders by class
  stay waiting on the measure, which says the one shader costs a frame about one percent of
  its time (docs/benchmarks.md).
- **Order**: the spec of the base and the open tokens first, then the base moved out of the
  kit with the gallery unchanged, then the renderer's cheap parts (stops, dashes), then the
  measure, then the rest.

R1 comes before any kit work on purpose: the paradigm is proven with numbers on four small
programs before the components are written against it.

**`quadra-term`**: on hold from 2026-11-11, resumed by the user on 2026-11-15 over the
user's own terminal engine, `oneterm-vt` (18): the parser, grid, limits and corpus are the
engine's and tested there; Quadra writes the view, the input and the delivery.

**After Quadra**: the apps, each from its spec. 1Term (the V repository's OneTerm, redesigned
by the user) is first, from 2026-11-15 (19); Notes, Folio and Cluster follow. Decisions that
belong to an app, such as 1Term's SSH engine, are taken when its phase starts.

## Risks

| Risk | Answer |
|---|---|
| The reactive, build-once model is new to this codebase | R1 is a go/no-go; R2 of 01 names the fallback and what it costs |
| Build time and memory on a small machine | Compile-time policy (01, R17); a build-time budget in 14; shared build output |
| Binary larger than the V build's 1.6 MB | Budget in 14; few dependencies (01, R15); measured in R1 |
| Windows Defender removes test binaries, as it does for the V release gate | Never touch Defender; report the binaries it removes |
| Layers multiply draw calls, or too few make repaints large | Promotion rules and a debug budget (07) |
| The build tool is a third-party fork that replaces cargo for the whole machine | Self-update off; standard cargo is always a working fallback |

## Decisions taken by the user (2026-11-11)

| Question | Decision |
|---|---|
| Language and scope | Rust; Quadra stays its own framework; redesign, not translation |
| UI paradigm (01, R1 and R2) | Retained tree, signals, views built once |
| Layout | Quadra's own engine (01, R8) |
| Dependencies | Limited, not zero (01, R15) |
| Accessibility | A crate may provide the OS adapter; the requirement is that everything is testable headless (01, R11) |
| OS bindings | The `windows` crate |
| Build tool | The cargo fork; the two hard rules it breaks are waived for it |
| Target | `x86_64-pc-windows-msvc` |
| Repository | A new one |
| Order | Quadra first; the apps afterwards; `quadra-term` on hold |
| Repository on disk | `D:\quadra`; the remote is set up later |

Open: the remote repository and its name.

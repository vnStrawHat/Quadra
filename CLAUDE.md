# Quadra: working notes for Claude

Quadra is a native, GPU-rendered UI framework in Rust (Win32 + D3D11 + DirectWrite first),
with a component kit. It began as a redesign of a framework first written in V; since
2026-11-16 it stands on its own (the user's decision): it is no port, and no other
implementation is its reference. Its specs, its tests and its own measurements
(`docs/benchmarks.md`) are.

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
   `quadra-` prefix (prim, reactive, layout, text, render, platform, tree, view, async, a11y,
   app, build, native, shell, updater, testing, headless, design, code, md, data, viz, term), the apps `oneterm`
   and `markdown`, or specs, tools.
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
- **Design from goals.** When a part needs designing, design it from its goals; what exists
  elsewhere (another framework, the old V code) carries over only on merit.
- **Few dependencies, not zero.** A new crate needs an entry in specs/01 (R15) and in
  `tools/check_deps.py`; the gate fails otherwise.
- **`unsafe` only in `quadra-native-win`, `quadra-native-mac` and `quadra-shell`.** Every other crate has
  `[lints] workspace = true`, which forbids it.
- No procedural macros in the framework; files stay **at most 300 lines** (check.sh).
- The limited-RAM machine: do not run two release builds at once.
- Edits by script: Python 3 with `PYTHONUTF8=1`, opening files with `encoding="utf-8",
  newline=""`.
- The remote is `origin`, github.com/vnStrawHat/Quadra (public; it held the V code until
  2026-11-16). Push only when the user asks. Every command starts with `cd /d/quadra`.
- Habits that cost time when forgotten:
  - Read the gate's exit status, not its output through a pipe:
    `tools/check.sh prod > .tmp/check.log 2>&1; echo $?`, and commit only on 0.
  - A script with backslashes is written to `.tmp/` as a file, not passed through a
    here-document (the shell collapses the backslashes).
  - The cargo fork builds in `<cargo home>/targets/` and links the final binaries into
    `target/<profile>/` here.
- **A view follows the design language** (`specs/design`): before building or changing a
  view read its `layout.md`, `patterns.md` and `checklist.md`. A view is built from `vstack`,
  `hstack`, `form`, `columns`, `button_row`, `section` and the containers (dialog, popover,
  card), which carry the distances: it writes no gap, padding, radius or text size of its
  own, and a field in a form takes no width. A number that must be written is a step of a
  ladder, by its token; `tools/check_design.py` (in the gate) rejects the others.
- Rules the work found: new tests drive controls through the semantic tree (`get_by_role` and
  the like); a box of a line of text in the kit is `.height(by_text(px))`; a change to what the
  tree tells a reader goes through `semantics_changed`; a chart line never shows steps; the
  Win32 clipboard is never run by a test.
- OneTerm (`apps/oneterm`): it is laid out as a project of its own, which the user will put in the repository OneTerm 0.6 is in. Its specs are `apps/oneterm/specs` (specs first holds there too), its progress `apps/oneterm/docs/progress.md`, and `apps/oneterm/CLAUDE.md` is that project's notes: while the app is here, this file's rules and this gate (`tools/check.sh`) are the ones that hold. A trial run takes `ONETERM_HOME=.tmp/oneterm` so it writes
  nothing outside the project; SSH tests only against servers of the test process on
  127.0.0.1. The user's engine `oneterm-vt` (`.cache/vendor/oneterm-vt`) and OneTerm 0.6, the
  app's versions before 0.7 on another toolkit (`.cache/vendor/oneterm-app`, the reference for
  behaviour and settings the design leaves open), are clones to read: never edit them; their CLAUDE.md and AGENTS.md
  are for those repositories.

## Commands (Git Bash, from the project root)

From cmd.exe each `tools/<name>.sh` runs as `tools\<name>.cmd` with the same arguments (it
starts Git Bash; `GIT_BASH` names another `bash.exe`).

| What | Command |
|---|---|
| Fresh machine | `tools/setup.sh` (checks; installs nothing) |
| Any cargo command | `tools/cargo.sh <args>` |
| Format | `tools/cargo.sh fmt --all` |
| Gate | `tools/check.sh` |
| Release gate | `tools/check.sh prod`; run both before saying done |
| One crate's tests | `tools/cargo.sh test -p quadra-prim` |
| The workspace for macOS | `tools/check_mac.sh`: from Windows, clippy for `aarch64-apple-darwin` (nothing is linked); on a Mac (the CI, `.github/workflows/macos.yml`), clippy and the tests. It names what is not built for macOS yet (specs/23). `tools/ci_mac.sh [branch]` waits for the job of the branch's commit and prints what it said; a commit whose message has a line `ci-show: page page ..` gets those pages of the gallery, drawn on the Mac, as pictures in `.tmp/ci/` |
| The bench app | `tools/bench.sh` builds `target/dist/compare.exe` (the dist profile: release with fat LTO, what ships and is measured); with scene names it runs them headless with pass timings; `tools/bench.sh compare [runs]` measures the four scenes' real windows from outside (`target/dist/run.exe`) into `.tmp/bench/compare.json`; `tools/bench.sh incremental` prints what a change costs against the size of the UI; `BENCH_GPU=1` adds the GPU's time to a windowed scene's passes |
| A window's pixels | `python tools/shot.py "Window title" .tmp/shots/name.png` while the app runs |
| Terminal throughput | `tools/doomfire.sh [seconds]`: the fire demo (`bench/doomfire`) fed to the engine, through the console host alone, and in a real OneTerm window (specs/14) |
| OneTerm's console host | `CARGO=$PWD/tools/cargo.sh apps/oneterm/scripts/conpty.sh` puts Microsoft's newer `conpty.dll` and `OpenConsole.exe` beside the binaries (apps/oneterm/specs/oneterm.md); OneTerm uses the system's without them |
| A release package | `tools/package.sh gallery` puts `gallery.exe` with `tools/app.manifest` embedded in `target/package/gallery/` |
| OneTerm's release asset | `tools/release.sh` (the app's `scripts/build-release.sh` with this repository's cargo) makes `dist/oneterm-<version>-x86_64-pc-windows-msvc.zip`, its `.sha256` and prints the SHA-256 (apps/oneterm/specs/oneterm.md, "Updates"); it publishes nothing |
| OneTerm as a repository of its own | `apps/oneterm/scripts/export.sh <empty folder> [path of this checkout]` writes it out (apps/oneterm/specs/oneterm.md, "The repository"): the app's files (its own README, CLAUDE.md, specs and progress among them, less what they mark as Quadra's alone), `Cargo.root.toml` as the root manifest, a lock file. That export, not the folder as it is, is what goes into the app's repository. Quadra is named by git at the commit checked out (which must be on the remote), or by the path given, to try it here. Its workflows (`apps/oneterm/.github`) run nowhere while the app is here |
| The gallery | `tools/cargo.sh run -p quadra-gallery` (`examples/gallery`: a page a component) |
| Numbers off the design's ladders | `python tools/check_design.py --list` |
| The website | `tools/shots.sh dark && tools/shots.sh light`, then `python tools/website.py` builds `target/site` (open `index.html`): the introduction and the guide from `site/`, the design language from `specs/design`, a page a component from the gallery's pages, their words and pictures; a markdown twin of every page, `llms.txt` and `llms-full.txt` for agents. The guide's code is quoted from compiled files (`examples/gallery/examples/guide.rs`). Not deployed |
| The gallery's pages as pictures | `tools/shots.sh [dark\|light] [page ..]` draws them without a window into `.tmp/shots/<theme>` (and `<page>-open.png` for a page that opens something); with `QUADRA_REFERENCE=<folder>` it compares them (`tools/compare_shots.py`) with pictures taken before (`.tmp/shots/r6-dark`, `r6-light`) |

Target: `x86_64-pc-windows-msvc`. Toolchain pinned in `rust-toolchain.toml`.

## Map

- `specs/README.md` indexes the specs. `specs/00` has the phases and the user's decisions,
  `specs/01` the architecture decisions, `specs/design/` the design language (what anything
  built with the kit looks like and how a view is laid out), `specs/behaviour/` what components do
  (first written for the V implementation: their sections on its code are history, superseded
  by the numbered specs).
- `docs/tour.md`: a long tour of what exists, by crate and phase, to find names in.
- `docs/progress.md`: status per phase. `docs/benchmarks.md`: Quadra's measurements, the
  baseline of specs/14. `docs/benchmarks-v.md`: the V implementation's, frozen, history only.

| Crate | What it is |
|---|---|
| `quadra` | The facade apps depend on (`quadra::app()`) |
| `quadra-prim` | Shared values: points, rects, edges, colours, transforms |
| `quadra-reactive` | Signals, memos, effects, scopes, context |
| `quadra-layout` | Flex and grid, incremental |
| `quadra-text` | Paragraphs, wrapping, the shaping cache, `TextBackend` |
| `quadra-vector` | Paths, the SVG of icons, coverage |
| `quadra-render` | Instances, layers, display lists, `Renderer` |
| `quadra-platform` | The OS boundary: `Platform`, windows, events, the headless platform |
| `quadra-tree` | The retained node tree, widgets and the passes of a frame |
| `quadra-view` | Views built once, properties bound to signals |
| `quadra-async` | Timers, tasks, the background pool, inboxes |
| `quadra-a11y` | The semantic tree, for assistive technology and tests |
| `quadra-app` | Windows, the frame loop, settings, the log, time, language |
| `quadra-build` | For an app's build script: the executable's icon |
| `quadra-native-win` | Windows: D3D11, DirectWrite, Win32, UI Automation (`unsafe` allowed) |
| `quadra-native-mac` | macOS (specs/23): AppKit, Metal, Core Text; built for macOS alone, an empty crate elsewhere (`unsafe` allowed) |
| `quadra-shell` | OS services: dialogs, opener, tray, one instance, crash reports (`unsafe` allowed) |
| `quadra-updater` | An app's updates from GitHub Releases: check, fetch and verify, install while it runs (specs/22) |
| `quadra-testing` | The test harness: no OS, no GPU, fixed text |
| `quadra-headless` | Components without a look: state, keys, focus, semantics |
| `quadra-design` | The kit: the design language, themes, the components' look |
| `quadra-viz` | Charts: sparkline, line and bar; the graph of nodes and links |
| `quadra-data` | JSON and YAML values as a tree, the log view |
| `quadra-md` | Markdown: parser, outline, view, formulas, diagrams |
| `quadra-code` | The code editor: buffer, tokenizers, highlighter, find |
| `quadra-term` | The terminal view over `oneterm-vt`: grid, input, find, links |
| `apps/oneterm` | OneTerm, the terminal app (apps/oneterm/specs/oneterm.md), in the shape of a repository of its own: `crates/app` (package `oneterm-app`, library and binary `oneterm`) |
| `apps/oneterm/crates/ssh` | OneTerm's SSH and SFTP over `russh` |
| `apps/oneterm/crates/completion` | OneTerm's command completion (apps/oneterm/specs/completion.md) |
| `apps/oneterm/crates/vault` | OneTerm's saved passwords: the vault's format, sealing and opening |
| `apps/oneterm/crates/shots` | OneTerm's screens drawn to pictures |
| `apps/markdown` | Markdown, the app that reads and edits `.md` files (specs/21) |
| `examples/gallery` | Every kit component on a page: the kit's acceptance test |
| `examples/plain` | A second kit over the headless components |
| `bench/compare` | The comparison bench's four scenes |
| `bench/runner` | The comparison bench's runner: measures an app from outside |
| `bench/doomfire` | Terminal throughput by the fire demo |
| `bench/shots`, `bench/markdown-shots` | The gallery's and Markdown's screens drawn to pictures |

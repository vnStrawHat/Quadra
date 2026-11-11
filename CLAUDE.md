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
| The bench app | `tools/bench.sh` builds `target/release/compare.exe`; with scene names it runs them headless with pass timings (docs/benchmarks.md has the runner's command) |
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

Phase R0 is done. **Phase R1, the vertical slice, is done** (docs/progress.md, "The exit of
R1"): the four scenes of the comparison bench run in real windows, and against the V build
every time is equal or a little better (list 0.36 ms, grid 1.29, document 0.70, start-up
41 ms, binary 0.88 MB; docs/benchmarks.md). Idle memory is 1 MB more. The decisions the phase
tested (specs/01, R1 to R5) stand.

What exists, in short (`docs/progress.md` has each crate with what it lacks and its tests):
`quadra-reactive`, `quadra-layout` (done); `quadra-render`, `quadra-text`, `quadra-tree`,
`quadra-ui`, `quadra-testing` (passes, layers, atlases, glyph tiles, views, virtual list,
scroll with culling, frame callbacks); `quadra-platform` (traits, the headless platform),
`quadra-app` (windows, the frame loop, recovery), `quadra` (the facade, `quadra::app()`);
`quadra-native-win` (Win32 platform, D3D11 with swap chains and the two-device start,
DirectWrite). Its shader is `shaders/prims.hlsl`, compiled by `tools/build_shaders.sh` (needs
the Windows SDK's fxc) into committed `.cso` files. Its tests use windows that are never shown.

**Phase R2, interaction, is in progress** (specs/00, Phases; specs/09). Done: pointer input,
from the platform's events through hit testing and dispatch (capture, bubble, stop, pointer
capture, hover, press, clicks with counts) to `on_click` and `on_pointer` on views; the
bench's counter counts real clicks. Styles have state variants (`.hover(..)`, `.active(..)`,
`Patch` and `State` in `quadra-tree/src/style.rs`); tokens and themes are R3. The wheel
scrolls `scroll` and `uniform_list`, nested ones included. Focus and keys are in
(`quadra-tree/src/focus.rs`: press to focus, Tab inside a focus scope, the ring, keys along
the focus path, `NodeRef::focus`), and so are actions and the keymap (`action.rs`,
`keymap.rs`; `App::keymap`; Tab is a binding). **The text input is done** (`text_input(value)` in `quadra-ui/src/input.rs`, the `TextField`
widget and `TextCommand` in `quadra-tree/src/field.rs`, the buffer in
`quadra-text/src/edit.rs`): typing, the editing keys, pointer selection, undo, the clipboard
(the app provides it as context; the Win32 one is never run by a test, and must not be), a
caret that blinks on a timer (`quadra-async`: `timeout`, `interval`; `Harness::mount` and
`Harness::advance` for tests), and input methods (`PlatformEvent::Composition`,
`PlatformWindow::set_text_input`, `quadra-native-win/src/ime.rs`). `crates/quadra/examples/form.rs`
shows it in a real window. One manual check is open: composing with a real input method
(Vietnamese Telex) on a desktop.
Overlays are in (`quadra-tree/src/overlay.rs`, `overlay(place, view)` in `quadra-ui`; the
example has a menu). The semantic tree is in (`quadra-a11y`; `quadra-tree/src/semantics.rs`;
`.role(..)`, `.label(..)`, the state setters; the harness finds controls with `get_by_role`
and the like and drives them with semantic actions: prefer these in new tests). The bridge
to assistive technology is in (`PlatformEvent::SemanticsWanted` and `Semantic`,
`PlatformWindow::update_semantics`; `quadra-native-win/src/a11y.rs` with the adapter of
`accesskit`; `tests/access.rs` reads a hidden window through UI Automation). The adapter
panics on an update that is not well formed (specs/12 lists the rules): a change to what the
tree tells a listener goes through `semantics_changed`, and `quadra_a11y::Mirror` in the
random test is what catches a missed case. Lists with rows of different heights are in
(`virtual_list` beside `uniform_list` in `quadra-ui/src/list.rs`, heights in
`list/heights.rs`, `Tree::after_layout`), and so are the tab index, cursors and scrolling
into view (`Tree::scroll_into_view`, `on_reveal`; `.reveal(..)` on lists). **Phase R2 is done**
(docs/progress.md, "The exit of R2", maps each interaction test of the V suite to its test
here). Since the tab index: cursors, scrolling into view (`Tree::scroll_into_view`,
`on_reveal`; `.reveal(..)` on lists), the password input and `.on_submit`, the text area
(`text_area`; `TextField::sync`, `Paragraph::place`), text that can be selected, drag and
drop (`quadra-tree/src/drag.rs`, `quadra-ui/src/div/drag.rs`, the example `board`), and
`tween` on the window's clock (`Tree::set_clock`, `Handle::now`). **R3 has started** (specs/13;
the design language is specs/behaviour/15): colours of box styles can be roles of the theme
(`Paint`, `Token`, `Theme` in `quadra-tree/src/theme.rs`; `Tree::set_theme`; the app's theme
is a `Signal<Rc<Theme>>` in the context of views), and the crate `quadra-kit` has the names
of the roles (`quadra_kit::token`) and the themes (`dark()`, `light()`, `theme_from_json`;
the files are in `crates/quadra-kit/themes/`). Text values hold down the tree
(`quadra-tree/src/inherit.rs`: `TextValues`, `Widget::inherit`, the tree's `text_epoch`; a
text has no style of its own). The kit has its fonts (`quadra_kit::fonts`), its numbers
(`size`), `install(app, &theme)` and `root()`; `crates/quadra-kit/examples/themes.rs` shows
both themes in a real window. The button is in (`quadra-kit/src/button.rs`; `action.rs`
has `Confirm` and the kit's keymap; the example `gallery` shows it), and a disabled node is
not clicked, focused or tabbed to (specs/09). `quadra-vector` is ported (paths, the SVG
of icons, coverage), and the kit has icons (`icon(icons::CHECK)`; the widget is `Drawing`
in `quadra-tree/src/drawing.rs`, its masks are tiles of the glyph atlas) and the text
input (`input(value)`; placeholder, selection and caret colours are text values), and the
selection controls (`checkbox`, `switch`, `radio_group`, `segmented`; `outline` is the
focus mark of what has no border), and popups (`popup.rs`: focus in and back, Escape, a
press outside; `popover`, `menu`, `select`), the tooltip (no node of its own: handlers on
the view's node, a timer, an overlay opened by hand) and the dialog (`Place::Top`), and
shadows (`.elevation(1|2)`, a paint value), `tabs` and `list` (boxes have `.display(..)` and
one-sided borders now). Every scroll view and list has a scrollbar (`quadra-ui/src/scrollbar.rs`,
the widget `ScrollThumb`: a layer of its own, shown on activity, faded by a timer). Next: panels and the titlebar; the components in the order of specs/13, each with its
section in the gallery example and its test through the semantic tree; tokens for radii and spacing, style transitions
(`.transition(..)` of specs/05, not built), then the components in the order of specs/13,
popovers and hover cards among them; the rest of `quadra-async`
(tasks, the pool, inboxes) when the first thing needs it. For readers still to do: text ranges and the caret of an input, range
values, positions in a set, a role for scroll views.

Room left that the numbers show (`tools/bench.sh grid`): 190 ns a binding in the grid; a flex item is measured once per height its column offers though its
height does not depend on it; a new painted band visits every node of its layer.

Habits that cost time when forgotten:

- Read the gate's exit status, not its output through a pipe:
  `tools/check.sh prod > .tmp/check.log 2>&1; echo $?`, and commit only on 0.
- A script with backslashes is written to `.tmp/` as a file, not passed through a here-document
  (the shell collapses the backslashes). `.tmp/edit.py FILE SPEC` applies exact replacements.
- The cargo fork builds in `<cargo home>/targets/` and links the final binaries into
  `target/<profile>/` here.
- The V runner and bench are built in the V repository (`tools/v.sh -cc clang -prod ...`; the
  runner needs `-cflags -Wno-int-conversion`).

`quadra-term` is **on hold** by the user's decision: do not build it until told. The apps come
after Quadra is complete.

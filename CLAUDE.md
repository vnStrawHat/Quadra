# Quadra: working notes for Claude

Quadra is a from-scratch, GPU-rendered native UI framework in V (Win32 + D3D11 + DirectWrite
first; Linux and macOS designed for later), with a component kit and real apps (gallery,
Notes, Folio, Cluster). It was called "Vela" until 2026-11-08; old notes may say so.

## Hard rules from the user (never break them)

1. **Stay inside the project directory**: toolchain, caches, downloads and temp files
   included. Never use the system temp dir, the scratchpad, `mktemp`, `~/.vmodules` or the
   home dir. `tools/v.sh` points VMODULES, VCACHE, VTMP, TEMP and TMP at `.cache/` and
   `.tmp/`; scratch files go in `.tmp/`.
2. **No PowerShell**: Git Bash + coreutils only, never `.ps1` (check.sh rejects them).
3. **Conventional Commits** for every commit message (`feat(kit): ...`, `fix(core): ...`,
   `docs(specs): ...`, `refactor: ...`, `test: ...`, `chore: ...`, `perf: ...`; `!` or a
   `BREAKING CHANGE:` footer for breaking API changes). Scopes are module or app names:
   core, ui, kit, text, gpu, layout, prim, platform, app, testing, gallery, notes, folio,
   cluster, specs, tools.
4. **Never name the projects the design was researched from** anywhere in the repo (code,
   comments, docs, specs, commit messages). Describe traits instead; the design language
   is called "One" (specs/15). The list is in `.private/forbidden-names.txt`, and
   `tools/check.sh` fails when one of them appears. Talking about them in chat is fine.
5. **Never test the real clipboard.**

## How to work

- Chat with the user in **Vietnamese**; code, comments, docs, specs and commits in
  **English**.
- **Performance first.** Specs come before code: change `specs/` first, then implement,
  then update `docs/progress.md` (the source of truth for what is done).
- Files stay **at most 300 lines** (check.sh). The framework has a **closure cap**
  (`closure_cap` in check.sh, only ever lowered): avoid `fn [..]` in `quadra/`, use the
  closure-free patterns in specs/14 (`core.handler_with(data, fn)`, `themed_with`, typed
  actions, `MenuAction` objects).
- Read `docs/v-notes.md` before fighting the compiler: V 0.5.2 has many codegen traps.
  More: no multi-value `if` expressions (assign to `mut` vars in an `if` block); single-letter
  struct names are reserved; globals stored through `cx.set_global` must be `pub` types;
  never add `#flag -lgdi32` (it makes V build a console-less GUI exe; load gdi32 at run
  time with `win.load_system_dll`).
- Edits by script: Python 3 with `PYTHONUTF8=1` and `newline=''` on read and write (the
  defaults are cp1252 and CRLF, which break Vietnamese strings and vfmt).
  `tools/ed.py` applies exact replacements from a spec file and fails on 0 or 2+ matches.

## Commands (Git Bash, from the project root)

| What | Command |
|---|---|
| Fresh machine | `tools/setup.sh` (V 0.5.2 into `.tools/v`, MSVC check) |
| Format | `tools/v.sh fmt -w quadra apps tests examples bench` |
| Gate (about 35 s) | `tools/check.sh` |
| Release gate (about 6 min, MSVC -prod) | `tools/check.sh prod`; run both before saying done |
| One test file | `tools/v.sh test tests/kit3_test.v` |
| Type-check a module | `tools/v.sh -check quadra/kit` (a "main module" error is expected) |
| API docs | `tools/docs.sh` (into docs/api, not committed) |
| Screenshots for design review | `tools/v.sh -cc msvc -prod -o .tmp/shots.exe bench/shots && .tmp/shots.exe .tmp/shots/dark dark` |
| Run an app | `tools/v.sh run apps/gallery` |

More in README.md. Requirements: Git Bash with coreutils, Visual Studio Build Tools (C++
workload + Windows SDK; `fxc` only when editing HLSL), Python 3 and curl.

## Map

- `quadra/`: `prim` (Point, Rect, Color: the shared value types), `core`, `ui`, `layout`,
  `text`, `gpu`, `vector`, `platform`, `native`, `app`, `testing`, `kit` (components; themes
  are JSON files in `quadra/kit/themes/`, loaded by `kit.load_theme`).
- `specs/README.md` indexes the specs; specs/13 has the roadmap and the After-v1 list,
  specs/15 the design language and themes, specs/14 implementation rules.
- `apps/`: gallery (one page per component), notes, folio (Markdown), cluster.
- `tests/`, `examples/`, `bench/` (benchmarks, leak probe, shots), `tools/`.

## Private material (`.private/`, git-ignored)

Research notes, the user's reference screenshots, the forbidden-names list and Claude's
memory from the first machine. It is copied by hand, not through git. If it exists, read
`.private/README.md` at the start of a session; if it is missing, ask the user to copy it.

## State at the last handoff (2026-11-08)

All milestones M0-M10 are done (docs/progress.md). The latest session: One Dark Pro is the
only dark theme and themes load from JSON; the project was renamed Vela -> Quadra and
`geom` -> `prim`. Open: the switch's on-track color (see `.private/README.md`), color emoji
(monochrome today), the M0 manual checks, the 150 ms start-up budget, the After-v1 list.
`dist/` still holds builds packaged under the old name; `tools/package.sh` rebuilds them.

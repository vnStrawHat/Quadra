# 13 — Apps and Roadmap

Three apps ship with the framework. The gallery shows every component. The Notes app proves the framework can carry a real product. The cluster dashboard (specs/16) proves the developer-tool kit.

## Gallery (`apps/gallery/`)

```
+----------------------------------------------------------------+
| Quadra Gallery                    [search]  [theme] [density] [vi] |
+-----------------+----------------------------------------------+
| Getting started | Button                                       |
| Typography      | Variants:  [Primary] [Secondary] [Ghost] ... |
| Colors          | Sizes:     [xs] [sm] [md] [lg]               |
| -- Forms --     | States:    [Disabled] [Loading]              |
| Button          |                                              |
| Input   ...     | Code:  kit.button('save').label('Save')...   |
+-----------------+----------------------------------------------+
```
- One page per component (never grouped), registered in `pages/registry.v` and created lazily. Each page shows variants, sizes, states and a code snippet, in the One style (specs/15).
- Extra pages: Typography (sizes, Vietnamese paragraph, IME test field), Colors (all tokens), Layout (flex and grid playground), Stress (1,000,000-row table, 1,000-button grid, live profiler numbers).
- A component is not done until it has a page and a harness test for its interactions.

## Notes (`apps/notes/`), the reference application
A small but complete notes app, built only with public framework APIs.

| Area | What it exercises |
|---|---|
| Sidebar with folder tree, note list, editor in a resizable split | `tree`, `virtual_list`, `split`, `textarea`, persisted UI state |
| Notes stored as files in `data_dir`, loaded and saved in background tasks | tasks, cancellation, debounced autosave |
| Full-text search in a background index, `ctrl-p` command palette | tasks, `command_palette`, keymap |
| Open a note in its own window | multi-window, shared models across windows |
| Table view of all notes (title, folder, modified, size) | `table` sort, resize, selection |
| Drag files in to import, drag notes between folders | file drop, internal drag and drop |
| Settings: theme or follow system, density, language vi/en, shortcuts | settings, themes, i18n, keymap overrides |
| Custom titlebar, close with "save changes?" | titlebar regions, snap layouts, close flow, `confirm` |
| Tray icon with quick note (optional setting) | tray, `quit_on_last_window` |
| Harness tests for every main flow | headless testing, scene snapshots, render-count assertions |

## Milestones

| # | Goal | Exit criteria |
|---|---|---|
| M0 | **Spike**: validate the risky decisions (done 2026-11-07 except the items below, see spikes/m0/RESULTS.md) | Win32 window, D3D11 flip swap chain with waitable, one rounded quad from a precompiled shader, "Xin chào Việt Nam" shaped and rasterized by DirectWrite through the vtable-index COM helper, correct at 100% and 150% DPI, IME events logged. **Headless proof**: WARP offscreen render plus DirectWrite run from a non-interactive scheduled task and write a PNG. V checks: generic methods with extra type parameters on generic receivers (`ModelCx[T].subscribe[M, E]`), generic function types as parameters, `Renderable[T]` with type-erased `AnyView`, `EventSink` interface dispatch, `malloc_noscan` buffers. |
| M1 | Core and headless foundation | Model store, ownership, effects, tracking. Flex layout. Headless platform, null renderer, fake text, virtual clock, harness basics. Chunks, overlap ordering, D3D11 drawing. A counter app passes headless tests, and idle CPU is 0%. |
| M2 | Engine complete | Grid, incremental layout, view cache and the four invalidation levels, text caches, scrolling, focus, keymap with chords, overlays, animation, semantics, images and SVG, canvas paths. Inspector, profiler, paint flashing. Benchmarks running in CI. |
| M3 | Kit, part 1, and gallery shell | Themes with follow-system, actions, checkbox/switch/radio/slider, layout components, display components, navigation, tooltips and menus. |
| M4 | Kit, part 2 | Input, textarea, number input, select, combobox, date picker, form validation, modal, confirm, toast, table, tree, command palette. |
| M5 | App services and Notes | Settings, window state, assets, i18n, tasks, router, single instance, logging, crash dumps, packaging. Notes app complete with harness tests. |
| M6 | Hardening: **v1** | Every budget in 12-performance met on the reference machine, all snapshots green on the non-interactive CI account, API docs generated with `v doc`. |
| M7 | Workbench (specs/16) | Activity bar, tab strip (dirty, close, reorder, overflow), pane tree with resize and collapse, panel header; gallery pages and harness tests; `apps/cluster` shell on fake data. |
| M8 | Terminal | VT parser and screen with table tests, terminal view with selection and copy, fake PTY in the harness, ConPTY on Windows; budgets of specs/16 met; Shell tab in `apps/cluster`. |
| M9 | Code editor | Piece table with property tests, virtualized view, V and YAML tokenizers, multi-cursor, find/replace, undo groups; budgets met; manifest editor in `apps/cluster`. |
| M10 | Live data | Live table, log view, value viewer, sparkline and charts; `apps/cluster` complete with harness tests for every main flow. |

M0 items still waiting for the user: the session-0 run as SYSTEM (`spikes/m0/session0_check.sh`, elevated), IME composition typed by a person, and a real 150% DPI window.

| App after v1 | Status |
|---|---|
| OneTerm (specs/22-27) | P1 (local terminal), P2 (SSH, SFTP, vault) and P3 (parity with the original) done; docs/progress.md lists the known gaps. Performance work is next. |

## After v1
1. UIA accessibility provider on top of the semantic tree (first priority).
2. Linux (Wayland, then X11; Vulkan; FreeType, HarfBuzz and fontconfig) and macOS (AppKit, Metal, CoreText).
3. Rich-text editor; dock layout with floating and detached panes (M7 only splits and tabs).
4. WinRT toast notifications, MSIX packaging and auto-update.
5. RTL caret movement and editing.

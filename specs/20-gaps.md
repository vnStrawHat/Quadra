# 20. Gap Analysis (2026-11-07)

Quadra was compared with three other GPU UI frameworks: a Rust framework, the component kit
built on it, and a Go framework with native and web UI. Sources: their public feature lists,
component catalogs, and published benchmark suite (docs/benchmarks.md runs the same four
scenes on Quadra). Linux and macOS are out of scope until asked for (specs/13).

Status: **have** (built), **part** (built with limits), **gap** (missing). Priority:
**P1** fill now, **P2** next, **P3** later or app-level.

## Performance
| Area | Quadra on this machine | Best published | Status |
|---|---|---|---|
| CPU per frame: virtual list scrolling | 0.46 ms | 1.46 ms | have |
| CPU per frame: 600 changing cells | 2.91 ms | 5.71 ms | have |
| CPU per frame: 20 000 chars rewrapped | 0.97 ms | 3.03 ms | have |
| Peak memory, all three scenes | 25–65 MB | 94–106 MB | have |
| Idle memory / idle CPU | 10.6 MB / 0.7–1.2 ms/s, all in the GPU driver | 33.7 MB / 0.09 ms/s | gap (idle CPU: the NVIDIA driver's thread, not Quadra's) |
| Binary size | 1.3 MB | 3.7 MB | have |
| Startup to first frame | 65 ms (first frames on WARP, then the hardware device) | 157 ms | have |

## Engine
| Feature | Status | Pri | Notes |
|---|---|---|---|
| Flex and grid layout, scrolling, uniform virtual list | have | | |
| Virtual list with variable row heights | have | P1 | `ui.list(id, count, render)` (specs/08) |
| Drag and drop between elements (payload, preview, drop targets) | have | P1 | see Designs below; `div.draggable`, `on_drop`, `drag_over` |
| Popover (anchored, click-outside and Esc to close) | have | P1 | `kit.open_popover`, `kit.popover_button` |
| Element transforms (translate, scale, rotate) and style transitions | have | P2 | `.translate` `.scale` `.rotate` (paint only), `.transition(ms)` on background, border, opacity and transforms |
| Animation timelines (sequence, parallel, repeat) | have | P2 | `.animate(ui.tween(..) + ui.pause(..))`, `ui.parallel`, `.repeat` `.forever` `.alternate` (specs/08) |
| Color emoji | have | P2 | COLR layers via `TranslateColorGlyphRun`, composed into RGBA tiles (specs/04); no bitmap or SVG glyphs |
| Backdrop blur, custom shaders | gap | P3 | |
| GPU paths | part | P3 | CPU-rasterized and cached; charts use quads |
| Accessibility (UI Automation) | have | P2 | tree, roles, names, ids, Invoke/Toggle/SelectionItem/ExpandCollapse/Value, focus events (specs/06); not yet: change events, text pattern |
| Right-to-left text and layout | gap | P3 | After v1 (specs/13) |

## Text
| Feature | Status | Pri | Notes |
|---|---|---|---|
| Shaping, fallback, ClearType, IME, selection | have | | |
| Rich text runs in one text element (bold, color, links) | have | P1 | `ui.rich_text(spans)` (specs/08); selection and inline icons later |
| Markdown view | have | P2 | `md.view(src).on_link(l)` (kit/md): headings, lists, fenced code highlighted by language, quotes, rules, pipe tables, inline styles and links; no HTML or reference links |
| Rich text editor | gap | P3 | After v1 |

## Platform and desktop APIs
| Feature | Status | Pri | Notes |
|---|---|---|---|
| Windows, popups, custom titlebar, DPI, clipboard text, single instance, crash dumps | have | | |
| Open/save file and folder dialogs | have | P1 | `cx.pick_files(platform.FileDialog{...})` |
| Open a URL or a file with its app, reveal in Explorer | have | P1 | `cx.open_external`, `cx.reveal` (ShellExecuteW) |
| Files dropped from Explorer | have | | |
| Native message box | have | P2 | `cx.message_box(platform.MessageBox{...})` (MessageBoxW) |
| OS notifications, tray icon | have | P2 | `cx.set_tray(tip)`, `cx.on_tray(l)`, `cx.show_notification(title, body)` (Shell_NotifyIconW v4); no tray menu helper yet (open a kit menu at the event point) |
| Global shortcuts, start at login, file watching | gap | P3 | |
| Clipboard images and other formats | gap | P3 | |
| Web view | gap | P3 | WebView2 needs its runtime |

## Components
| Component | Status | Pri | Notes |
|---|---|---|---|
| 59 gallery pages (forms, navigation, overlays, data, workbench, terminal, editor, charts) | have | | |
| Popover, hover card | have | P1 | `kit.hover_card(child, build)` |
| Drawer (sheet from an edge) | have | P2 | `ModalOptions.side` (left, right, bottom) |
| Inline alert, empty state, description list | have | P2 | `kit.alert`, `kit.empty_state`, `kit.description_list` |
| Calendar (standalone), time input, range slider | have | P2 | `kit.new_calendar` + `kit.on_date`, `kit.time_input` (hh:mm segments), `kit.range_slider`; calendar days have no arrow-key navigation yet |
| Color picker, rating, stepper, pagination, OTP input, tag input | gap | P3 | |
| Table: pinned columns, cell selection | have | P2 | `Column.pinned`, `TableOptions.select_cells`, ctrl-c copies the cell or rows as TSV |
| Dock: drag a tab to another pane or an edge to split | have | P1 | `work.Dock`, `TabStrip.dock`, `Panes.on_dock` |
| Code editor: tree-sitter grammars, language server | gap | P3 | lexers plug in; a server is app-level |
| Diff viewer | gap | P3 | the editor's gutter marks exist |

## Tooling
| Feature | Status | Pri | Notes |
|---|---|---|---|
| Inspector, profiler overlay, headless harness, pixel snapshots | have | | |
| Accessibility assertions in tests | have | P2 | `h.find_role`, headless `a11y_on`; a real UIA client test (`tests/uia_windows_test.v`) |
| Driving a running app from another process | gap | P3 | the harness covers tests |

## Plan
P1 in this order: file dialogs and shell open; popover; drag and drop with the dock using
it; variable-height virtual list; rich text runs. Each gets a spec section, tests and a
gallery page. P2 follows, accessibility first.

Status 2026-11-08: every P1 and P2 row is built. What remains is P3 (after v1 or
app-level), plus the limits named in the rows above.

## Designs
**Drag and drop** (`core/drag.v`). A hitbox may carry a drag source (payload `{kind, value}`,
a preview builder, `tear_off`) and a drop kind. A left press arms the deepest source; the drag
starts after 4 px (or `tear_off` px vertically, so tabs reorder sideways first). The source is
looked up again in the latest prepaint when the drag starts, because its preview closure
belongs to the render that made it and a click that re-renders frees it. During a drag, moves
go to no handler; the press is no longer a click; escape cancels. The target is the deepest
hitbox under the pointer whose drop kind matches; `PaintCx.is_drop_target` styles it
(`div.drag_over(patch)`); the release runs its `drop` handlers with a `DropEvent`. The preview
is laid out next to the pointer and deferred above every overlay, never prepainted, so it has
no hitboxes and never hides a target. The payload is strings so no closure outlives the drag.

**Dock** (`kit/work/dock.v`, `dock_model.v`). A strip given `.dock(group, l)` makes its tabs
drag sources (`tear_off` 12) and itself a target: a tab of its own group moves to the slot
under the pointer (`TabMove`), another group's tab is reported as a `DockEvent` at that
index. `Panes.on_dock(l)` wraps each leaf in a target: the outer quarter on a side is an
edge (split there), the rest the middle (move into that group); the zone is highlighted.
`work.Dock{root, tabs, active}` is the app-side model; `apply(e)` and `move(group, m)`
update it and close a group left empty.

**Popover and hover card** (`kit/popover.v`). `open_popover(cx, anchor, placement, build)`
opens a framed panel through the overlay host (outside click and escape close it). A hover
card opens after 400 ms over its trigger and stays while the pointer is on the trigger or
the card, with 150 ms of grace to cross the gap, so links in it can be clicked.

**File dialogs and shell** (`core/shell.v`, `platform/win/dialogs_windows.c.v`).
`cx.pick_files(FileDialog{title, save, folders, multiple, filters, default_name, dir})`
shows the modal common dialog owned by the window and returns the chosen paths (empty when
cancelled). `cx.open_external(target)` opens a URL or file with its app; `cx.reveal(path)`
selects it in Explorer. The headless platform takes queued picks and records opens, so
tests never show a dialog.

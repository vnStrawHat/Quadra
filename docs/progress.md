# Progress

Status per milestone of specs/13-apps-roadmap.md. Run everything with `tools/v.sh test quadra tests examples apps`.

| Milestone | Status | Evidence |
|---|---|---|
| M0 spike | Done; 3 manual checks pending (session 0, IME typing, real 150% DPI) | spikes/m0/RESULTS.md |
| M1 core and headless | Done | tests/state_test.v, examples/counter (0.0 ms idle CPU over 3 s) |
| M2 engine | Done, with the deviations in specs/14 | tests/input_test.v, tests/pixels_m2_test.v, tests/devtools_test.v, bench/ |
| M3 kit part 1 + gallery shell | Done | tests/kit_test.v, apps/gallery (every page rendered headless, pixel snapshots in .tmp/out) |
| M4 kit part 2 | Done | tests/kit2_test.v, tests/input_field_test.v, quadra/kit/input/buffer_test.v, gallery pages Inputs, Select & Date, Forms, Table & Tree, Command palette (ctrl-k) |
| M5 app services + Notes | Done, with the deferrals in specs/14 | tests/tasks_test.v, settings_test.v, i18n_test.v, app_services_test.v, crash_test.v; apps/notes (store tests, 6 harness flows, pixel snapshots); `tools/package.sh` builds dist/quadra-notes with manifest, version info and icon; single instance, close flow and window placement checked on the real platform |
| M6 hardening | Done except two items: the startup budget on this machine (cold D3D11 driver start alone takes 150 ms) and the session-0 run (needs the user). Memory leak fixed after release (see Memory) | `tools/check.sh` (format, portability, 300-line rule, tests), `tools/docs.sh` (API reference in docs/api), tests/patch_test.v and tests/patch_pixels_test.v, kit.titlebar in Notes, benchmarks below |
| M7 workbench | Done | quadra/kit/work (pane tree data with tests in pane_test.v), tests/work_test.v (select, close, middle-click, drag reorder, activity badge, pane resize, divider double-click, fold to header, wheel over an overflowing strip), gallery Workbench pages, apps/cluster (fake cluster of 154 resources, harness flow in cluster_test.v, snapshot .tmp/out/cluster.png) |
| M8 terminal | Done, with the deviations in specs/17 (OSC 8 links not clickable, mouse motion not reported). Of the four budgets, typing echo under one frame is not measured (it needs a timer across the real PTY round trip) | quadra/kit/term (parser, screen, reflow and key tests), tests/term_test.v (echo, history, selection, copy, bracketed paste, exit, resize on a FakePty), tests/conpty_test.v (a real program on ConPTY), gallery Terminal page, Shell tab in apps/cluster (`--local` opens the system shell), bench/term below |
| M9 code editor | Done, with the deviations in specs/18 (Status) | quadra/kit/code (buffer property test, doc_test.v for movement, every command, multi-cursor, undo groups and find, tokenizer_test.v for V, YAML and lazy highlighting), tests/editor_test.v (typing, clicks, drag, double and triple click, ctrl-click, alt-drag column, line keys, find bar and replace all, headless clipboard, IME, scrolling, closures flat), gallery Editor page, read-only manifest editor on the YAML tab of apps/cluster, bench/editor below |
| M10 live data | Done, with the deviations in specs/19 (Status) | quadra/kit/data (live_test.v keyed updates and flashes, log_test.v store and filters, value_test.v YAML subset and JSON), quadra/kit/viz (geometry_test.v), `term.style_spans` (screen_test.v), apps/cluster on a ticking fake cluster (live_test.v: the interval drives ticks, selection by key across reorders and replacements, closures flat over 30 ticks; flows_test.v: value view, manifest apply and rejection, logs follow and filter, metrics, events), six gallery pages (Live table, Log view, Value viewer, Sparkline, Line chart, Bar chart), bench/live below |
| Comparison benchmarks | Done (docs/benchmarks.md): faster and smaller than the best published numbers of three other frameworks in CPU per frame, peak and idle memory and size; startup is 199 ms against 157 ms (140 ms is the GPU driver) | bench/compare, bench/compare/run, tests/shaping_test.v |
| Gap analysis | Written (specs/20). P1 filled: file dialogs and shell, popover and hover card, drag and drop with docking, variable-height list, rich text. P2 filled (all): UI Automation, Markdown view, drawer, alert, empty state, description list, message box, tray and notifications, table pinned columns and cell selection, WARP-first start-up, standalone calendar, time input, range slider, transforms (translate, scale, rotate), transitions and animation timelines, color emoji. P3 remains | specs/20-gaps.md, tests/drag_test.v, tests/vlist_test.v, tests/rich_text_test.v, tests/a11y_test.v, tests/uia_windows_test.v, tests/table_pin_test.v, tests/shell_test.v, tests/upgrade_windows_test.v, tests/pickers_test.v, tests/motion_test.v, tests/timeline_test.v, tests/stroke_test.v, tests/emoji_windows_test.v |
| Folio (Markdown app, specs/21) | Done | apps/folio: folio_test.v (layouts, live edit, dirty and save, outline click, close prompt, outside changes), pixels_test.v (every layout, palette, both themes, math, diagrams in .tmp/out/folio_*.png); kit/md (md2_test.v, tex, mermaid), kit/code lex_md_test.v, tests/fonts_windows_test.v (in-memory fonts), tests/rich_text_test.v (inline boxes) |
| Design language "One" (specs/15) | Done: One Dark Pro and One Light are `kit.dark()` and `kit.light()`; IBM Plex Sans and Lilex are bundled in the kit (`kit.load_faces`, OFL texts in quadra/kit/fonts); controls, menus, tabs, overlays, chrome, tables, trees, the editor gutter and the Markdown view follow the spec. Since then: auto-hiding scroll thumbs on every scrolling container (one activity record per window, no per-container state), `calt` off for the UI face (`TextSystem.contextual_off`), accent match highlights in the palette and combobox, tooltip shortcuts and meta lines, invalid inputs (`input.set_invalid`), notifications at the bottom right (`kit.notify`), the hamburger app menu in the title bar, icon-only status bar toggles, the switch ring, the filled dropdown, toggled chips with a fill, and the Settings component (`kit.settings`). Then: draggable thumbs that also show when content moves by keys or the app, Settings groups expanding under their page and scrolling into view, notification actions and timeouts, Segoe Fluent caption glyphs on Windows 11, input text in the editor foreground, popups returning the focus to their trigger, hamburger menus in Notes and Cluster; One Dark Pro as the only dark theme with three new palette roles (`status_bg`, `tab_bg`, `scroll_thumb`); themes are JSON files (`kit.load_theme`, `kit.theme_from_json`, `Theme.to_json`; the built-ins are quadra/kit/themes/*.json, 57 us to parse). Cost of the bundled faces: 2.5 ms to load (views of the embedded bytes, one collection rebuild), gallery start-up 107 to about 109 ms, +4 MB working set, +1.6 MB binary | tests/fonts_windows_test.v (family names), tests/kit_test.v (`Ctrl-Shift-P` labels), tests/kit3_test.v (Settings, thumbs, notifications, match marks, theme files), pixel snapshots in .tmp/out (gallery, Folio, Notes, Cluster) |

## Benchmarks (bench/, MSVC -prod with the collector, fake text backend, 2026-11-07)

Re-measured after the memory fixes below and again after the gallery became one page per
component (49 pages, 49 sidebar rows), and again after the text caches became flat maps
with frame generations (quadra/text/cache.v). The
earlier M6 numbers came from builds without a
collector (V's MSVC default), so they did not include GC costs.

| Scenario | Time | Budget (spec 12) |
|---|---|---|
| Hover change, 1000-button grid | 0.016 ms (paint patch, partial GPU upload) | < 0.3 ms: met |
| Typing a character in the gallery Text input page | 0.57 ms (was 1.03 ms: text cache hits cost 0.6 µs, now 30 ns) | < 1 ms: met |
| Scrolling the gallery's 1,000,000-row table | 0.38 ms | < 1.5 ms: met |
| Window resize of the gallery, full relayout | 0.81 ms | < 6 ms: met |
| Full first render of the gallery (new app and window) | 2.9 ms | < 10 ms: met |
| Gallery draw calls per page | 1 | < 40: met |
| Gallery working set at startup (real D3D11 + DirectWrite) | 49–52 MB | < 80 MB: met |
| Memory over a long session (`bench/leak real 30`) | flat: 15.5 MB live, working set 69 → 70 MB over 25 rounds of 19 page switches, ~7000 hovers, scrolls and timers each | must not grow |
| Startup to first presented frame (real platform, from `main`) | 181–188 ms | < 150 ms: not met here. The cold D3D11 driver start is 150 ms on this machine (on a worker thread, overlapped). It was 159–165 ms before the collector was enabled |
| Idle | 0 frames, 0% CPU (M1) | met |
| Layout, flex, 10k nodes (builds the tree too) | 6.4 ms | n/a |
| Layout, grid, 10k cells | 6.5 ms | n/a |
| Text wrap, 100 KB, cold / warm caches | 7.4 / 0.015 ms | n/a |
| Full build, 1000-button grid (stress) | 9.3 ms | n/a |

### Memory (fixed 2026-11-07)

The first gallery release grew from 15 MB to 2.3 GB in use. Four causes, all fixed:
1. MSVC builds had no garbage collector (V's default for MSVC). `tools/v.sh` now always passes
   `-gc boehm_full_opt`, and `quadra.core` refuses to compile without a collector.
2. The stock Win64 closure thunk clobbered `xmm15`, which is callee-saved, and corrupted floats
   in optimized callers (random NaN geometry). The toolchain is patched (`tools/patches/v-closure.diff`).
3. V never frees closures. Quadra now owns their lifetimes (`quadra/core/tracked.v`): closures
   made while building kit components (every layout) or list rows (every prepaint) are freed
   after the next pass of the same kind. Each view's render closures are freed when it renders
   again, unless the render pinned them by creating something long-lived (see the rule in
   specs/14). Hot engine paths (notifications, timers, paint ticks, SVG paint) have no closures.
   Since 2026-11-08 the kit avoids them too (specs/14, avoiding closures): 117 → 23 in
   `quadra/`, capped by `tools/check.sh`; typing frees 4 closures a keystroke instead of 86.
4. Virtual lists and overlays added layout nodes on every prepaint, and the window's layout
   tree was only cleared by full builds. Prepaint now truncates the tree back to the build's
   nodes.

Check with `bench/leak`: `tools/v.sh -cc msvc -prod -o .tmp/leak.exe bench/leak && .tmp/leak.exe real 30 all`.
Scenarios are `all`, `switch`, `hover`, `scroll`, `type` and `table`. `tests/memory_test.v`
guards points 3 and 4. Run `tools/check.sh prod` before a release: it also runs the suite as
MSVC `-prod` builds, where points 1 and 2 show up.

### Text rendering (2026-11-07)

Text looked soft next to native apps on a 100% (96 DPI) monitor with ClearType on. Two fixes:
glyph coverage is now gamma and contrast corrected in the shader (light text on dark planes
was thin), and glyphs render with ClearType when Windows has it on, blended per subpixel with
dual-source blending (specs/04). `tests/cleartype_test.v` checks both paths on real pixels.
The gallery's long-session working set is 74 MB with 49 pages and the RGBA glyph atlas
(`bench/leak real 8 all`, flat); startup is unchanged (181–193 ms, 49–54 MB).
Re-measured after M8 (54 pages): working set 76.7 MB and live heap 24–26 MB, both flat, and
the live closure count is flat (632) since the closures made in paint, layout and prepaint
(stroke cache misses, split handles, tooltip delays) were replaced; it grew by about 30 per
round before.

### Terminal (bench/term, MSVC -prod, 2026-11-07)
| Benchmark | Result | Budget (specs/16) |
|---|---|---|
| Parse plain text | 50.4 MB/s | 50 MB/s |
| Parse SGR-heavy output | 52.7 MB/s | |
| 100 000 lines of scrollback | 17.8 MB | 40 MB |
| Repaint a full 230×64 screen | 0.55 ms | 2 ms |
| Resize (reflow) with 10 000 history lines | 3.6 ms | |
| Closures while 3 000 chunks stream in | flat (5) | |

Parsing reached the budget once scrolled-off rows were reused instead of allocated and
scrollback packing wrote ASCII bytes directly; the margin is thin, so a regression shows
here first. Resize went from 200–300 ms to 3.6 ms once the history pass stopped holding
its array across calls (specs/14, GC pins around calls).

### Editor (bench/editor, MSVC -prod, 2026-11-07)
| Benchmark | Result | Budget (specs/16) |
|---|---|---|
| Open 9.5 MB of V source and paint the first frame | 12.4 ms | 200 ms |
| Keystroke + repaint at line 50 000 of 100 000 | 0.38 ms | 2 ms |
| Keystroke + repaint at line 2 (every highlighter state below is dropped) | 0.46 ms | 2 ms |
| Scroll frame (one wheel step) | 0.31 ms | |
| Closures over 500 keystrokes with the caret blinking | flat (2) | |

Opening is cheap because the piece table keeps the file as one piece and only the line-start
index is built; highlighting is lazy, so only the visible lines are tokenized.

### Live data (bench/live, MSVC -prod, 2026-11-07)
| Benchmark | Result | Budget (specs/16) |
|---|---|---|
| Hold 1 000 000 log lines (~60 bytes each) | 76 MB, appended in 114 ms | 1 000 000 lines |
| Frame with 167 new lines (10 000 lines/s at 60 Hz) | 0.60 ms | 4 ms |
| Same, with a regex filter active | 0.61 ms | 4 ms |
| Text filter over 1 020 040 lines | 293 ms (once per filter change) | |
| Regex filter over 1 020 040 lines | 1.6 s (once per filter change; the backtracking `regex` module) | |
| Keyed update of a 10 000-row table, every cell compared, plus a frame | 2.64 ms | |
| Closures over 200 log appends and table updates | flat (27) | |

Appending was 16.6 s for the million lines until the batch loop stopped holding the batch
array in scope while it called `push` (specs/14, GC pins around calls).

Measure startup and memory with `tools/v.sh -cc msvc -prod -o .tmp/g.exe apps/gallery && .tmp/g.exe --measure`. GPU time per frame is not measured yet (no GPU timer queries).

## Manual checks (need a person at the machine)
1. Session 0: run `spikes/m0/session0_check.sh` from an elevated Git Bash. The headless and pixel tests must pass as SYSTEM, without a login session.
2. IME: type Vietnamese with Telex or VNI in `.tmp/window_demo.exe` and in a Notes note.
3. DPI: open the gallery on a 150% monitor and move it between monitors with different scales.
4. Custom titlebar (gallery and Notes):
   - Drag the window by the bar; double-click the bar to maximize.
   - Hover the maximize button for snap layouts (Windows 11).
   - Use minimize, maximize/restore and close (Notes asks to save unsaved notes).
   - Resize from every edge and both top corners.
5. Real terminal: `tools/v.sh -o .tmp/cluster.exe apps/cluster && .tmp/cluster.exe --local`, select a pod, open Shell. It is `cmd.exe` on ConPTY: type `dir`, run a full-screen program if you have one (e.g. `vim`), resize the window, select and copy with ctrl-shift-c. Close the app: no `cmd.exe` or `conhost.exe` from it may remain in Task Manager.
6. Editor: open the gallery Editor page. Type Vietnamese with Telex in the text and in the find query (ctrl-f; the preedit is drawn underlined at the cursor and the IME window sits next to it), ctrl-d through a few matches and type, alt-drag a column, ctrl-f then ctrl-h and Replace all, then ctrl-z. Check the caret blinks and the colors read well in both themes.
7. Live cluster: `tools/v.sh -o .tmp/cluster.exe apps/cluster && .tmp/cluster.exe`. Watch the table for a minute: changed cells flash once and fade, the selection stays on its pod. Open Metrics and hover the chart (crosshair and values follow the mouse; lines look smooth at 100% and 150%). Open Logs: it follows; scroll up and it stops; Follow resumes. Events lists new events at the top.
8. File dialogs: in an app calling `cx.pick_files`, open, save (with a filter) and pick a folder; cancel returns nothing. `cx.reveal(path)` selects the file in Explorer.
9. Dock: on the gallery Dock page, tear a tab off (drag it down), hover each pane edge and the middle (the zone highlights), drop it; drop one on the other strip; escape mid-drag cancels.
10. Screen reader: start Narrator (ctrl-win-enter) on the gallery. Tab through a page: Narrator reads each control's name and role; on a checkbox, caps-space toggles it; on a button, caps-enter presses it.
11. Tray and notifications: in an app calling `cx.set_tray('...')`, the icon shows in the notification area with its tooltip; click, double-click and right-click reach `on_tray`; `cx.show_notification` shows a Windows notification; the icon goes away when the app quits. `cx.message_box` shows a native box with the right icon and buttons.
12. Start-up switch: start the gallery several times and watch the first quarter second. The window draws at once on WARP, then moves to the hardware GPU (about 150 ms in): no blank or black frame may show at the switch. Resize right after start: no error in the console.

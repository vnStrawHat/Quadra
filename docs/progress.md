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
| M8 terminal | Done (specs/17). Of the four budgets, typing echo under one frame is not measured (it needs a timer across the real PTY round trip) | quadra/kit/term (parser, screen, reflow and key tests), tests/term_test.v (echo, history, selection, copy, bracketed paste, exit, resize on a FakePty), tests/conpty_test.v (a real program on ConPTY), gallery Terminal page, Shell tab in apps/cluster (`--local` opens the system shell), bench/term below |
| M9 code editor | Done, with the deviations in specs/18 (Status) | quadra/kit/code (buffer property test, doc_test.v for movement, every command, multi-cursor, undo groups and find, tokenizer_test.v for V, YAML and lazy highlighting), tests/editor_test.v (typing, clicks, drag, double and triple click, ctrl-click, alt-drag column, line keys, find bar and replace all, headless clipboard, IME, scrolling, closures flat), gallery Editor page, read-only manifest editor on the YAML tab of apps/cluster, bench/editor below |
| M10 live data | Done, with the deviations in specs/19 (Status) | quadra/kit/data (live_test.v keyed updates and flashes, log_test.v store and filters, value_test.v YAML subset and JSON), quadra/kit/viz (geometry_test.v), `term.style_spans` (screen_test.v), apps/cluster on a ticking fake cluster (live_test.v: the interval drives ticks, selection by key across reorders and replacements, closures flat over 30 ticks; flows_test.v: value view, manifest apply and rejection, logs follow and filter, metrics, events), six gallery pages (Live table, Log view, Value viewer, Sparkline, Line chart, Bar chart), bench/live below |
| Comparison benchmarks | Done (docs/benchmarks.md): faster and smaller than the best published numbers of three other frameworks in CPU per frame, peak and idle memory and size; startup is 199 ms against 157 ms (140 ms is the GPU driver) | bench/compare, bench/compare/run, tests/shaping_test.v |
| Gap analysis | Written (specs/20). P1 filled: file dialogs and shell, popover and hover card, drag and drop with docking, variable-height list, rich text. P2 filled (all): UI Automation, Markdown view, drawer, alert, empty state, description list, message box, tray and notifications, table pinned columns and cell selection, WARP-first start-up, standalone calendar, time input, range slider, transforms (translate, scale, rotate), transitions and animation timelines, color emoji. P3 remains | specs/20-gaps.md, tests/drag_test.v, tests/vlist_test.v, tests/rich_text_test.v, tests/a11y_test.v, tests/uia_windows_test.v, tests/table_pin_test.v, tests/shell_test.v, tests/upgrade_windows_test.v, tests/pickers_test.v, tests/motion_test.v, tests/timeline_test.v, tests/stroke_test.v, tests/emoji_windows_test.v |
| Folio (Markdown app, specs/21) | Done; the outline starts hidden | apps/folio: folio_test.v (layouts, live edit, dirty and save, outline click, close prompt, outside changes), pixels_test.v (every layout, palette, both themes, math, diagrams in .tmp/out/folio_*.png); kit/md (md2_test.v, tex, mermaid), kit/code lex_md_test.v, tests/fonts_windows_test.v (in-memory fonts), tests/rich_text_test.v (inline boxes) |
| Design language "One" (specs/15) | Done: One Dark Pro and One Light are `kit.dark()` and `kit.light()`; IBM Plex Sans and Lilex are bundled in the kit (`kit.load_faces`, OFL texts in quadra/kit/fonts); controls, menus, tabs, overlays, chrome, tables, trees, the editor gutter and the Markdown view follow the spec. Since then: auto-hiding scroll thumbs on every scrolling container (one activity record per window, no per-container state), `calt` off for the UI face (`TextSystem.contextual_off`), accent match highlights in the palette and combobox, tooltip shortcuts and meta lines, invalid inputs (`input.set_invalid`), notifications at the bottom right (`kit.notify`), the hamburger app menu in the title bar, icon-only status bar toggles, the switch ring, the filled dropdown, toggled chips with a fill, and the Settings component (`kit.settings`). Then: draggable thumbs that also show when content moves by keys or the app, Settings groups expanding under their page and scrolling into view, notification actions and timeouts, Segoe Fluent caption glyphs on Windows 11, input text in the editor foreground, popups returning the focus to their trigger, hamburger menus in Notes and Cluster; One Dark Pro as the only dark theme with three new palette roles (`status_bg`, `tab_bg`, `scroll_thumb`); themes are JSON files (`kit.load_theme`, `kit.theme_from_json`, `Theme.to_json`; the built-ins are quadra/kit/themes/*.json, 57 us to parse). Cost of the bundled faces: 2.5 ms to load (views of the embedded bytes, one collection rebuild), gallery start-up 107 to about 109 ms, +4 MB working set, +1.6 MB binary | tests/fonts_windows_test.v (family names), tests/kit_test.v (`Ctrl-Shift-P` labels), tests/kit3_test.v (Settings, thumbs, notifications, match marks, theme files), pixel snapshots in .tmp/out (gallery, Folio, Notes, Cluster) |
| OneTerm FPS HUD GPU rows (specs/03, specs/22) | `gpu.Device.adapter_name()` (DXGI `GetAdapter` + `GetDesc`, kept with the device, follows the WARP-to-hardware switch; `Null device` headless), `core.App.adapter_name()`, a `gpu` line on the devtools profiler; per-process GPU usage from PDH `GPU Engine` (busiest engine of this pid, clamped, every other tick, off the UI thread) in `quadra/native/sys` | quadra/native/sys/gpu_test.v, apps/oneterm/fps_test.v, workspace_ui_test.v, tests/pixels_m2_test.v |
| OneTerm OS code | Moved behind `quadra/native/cng` and `quadra/native/sys`; apps/oneterm has no OS files, backend imports (tests aside) or C calls | `tools/check.sh` portability sections, quadra/native/sys/crash_windows_test.v |
| OneTerm terminal gaps (specs/17, specs/22) | URL policy and confirmation dialog, URL detection across wraps with `ftp://`/`www.`/Unicode, selection anchored to rows (eviction, regions, erase rules), logical-line and cross-wrap word selection, per-terminal find bar in the original's order, cell-exact box/block/braille/powerline shapes with device-pixel cells and grid-placed glyphs, `cursor.color`, `layout.cell_width`, `colors.min_contrast`, `font.features` on DirectWrite, shared font zoom, weight names | quadra/kit/term/url_policy_test.v, url_detect_test.v, select_anchor_test.v, shapes_test.v, behaviors_test.v, apps/oneterm/wire_term_test.v, tests/shaping_test.v |
| OneTerm config, settings and chrome gaps (specs/22) | An unreadable config is never written over, per document; first-run files; one bad `lock_after_idle_minutes` or `vertical_tabs` value costs only itself; login Bash and Zsh with the Zsh prompt marks and PATH lookup; the seven-kind Shell dropdown; searchable Font Family (`TextSystem.font_families`), Log Folder picker, labels, ranges and wording of the original; default-key collision rule with "Still unbound"; About identity; Rename dialog; lock refusals; `...` list heading and Close. The Settings window in the original's look: `kit.settings(...).cards()` (icon navigation with search, page header with the Reset All undo icon, outlined group cards), stepper number inputs, compact selects, `Button.caret()`, the `input_bg` palette role, `InputOptions.height`/`center`, `Titlebar.title_size`; pixel-compared against the original at 1000x700 | apps/oneterm/config_guard_test.v, settings_ui_test.v, chrome_items_test.v, sftpui/panel_test.v |
| OneTerm look, kit side (specs/15) | Theme-driven: control heights `h_xs`..`h_lg`, `ui_scale` (rem) with `Theme.sp`, `text_step` with `Theme.ui_text` (kit text one step down a 12-20 scale); palette roles `selection`, `tab_fg`, `tab_active_fg`, `table_head_bg`/`_fg`, `table_row_border`, `list_even`, `scroll_thumb_hover`, `text_on_status`, `error_hover`, `cyan`; `Controls` families `solid_buttons`, `roomy_inputs`, `roomy_menus` (menus, tooltips at the pointer), `plus_keys` (`kit.key_text`), `card_dialogs` (`ModalOptions.alert`, `description`), `tinted_notices` (one stack, 5 s auto-hide), `accent_chrome` (title bar shading, drawn caption buttons, accent-line tabs, status dividers), `fixed_scrollbars` (`core.ThumbStyle`, a terminal thumb); select lists (`Menu.list`). OneTerm sets them in `themed()`; the other apps render pixel-identical. Framework pieces it needed: `Div.ring(w, c)` (an outer focus band; focused `roomy_inputs` fields), hover groups `Div.group()` / `group_hover` / `group_active` (a notice's close button shows only while it is hovered), `kit.resize_line` (split and dock handles: a `line` hairline with a `text_2` pill on hover and drag), `Button.compact()` (the title-bar menu); absolute children with an inset of exactly -1 px were placed as if unset (the `unknown` sentinel), now fixed | tests/kit3_test.v, tests/patch_test.v (group hover), quadra/layout/absolute_test.v, bench/shots before/after diff |
| OneTerm look, app side (specs/22) | Themes reconverted with the original's own keys and fallbacks: `line_strong` from `border`, `scrim` from `overlay`, `accent_text` from `link`, `panel` from `muted.background`, `info` falling back to cyan, `tab_bg`/`overlay_bg` to the background, plus the kit's derived roles (`selection` capped at 30 %, `tab_fg`, `table_head_*`, `table_row_border`, `list_even`, `scroll_thumb_hover`, `text_on_status`, `error_hover`, `cyan`). Views in the original's sizes, through theme accessors: title bar group, strip and dock buttons (the dock icon follows the state), rail header, cwd, handle fill and contrast-chosen collapsed glyph, status bar spacing, icons and toggles, gutter inset, progress track, dock section headers ("Session"), SFTP cells, header dividers and transfers, session rows and search box, dialog footers, alert, Rename, About links, Advanced, crash box, lock screen, find bar, completion kind words; FPS shows `-` before its first second. ConPTY close terminates a shell still running 2 s after its pseudo console closed; `tools/conpty.sh` keeps a destination in use | apps/oneterm/*_test.v, fps_test.v, quadra/platform/win/conpty_windows_test.v (fails on the old close), shots in .tmp/oneterm-look |
| OneTerm P1 (specs/22) | O1-O4 done in code: local shells on ConPTY, tabs, Spaces, the terminal behaviours, search, settings window, themes, key overrides. The engine replays all 45 recordings of the conformance corpus. A manual pass on a real ConPTY is pending (manual check 13). | apps/oneterm (oneterm_test.v on a FakePty, conpty_test.v on a real `cmd`), quadra/kit/term/behaviors_test.v, tests/term_corpus_test.v |
| OneTerm P2-P3 (specs/22-27) | The 1:1 port of the original (v0.6.11): SSH2 client and SFTP in V with Windows CNG, sessions and SFTP panels, vault, session store, updater, crash reports, terminal logs, completion, highlighting, 39 themes, 8 settings pages, dock, vertical tabs, lock screen, elevation. Live run against a local sshd passed (`ssh/interop*`, end-to-end connect, SFTP listing, duplicate, exit). The bundled ConPTY host (specs/17): `conpty.dll` beside the exe is preferred, kernel32 the fallback, `tools/conpty.sh` stages the pair with its digests checked (package.sh does it for OneTerm), the updater ends stale hosts from the install directory. Measured at 160x45 with `bench/fire`: inside OneTerm (terminal log on, to read the result) 279 fps / 37.0 MiB/s bundled against 218 fps / 28.8 MiB/s system; a bare ConPTY reader 383-400 fps / 50-53 MiB/s bundled against 253-267 fps / 33-35 MiB/s system (read and discard), 279-307 against 244-245 fps when the reader parses; a Sixel DCS reaches a local shell's reader only through the bundled host. Known gaps: `mlkem768x25519-sha256` and `zlib` not offered, `$SSH_AUTH_SOCK` not read, SFTP edit saves polled, the updater's real install and live GitHub not exercised. Windows Defender removes a varying few `-prod` test binaries on the build machine as `Trojan:Win64/Tedy.PAIB!MTB`, which fails `check.sh prod`: the detection differs per build, each flagged test passed in at least one direct run, and the release `oneterm.exe` ran with no detection | `tools/check.sh` (139 of 139), apps/oneterm/*_test.v, sub-module tests |
| Element memory (specs/07, specs/12, specs/14) | Render arenas: `ui.div()` and `ui.text()` in a view's render reuse the elements of its render before last (reset at the end of the build that replaced them, dropped when the render is pinned; a drag preview pins); `ui.number`; arena text elements lay numbers out into their own wrap (`TextSystem.wrap_number`); flex scratch per depth; allocation-free UTF-8 decoding and prepaint ids. Kit builds during layout use a window pass arena and nested renders the view's arena; svgs and div interaction states are reused; `kit.menu_button_with` builds menu items on open; OneTerm idle 236 → 125 KB/s. Grid scene 1902 → 42 KB a frame, 2.29 → 1.33 ms CPU, 60 → 29 MB peak (docs/benchmarks.md, Element memory). Gallery shots pixel-identical (82 dark, 82 light); `bench/leak` flat | tests/arena_test.v (reset, clean views kept, elements made outside a render or kept by a pinned render, number formatting, lazy menus), tests/arena_build_test.v (pass arenas, nested renders, keep_build pins nested views), quadra/text/fake/fake_test.v (wrap_number = wrap), quadra/text/text_test.v (decode) |
| Closure leaks (specs/14, No closures in apps) | Apps and examples make no capturing closure (`fn [` fails check.sh, cap 0); quadra keeps 3 made-once ones (cap 22 → 3). Closure-free APIs: `_with` variants for timers, debounce, observe, subscribe, updates, settings, tasks, modals (`body_with`, `footer_with`, `kit.build_with`, `kit.view_body`), hover cards, lists, terminals (`term.start_with`, `PtyOptions.ctx`), palette commands (`Command.on_run`), table cells (`data.Cells`), `App.post_main`; inbox, frame work functions, UIA actions, image loads, kit rules and the table sort without closures. Debug-build dispatch detector (`App.closure_leaks`, `QUADRA_TRACE_CLOSURES`). Leaks fixed: a closure per task, timer, debounce, update, palette opening, dialog, terminal (with its inbox), SSH connect, transfer, sort and image layout, and per settings save and theme change (`follow_system` added a hook each time). OneTerm idle: closures created 60/min → 0, allocation 235 → 46 KB/s, a root render 156 → 28 KB | apps/*/ gate tests (`test_flows_leak_no_closures`, gallery `test_overlays_leak_no_closures`, sftpui/leak_test.v, apps/oneterm/leak_gate_test.v), tests/with_test.v |
| Vault unlock memory (docs/v-notes.md) | Argon2id's 64 MiB lives in the C heap and is wiped and freed when the key is out (`tools/patches/v-argon2-release.diff`): a stale word kept it on the collector's heap, and tcc builds never return heap memory. Private WS after an unlock: 140 → 12 MB on real backends | apps/oneterm/vault_memory_test.v |

## Release toolchain (2026-11-09)

Release builds moved from MSVC to the portable clang 23.1.5 (llvm-mingw) that `tools/setup.sh` installs in `.tools/llvm-mingw`: no Visual Studio, and packaged apps no longer need `vcruntime140.dll`. Against MSVC -prod, the median of the 11 benchmarks below takes 0.75 times as long (0.65 to 1.06; only the two text-wrap rows are slower), gallery start-up is 115 ms against 122 ms and the working set about the same. The tables below are still the MSVC numbers.

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
   Since 2026-11-10 apps make none at all and `quadra/` keeps 3 made-once ones (specs/14, No
   closures in apps); a debug-build detector names any dispatch that makes one.
4. Virtual lists and overlays added layout nodes on every prepaint, and the window's layout
   tree was only cleared by full builds. Prepaint now truncates the tree back to the build's
   nodes.

Check with `bench/leak`: `tools/v.sh -cc clang -prod -o .tmp/leak.exe bench/leak && .tmp/leak.exe real 30 all`.
Scenarios are `all`, `switch`, `hover`, `scroll`, `type` and `table`. `tests/memory_test.v`
guards points 3 and 4. Run `tools/check.sh prod` before a release: it also runs the suite as
clang `-prod` builds, where point 2 shows up (point 1 was specific to MSVC).

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

### OneTerm throughput (bench/fire/run.sh, 2026-11-10)
Fire at 160x45 inside OneTerm (terminal 176x46, terminal logging off, 75 Hz display), median of 3
interleaved 15 s runs. The original, same machine and command: 409.6 fps at 159x45.

| Build | Bundled host | System host |
|---|---|---|
| Before, clang -prod | 363.1 fps (360.6 rebuilt) | 253.4 fps |
| Before, MSVC -prod | 335.2 fps | 250.4 fps |
| After, MSVC -prod | 403.4 fps (53.7 MiB/s) | 255.3 fps |
| After the delivery and highlight fixes, before the parser ones, clang -prod | 404.7 fps | 256.9 fps |

The final clang `-prod` OneTerm could not be measured: Windows Defender quarantined every such
build of it (`Trojan:Win64/Tedy.PAIB!MTB`, the false positive noted above), while it let the MSVC
build, the reconstructed baseline and the benchmark binaries through. The ceiling on this machine
is the host: a reader that discards the bytes gets 428 fps at 176x46 (bundled) and 277 fps
(system); the system host also drops most frames (8 MiB/s reach the reader), so its number is
the host's. With terminal logging on, 386.6 fps (MSVC). `--stats` shows 75 fps: that is the
display, which takes one fire frame in five; fire's rate is PTY throughput.

What it took, from the profile (`--stats` counters, a capture of the host's output parsed alone,
the collector's count and allocated bytes per second):
- Collector pauses stop the reader thread too: 13-15 collections a second came from a clone of
  every 64 KiB read and a fresh inbox buffer per drain. Reads are now lent to `on_data` and the
  inbox swaps two buffers (specs/17, Delivery): about 1 a second; the output pipe has 1 MiB, as
  in the original. With parsing switched off, OneTerm went from 372 to 407-416 fps.
- Semantic highlighting rescanned every changed line with fresh buffers and a string per line
  (22 MiB/s allocated under fire, 1.1 ms of the grid's 1.7 ms paint): buffers reused, runes
  scanned directly, URL coloring skips rows without `:`, `.` or OSC 8 (specs/22).
- Parse of the captured stream: 161 to 419 MiB/s. Two binary searches of the width tables per
  half block were most of the print cost (one-entry width cache); whole CSI and UTF-8
  sequences take a fast path in the ground state, SGR ahead of the CSI dispatch, and the
  extended colors are the first SGR arm (V tests match arms in order). In OneTerm (MSVC) parse
  went from 1.7-2.1 to 0.8 ms per 140 KB chunk and the grid paint from 4.0 to 2.9 ms.

bench/term, clang -prod, before and after: parse plain text 72-75 / 70-72 MB/s, SGR-heavy 83-84 /
93-101 MB/s, fire half blocks 141 / 361-367 MB/s, a fire frame through the view 4.3-4.7 / 3.0-3.3 ms
(parse 1.9-2.1 / 0.75-0.79 ms), repaint, scrollback and resize unchanged. Idle OneTerm (cmd
prompt, caret blinking, clock): 9-17 ms/s CPU in both builds, about one 15.6 ms tick a second;
working set with 12 000 lines of scrollback 114 / 109 MB.

### OneTerm memory (bench/mem/run.sh, 2026-11-10)
Medians of 3, MSVC `-prod`, private working set / commit in MB, before and after: idle 21.5 / 115.6
to 20.8 / 67.6; five tabs and the dock 24.2 / 182.7 to 22.4 / 68.8; 10 s after fire 41.3 / 112.1 to
23.3 / 73.0; a full 10 000-line history 43.2 / 107.7 to 27.3 / 75.2. Thread stacks reserved
instead of committed, the collector built to return memory and a trim after bursts
(`core/trim.v`), a 1024 first atlas page, burst buffers released by the inbox, history runs
sized exactly (specs/12, Memory). Fire unchanged (399.8 / 401.5 fps, medians of 3 interleaved). Breakdown and
procedure: docs/benchmarks.md, OneTerm memory. Open: working-set trimming once minimized, and
the private working set left is mostly the GPU driver and the system heaps.

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

Measure startup and memory with `tools/v.sh -cc clang -prod -o .tmp/g.exe apps/gallery && .tmp/g.exe --measure`. GPU time per frame is not measured yet (no GPU timer queries).

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
13. OneTerm: `tools/v.sh -cc clang -prod -o .tmp/oneterm.exe apps/oneterm && .tmp/oneterm.exe --data .tmp/oneterm-data`. Type in cmd and PowerShell; split, resize and close Spaces; run `vim` or another full-screen program and use the wheel; compose Vietnamese with Telex (the preedit shows at the cursor); ctrl-f; ctrl-click an https link; change the theme and font size in Settings (ctrl-,).

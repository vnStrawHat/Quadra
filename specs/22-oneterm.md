# 22. OneTerm

OneTerm is a terminal application for local shells and SSH/SFTP. This spec rewrites it in V
on Quadra, in `apps/oneterm`. The Rust original (v0.6.11) is the behavioural reference; where
this spec is silent, the original decides. The workbench follows the One design language
(specs/15). The terminal engine is `kit/term` (specs/17), upgraded so every Quadra app gains;
its limits are the original's (specs/17, Limits).

## Phases

| Phase | Contents | State |
|---|---|---|
| **P1 Local terminal** | local shells on ConPTY, tabs, Spaces, terminal behaviours, settings, themes | done |
| **P2 Remote** | SSH client, session tree, SFTP browser, right dock, vault. The workspace shell (this spec) owns the dock frame and the wiring (Remote, below); `ssh`, `vault`, `store`, `sessionui`, `sftpui` are written beside it | done |
| **P3 Polish** | OSC 133 marks, completion, semantic highlight, OSC 9 toasts and 9;4 progress, OSC 7501 status, Sixel, broadcast channels, vertical tab rail, git/net/CPU in the status bar, terminal logging, auto-update, the elevated window, crash reports | in scope |

P1 milestones, each green on `tools/check.sh` before the next:

| Id | Contents |
|---|---|
| O1 | window, config files, one shell tab, PTY environment and shell integration, status bar with clock and cwd |
| O2 | tabs and Spaces with their menus and key bindings |
| O3 | terminal behaviours (below) and the engine corpus |
| O4 | settings window, themes, key binding overrides |

The shell milestones (S1 menus, S2 status bar, S3 dock, S4 rail, S5 channels, S6 HUD and lock, S7 settings pages, S8 themes, S9 elevation) follow the sections below.

## Window

1600×1000, capped at 85 % of the display and centred, minimum 640×480. Title bar, then the dock area (the centre: tab strip and Spaces, or the rail and Spaces; beside it the right dock, from the strip down), then the status bar.

- **Title bar**: terminal glyph in `#58C4DC`, the app menu as a button named "OneTerm", then the elevation suffix. App menu: About, separator, Appearance ▸ (Light, Dark, checked by the theme's mode), Theme ▸ (every theme, checked), Show FPS Monitor (checked), separator, Settings..., Lock OneTerm, separator, Quit. Appearance swaps to the last theme used in that mode (default One Light / One Dark, in memory).
- **Tab strip** trailing actions, in order: `+` dropdown, `...` tab list (the heading "Terminal Tabs", one row per tab with the active one checked, separator, Close), right-dock toggle. The `+` button's tooltip is "New Terminal"; a menu taller than the cap scrolls. Tabs 100–220 px wide, label and close button.
- **`+` dropdown**: local shells; `Run as administrator ›` submenu with the same three shells (absent in an elevated window); labelled separator "SSH Sessions"; ungrouped sessions, then per group a labelled separator and its sessions (8 px color square and title), or a disabled "No saved sessions"; separator; Quick Connect..., New Saved Session.... An elevated window shows the three shells only. Sessions come from `saved_sessions()`, read from `store` each time the menu is built (`ssh_session.json`; one `store.SessionStore` per process).

### Status bar

Left: the Collapse/Expand Tabs button and a divider (only with Vertical Tabs on), then the clock `YYYY-MM-DD HH:MM:SS`. Centre (grows, never pushes the ends): breadcrumb, git status. Right: network, CPU/MEM, the `--stats` text when on, right-dock toggle. A divider sits only between two drawn items. Every item has a leading icon that goes with its label.

| Item | Rule |
|---|---|
| Breadcrumb | the active Space's OSC 7 cwd. Click copies the sampled value (never the shortened one). Elides from the left at a separator; only a single component longer than the budget is cut inside. A shortened label has its full value as tooltip, an unshortened one "Click to copy" |
| Git | `main`, `main* (+100 -41)` (success/danger colours), `main ↑2 ↓1`, `a1b2c3d` (detached). Hidden outside a repository, when `git` is missing, or before OSC 7. Off the UI thread, one run at a time, at most every 2 s per cwd; a late answer for a left cwd is dropped. Runs `git --no-optional-locks` in the cwd as working folder |
| Network | `↓ 12.3 Kbps  ↑ 0 bps`, from `net_counters(space)` deltas each second; hidden when it returns none (every local tab) |
| CPU/MEM | `CPU 1.2%  MEM 45.2 MB`, process only, sampled every 2 s off the UI thread. MEM is the private working set, else the working set. Hover: **Memory** (Private working set, Working set, Commit (private bytes), Peak working set) and **CPU** (Usage `1.2% of 16 logical cores`, CPU time (user + kernel), Threads, Uptime) |
| Narrow window | the centre shrinks first. Width left = window − fixed items. The branch asks first; the path keeps at least 80 px; the branch keeps at least 40 px, eliding from its end |

`--stats` still prints the active terminal's last second, right of CPU/MEM, and as a `stats:` line on stderr (bench/fire/run.sh reads it). `--mem` prints a `mem:` line of memory counters on stderr once a second and `--tabs <n>` opens n tabs at start (bench/mem/run.sh, docs/benchmarks.md).

### Right dock

Follows the kind of the active tab's active Space (an empty Space counts as local): it opens when an SSH one becomes active, closes when a local one does, and only when the kind changes. The status bar button, the strip button (not the rail) and `toggle_right_dock` (`ctrl-shift-b`) toggle it and store the choice per kind in `ui_config.json` `right_dock` (`local` false, `ssh` true).

Width = `clamp_right_dock_width(preference, window)`: at most 35 % of the window, 240 px floor. The drag handle sets the preference, capped where it happens; a narrower window narrows the dock, a wider one brings the preference back. Content: a vertical split (default 50/50) of two sections, `session_panel` under the header "Session" (with a `+`) and `sftp_panel` under "SFTP Browser" (a `sessionui` and a `sftpui` view, specs/26 and 27). A header is `panel_header_h` tall on `tab_bg` with a bottom line, the title small in `text`, its buttons (xsmall) in a full-height group behind a left line, 8 px padding, 4 px gap. The dock and its panels sit on `canvas`. The Session header's `+` opens New SSH Session, the SFTP header carries the browser's expand toggle; expanded, the Session section is hidden and the browser (Local + Remote) fills the whole workspace between the title bar and the status bar, over the terminal area and the dock, with no drag handle; collapsing brings back the terminals and the dock at the width the user dragged. **Deliberate departure:** the original (since its US-0203) keeps the expanded browser inside the dock; the user decided on 2026-11-10 that this is a mistake and asked for the workspace-wide expand. Closing never drops panel state.

`docks.json`: `{"schema_version": 1, "version": 3, "right_dock": {"open": bool, "size": preference, "split": ratio}}`; other keys (`sftp_table_state`, `center`) are preserved on write; an invalid file is moved aside at the next write. At start the open state comes from the active tab's kind, not the file. An elevated window never reads or writes it.

### Vertical tabs

`ui_config.json` `vertical_tabs` `{enabled false, width 220, collapsed false}`; `width` is clamped to 140–400 on read. Rail width = `clamp_tab_rail_width(preference, group)`: min 140, max `max(140, min(400, 35 % of the centre))`; 44 px when collapsed.

Rail: 32 px header (14 px in, 8 px from the right: "Terminals" xsmall in `text_2`, a small `+` right; a 1 px line under it once the rows scroll), then rows that scroll with the kit's auto-hiding thumb. Row 48 px, no divider, 14 left / 10 right padding: line 1 chips, recording dot, title; line 2 the cwd (cut at its start), status icon at its end; with no cwd one centred line. Selected: `text` at 10 % fill, a 3 px accent pill inset 8 px top and bottom. Hover: `text` at 5 %. No close button. Click selects (on release, so a press can start a drag out of the row), middle closes, double renames, right opens the tab menu ("Close Tabs Below" instead of "Close to the Right"). The whole row drags: on a row it takes that slot, below the last it appends, on an empty Space it moves in. A 4 px handle on the right edge resizes, filled with `border_focused` on hover and while dragged; the preference is written once on release when it changed.

Collapsed: 44 px cells, the shell glyph centred (`cmd`, `powershell`, else terminal), OSC 7501 state as the cell fill (working `accent`, blocked `warn`, done `ok`, error `error`), no chips; on a fill the glyph (and the selected cell's pill, and the recording mark) is whichever of `canvas` and `text` contrasts more; hovering shows a card with the expanded row's lines. The collapse button lives in the status bar. Toggling Vertical Tabs keeps every session; the terminals only resize.

### Broadcast input channels

A Space joins A..E from its tab's menu: "Input Channel" between Duplicate and Close. Rows: `Channel A`..`E` (current prefixed `* `), then `Leave Channel` and `Close Channel <X>` for a member, then `Join All Spaces In Tab To <X>` and `Leave With All Spaces In Tab` with more than one Space. Rows act on the tab's active Space; disabled while it holds no terminal or has exited. Bound ids `join_input_channel_a..e`, `leave_input_channel`, `close_input_channel` ship unbound.

Input typed into a member is written to every other live member too: keys, text, paste, ctrl-c; not replies to the program and not mouse reports. Membership is in memory, per Space, moves with a dragged terminal; a Split or Duplicate starts outside. A Space whose session ends leaves by itself. `channel_color` is `series[0..4]`. Chips: 16 px square, letter in `background` on the channel colour; on the tab in A..E order (distinct), and as a badge in the Space's top-right corner, 5 px in. A selected member's ring takes the channel colour.

### Program status, recording, FPS HUD

`program_status(space)` (OSC 7501) draws after the chips on a single-Space tab: working a spinner in accent (one turn per second), blocked `bell` warn (opacity pulse 1 to 0.35 and back over 1.2 s), done `circle-check` ok, error `circle-x` error. A split tab shows none. Tooltip `Working`, `Waiting for you`, `Done`, `Error`. `is_recording(space)` draws the recording dot (and, on a split tab, a red `●` at the Space's top right).

FPS HUD (`show_fps`, app menu, General page, `toggle_fps` unbound): 12 px from the right, 12 px below the strip; rows FPS, GPU, API, GPU usage in mono 12. FPS shows `-` until the HUD has counted one whole second. Frames counted from the window's draw counter once a second. GPU is `app.adapter_name()`, with ` (software)` added on a software device and `n/a` when empty; it is read every tick, so it changes when the hardware device replaces WARP. API `Direct3D 11`. GPU usage: while the HUD is on, a `core.spawn_task` opens a PDH query on `\GPU Engine(*)\Utilization Percentage` (English counter name, `pdh.dll` loaded at run time; the first open costs 100-165 ms) and, every other 1 s tick, another task collects it. The reading is this process's busiest single engine (instances named `pid_<pid>_...`, any GPU, any engine type), clamped to 0-100 %, the definition of Task Manager's GPU column; shown `1.8%` (one decimal), `-` until the first sample lands, `n/a` when the query cannot open or off Windows (`quadra/native/sys`: `open_gpu_usage`, `GpuUsage.sample`, `busiest_engine`). One sample at a time; turning the HUD off closes the query. Opaque to clicks.

### Lock screen

Only with a master password (`vault.VaultState`, specs/25). Without one nothing locks and Lock OneTerm says why: "OneTerm cannot lock: <reason>" for an unusable vault, "No master password is set up. Managed from the normal OneTerm window." in a read-only window, otherwise "Set up a master password in Settings > Security first: the lock screen asks for it." Locked: every window shows only the lock view: icon, "OneTerm is locked", masked input (Enter submits), the error from `apply` with the state's own back-off (2/4/8/16/30 s from the fifth wrong try), Unlock, how to recover (delete `vault.json` in the printed folder), Quit OneTerm. Sessions keep running. Lock after `lock_after_idle_minutes` (15, clamped 1–480); keys, mouse moves, clicks and scrolls count as input; a machine that slept past the timeout locks before the input lands. `ctrl-shift-l` locks at once.

### Elevated window

`--elevated-shell cmd|powershell|pwsh` plus a token check: `process_elevation()` is the only source of the marker and of every gate. `OneTerm (Administrator)` in the OS and app title bars (the suffix bold in `warn`); a failed query reads `OneTerm (elevation unknown)` and is restricted all the same. Restrictions: local shells only; no SSH, SFTP, saved sessions, Quick Connect, right dock, updates or `Run as administrator`; writes no settings (`save` and `docks.json` refuse); does not read `docks.json`; the shell is the fixed path under `%SystemRoot%` or `%ProgramFiles%`, ignoring `terminal.json`'s shell settings and `cwd`; Duplicate and New Terminal Here open at home. `action_allowed_when_elevated(id)` is the table of allowed ids; an unknown id is denied. The token query and the launch are in `quadra/native/sys`. Launching: `ShellExecuteExW` verb `runas` of the running executable with `--elevated-shell <token>`, off the UI thread, one at a time; a declined prompt says nothing.

## Tabs

A tab owns a tree of Spaces. Only the active tab's terminals are in the element tree; the
others keep running and keep their screens.

| Operation | Behaviour |
|---|---|
| New | `+` menu (Window), `ctrl-t` opens the default shell |
| Tab menu | Rename..., Duplicate, separator, Input Channel ▸, separator, Close, Close Others, Close to the Right ("Close Tabs Below" on the rail); the bulk rows are disabled when they would close nothing and confirm when they close more than one tab; every row acts on the tab clicked |
| Rename | double-click too; a dialog with the field "Tab title", Save and Cancel; empty is refused with the toast "Tab title cannot be empty." and the dialog stays; a manual title beats every other title |
| Duplicate | the same shell in the source's live cwd |
| Close last tab | resets in place to one empty tab titled "Terminal" |
| Reorder | drag in the strip or rail; middle-click closes |
| Drag into a Space | a tab dragged out of the strip or rail onto an **empty** Space moves its active terminal there (the emptied tab closes); a Space holding a terminal refuses it |

**Title**: manual title, else with `tab_title: "osc"` the OSC 0/2 title (an absolute path is
trimmed to its base name), else the shell name or the program without `.exe`, else "Terminal".

## Spaces

A binary tree of terminals (`work.PaneNode`) with drag handles.

- Split Right/Left/Up/Down adds an **empty** Space next to the active one and makes it active.
- An empty Space shows the terminal icon, "Space #N" and "Right-click → New Terminal Here" (default shell).
- Close Space collapses the tree. A lone Space has no chrome; otherwise each has a 1 px
  border, and the active one a 2 px accent ring. Clicking a Space activates it.
- A process that exits leaves its Space showing the last screen and the exit line.

Terminal context menu, in order: New Terminal, Duplicate Session, Split Right, Split Left,
Split Up, Split Down, Copy, Paste, Select All, Clear, Log ▸ (Start, Stop), Close Terminal Tab, Close Space.

## Key bindings

Global defaults; `ui_config.json` `key_bindings` overrides them by id, `""` unbinds.

| Id | Default | Id | Default |
|---|---|---|---|
| close_panel | ctrl-w | terminal_copy | ctrl-shift-c |
| new_terminal_tab | ctrl-t | terminal_paste | ctrl-shift-v |
| quit | ctrl-shift-q | find | ctrl-f |
| open_settings | ctrl-, | terminal_select_all | ctrl-shift-a |
| split_right, split_left | ctrl-shift-right, ctrl-shift-left | close_space | ctrl-shift-x |
| split_up, split_down | ctrl-shift-up, ctrl-shift-down | toggle_right_dock | ctrl-shift-b |
| lock_oneterm | ctrl-shift-l | new_ssh_session | ctrl-shift-n |

Also, shipped unbound: toggle_gutter, toggle_fps, about, terminal_clear, duplicate_session, the channel ids, open_session, delete_session, session_property, the nine sftp_* ids. The ids, labels and groups are the table of `keys.v`, shown in that order on the Key Bindings page. A binding row shows `Default: <key>` (or `Default: (unbound)`) only while it differs from the default. Edit captures the next non-modifier key (Esc cancels); a key taken by another action in the same key context is refused with a notice naming it. Overrides store only rows that differ; `""` unbinds.

A shipped default that a user override already holds, in the same context, loses at start, after a rebind and after a Reset: the action is left unbound. A Reset that ends that way says "Still unbound: <label> holds this key. Rebind either one to free it." on the row.

In the terminal: `ctrl-minus`, `ctrl-=` (and `ctrl-+`), `ctrl-0` zoom the shared terminal font of every terminal in the window (±1 px, 6–100, not saved);
`shift-pageup/pagedown/home/end` scroll; `shift-insert` pastes; `ctrl-c` always interrupts.

## Config

`%USERPROFILE%\.OneTerm\`, or `--data <dir>` (development and tests always pass it, inside
the project). Written atomically with a `.bak`; an invalid file is moved aside.

Reading a file has three outcomes. Missing: it is created with the defaults at first run, except in an elevated window, which writes nothing. Unparsable: moved aside, defaults. **Existing but unreadable** (permissions, I/O): the defaults are used and that document is never written, because the file may still be valid; each document has its own `persist_blocked` flag, which is never saved. An elevated window whose `ui_config.json` is absent or unreadable says once "Settings could not be read for this account; this window is using the defaults."

In `ui_config.json`, one bad `lock_after_idle_minutes` (not a whole non-negative number) or `vertical_tabs` (not an object, or a field of the wrong type) costs that setting or field, never the document.

`terminal.json` (comments allowed), defaults:

| Block | Keys |
|---|---|
| font | family "Lilex", size 15, weight "normal", features [], ligatures true, fallbacks ["Symbols Nerd Font Mono", "Symbols Nerd Font"] |
| cursor | shape "block" (bar, underline), color null, blink true |
| layout | line_height 1.2, cell_width null, padding {top 0, right 5, bottom 0, left 10}, show_gutter false, tab_title "default" (osc), semantic_highlighting "auto" (on, off) |
| shell | kind "cmd" (powershell, pwsh, bash, zsh, sh, custom), program null, args [], env {}, cwd null (home), utf8 true |
| scroll | multiplier 1.0, alternate_scroll true, scrollback_history 10000 |
| mouse | show_context_menu, copy_on_select, middle_click_paste: all true |
| bell | enabled true |
| colors | foreground, background, cursor, selection, gutter_fg, gutter_bg, clock_fg, line_number_fg: null (theme); min_contrast 0; ansi [] |
| completion | enabled, accept_tab, fuzzy, inherit_ancestor_options, disable_in_alt_screen, require_prompt_region, redact_sensitive: true; windows_allow_coreutils false; max_history 500, min_prefix_len 1, max_visible_items 8, force_family null, sources {memory, manual, external: true} |
| logging | local false, ssh false, directory "<config>/logs", write_mode "append" (overwrite) |
| sftp | editor {mode "os_default" (custom), program "", args []}, edit_max_file_size 1048576 |
| ssh | keepalive_enabled true, keepalive_interval_secs 30, keepalive_max 3 |
| security | allow_clipboard_read false |

`ui_config.json`: `ui_font_size` 16, `theme_name`, `key_bindings`, `right_dock`, `show_fps` false, `lock_after_idle_minutes`, `vertical_tabs`. `update_config.json` holds `auto_check` true, `channel` stable, `check_interval_hours` 24, `proxy_url`, `verify_certificates` true, `skipped_version`; the settings window merges into the file, so the updater's cache fields survive.

Themes are Quadra theme files (specs/15): the 39 bundled ones are converted from the original by `themes/convert.py` into `themes/*.json`, embedded through the generated `themes_embedded.v` and parsed on demand; `<config>/themes/` adds the user's. The list is sectioned Light / Dark: the section holding the selected theme first, the selected theme leading it, the rest by case-insensitive name. The default is One Dark.

### Settings window

`kit.settings(...).cards()` with the pages of the original, in order, in a 1000 × 700 window (capped at 90 % of the display and centred, at least 480 × 400) whose title bar is 34 tall with a 16 px title. Every field applies at once and is saved. Reset All is a small ghost icon button (undo) with the tooltip "Reset All" at the right end of the page header, shown only while the page differs from its defaults. The search field above the navigation filters rows, groups and pages. Page icons: General sliders, Key Bindings menu, Terminal square terminal, Completion search, SSH network, Security lock, Network globe, About info. Number fields are steppers 128 wide, dropdowns compact selects, text fields 256 wide. The theme takes the original's shape (`themed()`): `radius_lg` 4, `radius_xl` 6, `input_bg` = `line` at 30 % in dark themes, and the controls `plain_toggles` and `select_input`.

| Page | Groups |
|---|---|
| General | Theme (Mode, Color Theme), Interface (UI Font Size, Show FPS Monitor, Vertical Tabs), Shell |
| Key Bindings | App Menu, Edit Menu, Terminal Context Menu, Input Channel, Session Tabs Context Menu, SFTP Context Menu |
| Terminal | Font, Cursor, Layout, Scroll, Bell, Mouse, Security, Logging |
| Completion | Completion |
| SSH | Connection, SFTP Editor, SFTP Edit Limit |
| Security | Master Password, Screen Lock |
| Network | GitHub Connection |
| About | Application, Links, Updates |

A group shows the original's description when it has one (`kit.SettingsGroup.description`); a page whose single group would repeat its name (Completion) carries the description on the page and leaves the group untitled. The SSH keepalive settings apply at the next connection; the SFTP editor and edit limit reach the browser at once.

- An elevated window has no Network page (its updater does not exist).
- **Shell**: the dropdown offers all seven kinds by `display_name`. Custom Program is for Custom; changing the kind clears it.
- **Font Family**: a searchable list (about five rows high, "Search fonts...") of "Default (theme)" and the installed and bundled families (`TextSystem.font_families`). "Default (theme)" stores `''`, the theme's monospace font. With no enumeration only that row is listed.
- **Log Folder**: clicking the field opens the system folder picker ("Select terminal log folder").
- Dropdowns show the original's labels; the config keeps its ids.
- **Number fields**: keepalive interval 5–3600 step 1, keepalive max 1–20, scroll multiplier 0–20 step 0.5, edit limit 0–4096 MB whole numbers.
- **About**: the Application group is a centred 96 px icon, the name, the version and the tagline, as in the About dialog. A `kit.SettingsRow` with neither title nor description gives its control the whole row.

## Completion, semantic highlight, toasts, progress (P3)

The engines are specs/24. The view side uses `Terminal.hooks` (specs/17); a `TermExt` per terminal holds a `completion.Controller`, a `highlight.Overlay` and a `termlog` controller, one `completion.History` is shared by the app.

- **Input line**: on every Terminal render the line under the cursor is read. A row with an OSC 133 B mark gives the cells from `Terminal.input_start` to the cursor and the region is `Terminal.in_command_input()`. **Decision:** any other row falls back to the original's `strip_prompt` over the cells up to the cursor, and the region is "a prompt was found", which also covers shells without integration.
- **Overlay**: anchored under the start of the token (cursor column minus `typed_len`), flips above the cursor row when it does not fit below, kept 8 px inside the window; 180 px minimum width, `↑ N more` / `↓ N more` rows around a `max_visible_items` window, matched prefix in accent, description italic. Rows end with the kind as a muted lowercase word (`history`, `command`, `option`; white on the selected row, whose fill is the first series colour), as the original's code ships. **Decision:** the overlay shows the command-path breadcrumb when a command is resolved; the original passes none.
- **Keys**: `ctrl-shift-space` forces it open (`min_prefix_len` bypassed). While visible: Tab selects the first row then accepts (`accept_tab`), Up/Down and `ctrl-p`/`ctrl-n` move only once something is selected, Enter accepts only a selection, Esc dismisses (swallowed). Enter with nothing selected runs the command and records the line first. Accepting writes `Accept.bytes()` to this terminal only, no broadcast. A program taking the alternate screen dismisses.
- **Highlight**: `layout.semantic_highlighting` `off` disables; `auto` and `on` are the same, as in the original. Role per logical line from the first marked cell: prompt only when the line above left the prompt region or closed it with B, output for an output head, otherwise unmarked (the prompt detector runs). Style table: `highlight.load_default_styles()` in every theme, as in the original. Classes are cached per logical line by its rows' generations; a rescan reuses its buffers and scans the cells' code points directly, so a screen that changes every frame allocates nothing for it (a full-screen animation rescans every line each frame). URL coloring skips a row with no OSC 8 cell, `:` or `.`. The exit tint applies only at the bottom of the scrollback.
- **Toasts** (OSC 9): `kit.notify(.info)`. **Progress** (OSC 9;4): 5 px bar at the Space's top, 40 % muted track; blue for set and indeterminate (full), error red, paused yellow.
- **Status** (OSC 7501): stored per Space; `idle` draws nothing; the process ending clears working and blocked. **Pointer** (OSC 22) and icon name are not shown, as in the original.
- **Logging**: `logging.local` starts a logger with the terminal (file name identity: shell name plus the app's pid, since the PTY exposes no child pid); the context menu's Log ▸ Start/Stop toggles it, reporting success and failure in toasts. The original keeps these rows in the terminal menu, not the tab menu.

## Services (specs/25)

- **Vault**: `vault.VaultState` lives on the app. The Security page and the lock screen use `begin_*`, `job.run()` on a worker owned by the OneTerm view (a closed settings window cannot lose the result), `apply`, then `take_followup` (rehash). Dialogs: Set Up, Unlock, Change, Reset, with the original's copy.
- **Update**: `update_config.json` is shared with `update.UpdateConfig`; the UI builds the engine's config from the file and the app's preferences per run and merges the check cache back with `UpdateCheckCache.save_to`. Status text and buttons as in the original; About also has the Update Status, Install and Skip controls. Auto-check at start only on `-prod` builds unless a client is injected (`OneTerm.update_client`; the original skips debug builds too), never in an elevated window. Download, confirm, `install_staged_update`, quit.
- **Crash**: the platform filter first, then `crash.install` chaining it, at the very start of `main`. The next start shows one dialog per report, newest first (`crash.load_pending_reports`): Dismiss deletes, Copy, Create Issue. An elevated window uses `crashes/elevated/` and shows nothing.
- **About**: version `0.6.11`, the original's numbering; a lower version would offer the original's own releases as an update from the same repository.

## Remote (the wiring)

`connect.v` and `connect_place.v`, `dock.v` (panels and following), `gutter.v`.

- **Connecting.** `sessionui.ConnectRequest` becomes an `ssh.Config`: `known_hosts_path` is `~/.ssh/known_hosts`, or `<data>/known_hosts` under `--data` (development and tests never touch the real one); the SFTP Local pane starts at `<data>` there too. Password with an empty value is auth none. Keepalive comes from settings. `ssh.connect` runs on a task whose result is a plain `ConnectOutcome` (spawn_task only carries strings); secrets are wiped after it returns and never stored on a Space. `on_cancel` cancels and forgets the attempt, a later result closes its session silently.
- **Errors.** `HostKeyUnknown` goes to `host_key_unknown`; `ConnectError` and `HostKeyChanged` to `connect_failed` by their own text; anything else gets `SSH connect failed: `. **Decision:** a hop's failure is passed as its inner message, starting `jump host <user>@<host>:<port>:` (specs/26 needs the prefix first), so it loses the `SSH <phase> failed:` the original also shows.
- **Landing.** A new connection opens a tab named after the session. **Decision:** when the active tab is one empty Space, the connection fills it instead (the original adds a tab beside the empty one). The tab has no colour and no SSH marker: the original gives it neither. Logging follows the session's override over `logging.ssh`, file identity `<user>_<host>_<port>`.
- **Lost connection.** No reconnect, as in the original. The Terminal is `quiet_exit`: no exit line; one toast `SSH connection closed.`, a banner at the Space's bottom `SSH connection closed. Input is disabled.`, SFTP unbinds, broadcast leaves.
- **Following.** `sync_active` runs on every change of the active tab or Space: dock kind, `sftpui.set_session` (keyed by the session's `Sftp`), and the OSC 7 path to `sftpui.follow_cwd` (`''` for a local terminal); a remote terminal's cwd change pushes it again.
- **Duplicate.** The Duplicate Session row is a submenu: In New Tab, Into Space #N (each empty Space of the tab), Split Right, Split Down; disabled when the session has ended. Local re-runs the shell in the live cwd; SSH opens the Duplicate dialog prefilled from a secret-free `DuplicateSpec`, its tag naming the destination, then runs `cd -- '<cwd>'`. Tab menu Duplicate and `duplicate_session` go to a new tab.
- **Closing.** One tab closes unconfirmed (×, middle, Close, Close Terminal Tab); a bulk row closing more than one asks "Close Tabs". `close_space` needs a sibling Space: alone it does nothing and the row is hidden. Tab close ends its sessions; at exit the browser is disposed and SSH sessions closed.
- **Keys.** `new_ssh_session` opens Quick Connect, as the original's `NewSession`. The three session ids and the nine `sftp_*` ids fire only with focus inside their panel (`key_context` `SessionPanel`, `SftpPanel`), where the original handles them; the browser's menu shortcuts come from the same table.
- **Gutter.** `layout.show_gutter`, `toggle_gutter`: `[HH:MM:SS]  <n>` left of the grid, via the terminal's `overlay` and `output` hooks (specs/17). A line is stamped once when it first has content, even in a hidden tab; numbers run with the line feeds received, never below history + rows; a cleared scrollback drops the old stamps. Width is `(11 + digits) cells + 8 px`, added to the left padding. Colours: `gutter_bg` (terminal background), `gutter_fg` (foreground at half lightness), `clock_fg` and `line_number_fg` (gutter_fg).

## Local shell

ConPTY (specs/17, ConPTY host): the bundled Windows Terminal host when `conpty.dll` and
`x64/OpenConsole.exe` sit beside `oneterm.exe`, the system host otherwise; the choice is logged at
start (`conpty: bundled` or `conpty: system`). `tools/conpty.sh <dir>` stages the pair from the
original's `crates/app/assets` (fetched by `tools/setup.sh`): it checks both SHA-256 digests against
`conpty-manifest.json`, copies through a temporary name only when the bytes differ (a running
OneTerm may hold the files), and writes `conpty-NOTICE.txt` (the MIT notice) beside them.
`tools/package.sh apps/oneterm ...` runs it into `dist/<exe>/`; for a dev build run
`tools/conpty.sh .tmp` next to `.tmp/oneterm.exe`. Both places are git-ignored: the binaries are
never committed. Environment: `TERM=xterm-256color`, `COLORTERM=truecolor`,
`TERM_PROGRAM=OneTerm`, `TERM_PROGRAM_VERSION`, `LANG` (inherited or `en_US.UTF-8`); it drops
`WT_SESSION` and other terminal identities. Shell integration needs no script files:

| Shell | How |
|---|---|
| cmd | `/K chcp 65001 >nul`, a `PROMPT` that emits OSC 7, then 133 A, the path and `>`, then 133 B |
| powershell, pwsh | `-NoLogo -NoExit -Command <init>`: wraps `prompt` to emit D;exit, OSC 7, A, B |
| bash | `-l`; `PROMPT_COMMAND` and `PS0`: A, B, C, D, OSC 7. A user `PROMPT_COMMAND` that already emits OSC 133 is left alone; any other is appended after ours |
| zsh | `-l`; `PS1` with D, A, B (no C: that needs a function), unless the user set `PS1` |
| sh | no integration |

Bash, Zsh and Sh are found on `PATH`, then at `/bin/<name>`, then through `$SHELL` when it names the same program. `PROMPT` and `PS1` set in `shell.env` win; `shell.args` come after the kind's own arguments.

## Terminal behaviours (O3)

| Feature | Behaviour |
|---|---|
| Selection | drag; double = word across soft wraps (stops: `,│\`|:"' ()[]{}<>` and tab; a bracket selects to its partner across lines), triple = the whole logical line across wraps; alt = block for any click count; a drag keeps the word or line unit; the ends are anchored to rows: output scrolling moves them, a row leaving the history clears the selection, as does a width change, and erase rules are specs/17 |
| Copy | copy-on-select by default; Select All includes the scrollback |
| Paste | cap 1 MiB (warning toast). Bracketed strips ESC and ETX; plain rewrites LF to CR. Middle-click pastes; apps in a mouse mode get the click unless shift is held |
| Search | `ctrl-f` toggles a 360 px bar top right of the terminal (one per terminal, survives a tab switch): placeholder "Find in terminal", `Aa` "Match case", `W` "Match whole word", counter `n/total`, "No matches" or empty, up "Previous match (Shift+Enter)", down "Next match (Enter)", close "Close (Esc)"; matches run top to bottom, one row at a time, and start at the first; Enter next, shift-Enter previous, both wrap; 150 ms debounce; refreshes on output keeping the index; the match is centred |
| Scroll | the wheel goes to mouse reports, alternate-screen arrows or the scrollback, times `multiplier`. Typing returns to the bottom; output does not |
| Cursor | bar 20 % of the cell, 500 ms blink when focused, hollow when not. The settings shape overrides DECSCUSR (only hide is honoured) |
| Resize | grid = floor(px / cell) less the padding, 1–1024 rows, 1–2048 columns; the main screen reflows |
| OSC 7 | sets the cwd (status bar, Duplicate); accepts raw paths as well as `file://` URLs |
| Bell | a bell badge top right, cleared by typing; no sound |
| IME | composition at the cursor; off on the alternate screen |
| Links | OSC 8 and detected URLs (`http://`, `https://`, `www.`, across soft wraps, Unicode letters included). Detection, the pointer and the hint follow the editor and the Markdown preview, a deliberate departure from the original by the user's decision on 2026-11-10: the shared `quadra/text` rules (a word boundary before the scheme; `<`, `>` and `"` end a URL; a `)` closing a `(` inside it stays, `.../Foo_(bar)`, also across a wrap; no `ftp://`, which never opened), and the pointer and the underline show only while ctrl is held over a link (hitboxes with `cursor_mods`, updated on the key alone). Resting on a link shows "Ctrl+click to open" (a hitbox hint, specs/08), except while a program reads the mouse. Ctrl-click opens through the target policy: https opens; http, a non-default port, or an OSC 8 label naming another host asks "Open link?"; credentials, control or bidi characters, too long (256 KiB), an invalid URL or any other scheme is denied and logged |
| OSC 52 | write allowed (256 KiB); read refused unless `allow_clipboard_read` |

Engine: `kit/term` gains what these need (specs/17). Its conformance test replays the 45
recordings of the original's corpus at their geometry and compares grid text, wrap flags,
attributes, colors and cursor. The corpus is Apache-2.0 and is not carried in this repo:
`tools/setup.sh` fetches it into `.cache`; `tests/term_corpus_test.v` requires every recording
to pass and skips when the corpus is absent.

## Deviations from the original

Proposed defaults, cheap to reverse:

- `ctrl-shift-up/down` split; the original binds them to split and scroll at once. Scrolling keeps `shift-pageup/pagedown`.
- Clear erases the screen and scrollback in the terminal; the original types `clear`, which `cmd` does not have.
- ANSI 0–15 come from the theme (specs/15); the original fixes them and only `colors.ansi` overrides. `colors.ansi` still wins here.
- A partial settings block keeps defaults for what is missing; the original rejects the file.
- Themes use Quadra's format, not the original's; the two default themes are named One Dark and One Light.
- `colors.foreground`, `selection` and `line_number_fg` default to null (theme), so a light theme stays readable; the original pins them to dark-theme values.
- Git and the process sample run on `core.spawn_task` threads; CPU/MEM is the process only.
- Session group separators in the `+` menu are labelled rules; the kit draws no dashed rule.
- Moving the pointer diagonally out of a submenu row can switch to the row it crosses; there is no hover grace.
- A tab activates on release, so a reorder drag does not activate the tab it moves. Needed so an inactive tab can be dragged into the current tab's empty Space.
- General > Shell also has a UTF-8 row, for `shell.utf8`, which the original has no control for.
- A Custom shell with an empty program tries to run `custom`; the original reports an error. Neither starts a shell.
- The Font Family list does not resync when the config file is edited from outside.
- In an elevated window, Reset All can show on About, because the page index is shared with the normal window.
- A readable but corrupt `update_config.json` is replaced when update settings change; an unreadable one is left alone.
- Network and About share one Reset All: both are the `update_config.json` block.
- The rail's list shows the kit's auto-hiding scroll thumb; the original draws none.
- A tab being dragged shows no insertion line; the drop slot is the row or tab under the pointer.
- `ui_font_size` scales the kit's text sizes (`text_scale = size / 16`): 16 keeps the kit's own scale.
- Bundled theme files are named `<slug>.theme.json`: V 0.5.2 merges `$embed_file` entries with the same relative path, and the kit also embeds `themes/one-light.json`.
- The gutter's clock and number are separate text runs one cell apart, drawn over the terminal's padding, not shaped in the grid.
- SGR 21 cancels bold, as the corpus expects; a double underline is `4:2`.
- Saving from the settings window rewrites `terminal.json` without its comments; the original preserves the document.
- `font.features`, `font.fallbacks` and `ligatures: false` are applied (specs/17, View); an entry in `font.features` wins over `ligatures` for `calt`.
- `layout.cell_width` overrides the measured advance of `0`; the width is then snapped to whole device pixels with the rest of the cell. `colors.min_contrast` of 0 keeps the default of 4.5, as the original does; 0 < v <= 1 turns it off.
- `cursor.color` paints the cursor; null uses the theme's.
- Zooming affects the shared font size of all terminals until the next reset.
- `font.weight` is parsed case-insensitively and accepts `regular`, `extralight` and `extrabold` beside the listed names.

## Testing

Headless, on `FakePty`: tabs, Spaces, menus, key bindings, config round trips, shell command
lines and the environment. Every run uses `--data` inside the project. A manual pass on real
ConPTY covers each milestone.

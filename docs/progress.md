# Progress

Status per phase of specs/00-migration.md. Run everything with `tools/check.sh`. Since
2026-11-16 Quadra stands on its own (no port of the V implementation): where an entry below
compares with the V build, its suite or its pictures, that is the history of the phases.

| Phase | Status | Evidence |
|---|---|---|
| R0 repository and toolchain | Done (2026-11-11) | `tools/check.sh` and `tools/check.sh prod` pass; `quadra-prim` (points, rects, edges, corners, colours, affine transforms) with its tests |
| R1 vertical slice | Done (2026-11-11), with the evidence for its go/no-go: the four programs run in real windows and the comparison bench is at the V build's numbers or under them. The structural budgets that need input or style variants move to R2 and R3 with those features | See "R1" below and docs/benchmarks.md |
| R2 interaction | Done (2026-11-11), see "The exit of R2" below: pointer input (hit testing, dispatch with capture and bubble, hover, press, pointer capture, clicks with counts) from the platform's events to handlers on views; styles with state variants (`hover`, `active`, `when`), resolved by the style pass for the nodes whose state changed; the wheel, which scroll views and virtual lists consume as far as they can scroll and pass on; focus (a press focuses, Tab walks the tree inside a focus scope, the ring shows for a focus that came by the keyboard, a removed node hands its focus to an ancestor) and keys dispatched along the focus path; `NodeRef`; actions as types and the keymap (keystrokes and sequences, contexts by name, the nearest context and the latest binding win, an unhandled action leaves the key a key; Tab is a binding); the text input (`text_input(value)`: typed text, the editing keys as bindings of the context "text", pointer placement and selection with double and triple clicks, undo, sideways scrolling with the caret; `crates/quadra/examples/form.rs` runs it in a real window, Vietnamese and emoji typed through `WM_CHAR`), with copy, cut and paste through the clipboard of its context, a caret that blinks on a timer while the input has the focus, and input methods: what is composed shows at the caret, underlined, and the window is told where the caret is (IMM32 on Windows; composing with a real input method is a manual check still to do). Overlays: roots above the main tree, each a layer, placed beside a node, at a point or in the middle of the window, hit first, told of presses outside (`overlay`, `Place`; the example's menu). Semantics: the tree's semantic side (`semantic_tree`, `act`, `listen` and `take_semantic_update`), set from views with `role`, `label`, `test_id` and the state setters; its updates are well formed by the rules the OS adapter enforces (a `Mirror` follows the tree in tests, the random test included). The bridge to assistive technology: the platform's `SemanticsWanted` and `Semantic` events and `PlatformWindow::update_semantics`, wired by the app; on Windows the adapter of `accesskit` (UI Automation), lazy until a reader asks. Scrolling into view (`NodeRef::scroll_into_view`, a focus that came by Tab, the semantic action, `.reveal(..)` on lists for a row mounted or not). Tab index (`.tab_index(n)`). Text that can be selected (`text(..).selectable()`). A password input (`.password()`) and `.on_submit(..)`. The text area (`text_area(value)`): lines broken at its width, Enter, the caret up and down keeping its x, Home and End by shown lines, selection across lines, the caret kept in view by a scroll view around it; typed into and selected in a real window. Cursors (`.cursor(..)`, the text cursor over an input; `PlatformWindow::set_cursor`). Lists with rows of different heights (`virtual_list(count, estimate, scroll_y, row)`): rows measured after the layout that follows their mounting, heights in a binary indexed tree, what shows kept in place when a row above it turns out another height. The form example was read and driven in a real window through UI Automation (texts, the input's value set, the menu expanded, an item invoked). Drag and drop inside a window (`.draggable_with(..)`, `.on_drop(..)`, `.drop_target(..)`; `crates/quadra/examples/board.rs`, dragged in a real window). A value that eases to its target by the window's clock (`tween`). Since the exit (2026-11-14), readers are told the caret and the selection of an input and can set them, the numbers of sliders and progress bars, where an item is in its set and how many the set holds (a list of a thousand rows says so), and which boxes scroll; read back through UI Automation in `crates/quadra-native-win/tests/access.rs`. Since then also drops from the OS (files, text) and relations (named by, described by, controls). Since then also the bounds of a range of text, a run a line of a text area, the clause of a composition, the level of a row of a tree, contexts as predicates (`editor && !find`) and the keymap file (`Keymap::load`, `Actions`; `tests/keymap_file.rs`) | `crates/quadra-tree/tests/input.rs` (6), `tests/focus.rs` (6), `tests/keymap.rs` (5), `crates/quadra-text/tests/edit.rs` (5), `crates/quadra-view/tests/text_input.rs` (4), `tests/clipboard.rs` (2, on a clipboard in memory), `tests/composition.rs` (Vietnamese composed with Telex and committed), `tests/overlay.rs` (3), `tests/semantics.rs` (5: the tree as text, controls found and driven without the pointer, a failed query printing the tree, a listener hearing only what changed, a mirror that holds the tree after every update), `crates/quadra-native-win/tests/access.rs` (a UI Automation client on a second thread reads and drives a hidden window), `crates/quadra-view/tests/tween.rs`, `crates/quadra-view/tests/text_area.rs` (4), `tests/selectable.rs`, `tests/drag.rs` (3), `crates/quadra-view/tests/reveal.rs` (4: nested scrolling to a node, Tab and a reader, a row of a measured list far away, a uniform list), `crates/quadra-view/tests/scrollbar.rs` (3: the thumb's length and place; shown by a move or a scroll, faded a second later, no frame asked for meanwhile; dragged), `crates/quadra-view/tests/strip.rs` (3: a strip scrolled sideways by the wheel, which passes on what it cannot scroll, with no scrollbar; its content as wide as the strip at least; a box that scrolls sideways leaves the plain wheel to the page, and has its bar along its bottom), `crates/quadra-view/tests/virtual_list.rs` (4: rows as tall as what they hold and one after another, a jump and the end, a row that grows, forty wheel steps up through unmeasured rows without a jump), `tests/caret.rs` (the budget "caret blink": one node painted, its range overwritten in place), `tests/style.rs` (3: the budget "pointer enters a button" is one node styled, none laid out, one painted), `crates/quadra-view/tests/input.rs` (5, with nested scrolling by the wheel and a form walked with Tab), one in `crates/quadra-app/tests/app.rs`; the bench's counter counts clicks posted to its real window |
| R3 kit | Done (2026-11-12), see "The exit of R3" below: tokens and the theme for the colours of boxes (`Paint`, `Token`, `Theme`, `Tree::set_theme`; the app's theme as a signal in the context of views). The kit's roles and themes (`quadra-design`: `token::*`, `dark()`, `light()`, `theme_from_json`, the two theme files). Text values that hold down the tree (`.text_color(..)`, `.text_size(..)`, `.font_weight(..)`, `.line_height(..)`, `.font_family(..)`; a text has no style of its own), with roles for the colour. The pose of a box (`.translate`, `.scale`, `.rotate`, `.opacity`: constants of its layer, the pointer goes by the box), transitions (`.transition(ms)`: the background, the border's colour and the pose ease, run by the tree) and timelines (`.animate(timeline()..)`), in `quadra-tree/src/motion.rs` (`crates/quadra-view/tests/motion.rs`, 4: a pose moves what is painted and keeps the box; a change of colour and pose takes its time, from where it is; a pose that eases paints nothing and a new theme shows at once; a timeline by the clock). Rich text (`rich_text([span(..).bold(), span(..).link(..)]).on_link(..)`): a paragraph of runs (`Paragraph::rich`, `Shaping`; a word across runs is shaped in pieces and never breaks there), the widget `RichText` in `quadra-tree/src/rich.rs` (colours of spans may be roles: `Widget::theme`; links are underlined, take the pointer and their click), `crates/quadra-view/tests/rich.rs` (2) and a test of the paragraph. Since then: links for the keyboard and a reader, a selection over the texts of a document (`quadra-md`), lines as high as their largest run, the measures of radii and spacing (`token::MEASURES`), and the components (R3 to R6) | `crates/quadra-tree/tests/style.rs` (a theme change styles and paints the nodes that hold roles, lays out none), `crates/quadra-view/tests/theme.rs`, one in `crates/quadra-app/tests/window.rs`, `crates/quadra-view/tests/text_style.rs` (values from the boxes around, a size that changes lays out what has it from there, a colour only paints), `crates/quadra-design/tests/theme.rs` (3: the built-in themes, a file that lists what differs, errors named by their path) |
| R4 domain crates | Done (2026-11-14), see "The exit of R4" below: `quadra-viz` is done (the row of the crate below). The live table is the kit's table told the keys of its rows (the row of the kit below; its page differs from the V picture in 2.17 % of the pixels, as the table's does). `quadra-data` is done (its row below). `quadra-code` has its tokenizers and `quadra-md` its parser and its view (their rows below). Images are in (`image(source)`, the row of `quadra-view` below), and the documents of `quadra-md` show them. The editor of `quadra-code` is in (its row below); `quadra-term` is on hold | the tests of `quadra-viz` |
| R5 release | Done but for the manual checks (2026-11-14), see "The exit of R5" below: specs 15 (app services) and 16 (shell services). Done: the app's folders, settings, the log and crash reports of panics, dates and times, local time and the app's language, window state, pages and closing (the row of `quadra-app` below). `quadra-shell` (the row below), packaging (`tools/package.sh <example>`, `tools/app.manifest`), the API documentation (`cargo doc` in the gate, warnings denied; the facade re-exports the app's services and says where things are) | `crates/quadra-app/tests/settings.rs` (2), `tests/crash.rs`, `tests/time.rs` (2), `tests/i18n.rs` (3), a remembered window in `tests/settings.rs`, `tests/router.rs` (2), `tests/close.rs`, a place on no display in `quadra-native-win/tests/window.rs`, the calendar in both languages in `crates/quadra-design/tests/dates.rs`, the offset of the time zone in `quadra-native-win/src/zone.rs` |
| R6 base components | Done (2026-11-14), see "The exit of R6" below: the spec of the base and of the open tokens (specs/17); the shadows of each level above the window are the theme's (`Theme::with_shadows`, `Cast`), the kit's dark and light themes give theirs, and the gallery's pictures are the same to the byte. `quadra-headless` has the actions and the keymap (`quadra_headless::action`, re-exported by the kit), the popup (`popup::popup` on a plane the look gives, `open_now`, `hang`, `Edge`) and one of a few (`choice::group`, `choice::option`), the button and the toggle (`press`), the list and the places of a sidebar (`list`), the sections of an accordion (`disclose`) and the split (`split`), the menu (`menu`, with a row's own menu and `on_context`), what is picked by typing (`pick`: `fuzzy`, `best`, `Found`) and the combobox (`combo`), the tooltip and the hover card (`hint`), the dialog (`dialog`) and the notifications (`notices::Queue`), the slider and the range (`slider`), the number input (`number`), the parts of a time and the days of a calendar (`time`), the rules of a field (`rules`), the tree (`tree`, `Branch`), the table (`table::Sheet`) and its live data (`live`), the search and pages of settings (`settings::Model`), the workbench (`work`: the pane tree, docking, the tab strip's `Strip`); the kit's popups, radio groups, segmented controls, tabs, buttons, checkboxes, switches, lists, sidebar, accordion, split, menus, context menus, selects, combobox, command palette, tooltip, hover card, dialogs, drawers, notifications, sliders, number inputs, time input, calendar, date picker, field rules, tree, table, live table, settings, panes and tab strip are built on them; a second kit, `examples/plain` (`quadra-plain`: square, flat, ink on paper, with no code of the kit), has a button, a checkbox and a switch, radios and tabs, a slider, a select with its menu, a dialog and a list, in a window (`tools/cargo.sh run -p quadra-plain`). The renderer has gradients of several stops, linear, radial and conic, mixed in Oklab (`Color::mix_oklab`; `Gradient`, baked into a ramp of the colour atlas that the shader samples; `.gradient(..)` on a box, `Tree::set_gradient`) and dashed and dotted borders (`BorderStyle`, `.border_style(..)`: marks along the middle of the border and round its corners' arcs, whole periods between the corners; the plain example's window shows both); what a frame costs the GPU is measured (`RenderStats::gpu`, D3D11 timestamps read back without waiting; the bench prints it: 146 µs for the grid, docs/benchmarks.md); a box blurs what is behind it (`.backdrop_blur(px)`, `Tree::set_backdrop`: a layer whose backdrop is copied before its first run into a texture whose mips are the blur, 25 µs of GPU for a pane of 600 by 400); a box is cut to any shape with what it holds (`.clip_shape(svg)`, `Tree::set_clip_shape`: a mask of coverage at the box's size, named by the layer's constants and taken by the layers inside it, so nothing of the instances changes); the grid's paint pass is as fast as before (a tile key of its own for ramps cost 8 µs a frame, so a ramp is an image piece no image has, and a node stays 256 bytes), with pictures the same to the byte; the gate fails when the base names a look | `crates/quadra-tree/tests/style.rs` (a theme with other shadows paints the node again and lays nothing out), `crates/quadra-headless/tests/base.rs` (2: a popup of bare boxes takes the focus, keeps Tab inside, closes by Escape, a press outside and its opener, and gives the focus back; a choice is one stop of Tab, its arrows go round, a click chooses), `tests/controls.rs` (5: a button, a checkbox and a switch, a list, a section, a split, each of bare boxes), `tests/popups.rs` (4: a menu with a row's own menu, what typing finds, a combobox, a tooltip and a queue of notices), `tests/values.rs` (4: a slider and a range, a number, a time and a day, rules), `tests/data.rs` (3: a tree by its keys, a table sorted, selected and copied to the harness's clipboard, a search of settings), `tests/work.rs` (2: a tab chosen, moved and closed by the middle button, a seam dragged and made even), `crates/quadra-native-win/tests/gradient.rs` (each kind of gradient through its stops to pixels on the software rasterizer), a test of pixels of dashes and dots and one of the GPU's time coming back in `tests/pixels.rs`, `tests/backdrop.rs` (2: the seam behind the glass soft and only there, the same each frame; a translucent fill tints it), `tests/mask.rs` (a box cut to a circle, its edge partly cut, and a layer in it cut too), a unit test of the Oklab mix and one of ramps, `examples/plain/tests/plain.rs` (6: a button and the toggles, radios and tabs by a click and the arrows, a slider by its keys, a select and its menu, a dialog answered or given up, a list by the arrows and a click) |

`quadra-term` was on hold from 2026-11-11 and is resumed (2026-11-15) for OneTerm (specs/18, 19). Done (2026-11-15): the session (`Session`, `Feeder::feed` returning the replies in order, `Link`, `echo()`; the signals `title`, `cwd`, `bells`, `progress`, `status`, `exited`, `on_notification`; OSC 7501 in `status.rs`) and the view (`terminal(&session)`: the grid sized from its box after layout, the snapshot taken in the update and painted alone, runs of background, a glyph a cell, bold and italic faces, underline and strike, the cursor's shapes and blink, the selection; keys through the engine's encoder, `TermCommand` in the context `terminal` for Tab, copy and paste, `bind_keys`; the pointer selects or reports, the wheel scrolls the history, reports, or sends arrows). Tests: `crates/quadra-term/tests/session.rs` (5), `tests/input.rs` (4), unit tests of OSC 7501 and of keys. `quadra-shell` has the pseudo console (`Pty::spawn(options, output, exited)`, `write`, `resize`, `close`: a thread owns the console and polls it; writes wait for room; `tests/pty.rs` runs `cmd /c echo`). The app OneTerm (`apps/oneterm`, package `oneterm`, binary `oneterm`; specs/19): M1 is done: the window with its own frame (1440 by 900, at most 85 % of the display), the title bar of 38 px (`Titlebar::height`) with the mark, the program's title and the count of spaces, one local shell filling the workspace with the focus (`terminal(..).autofocus()`; Command Prompt is told to say its directory, OSC 9;9), the status bar in mono 11 px with the directory (a click copies it) and the clock in the app's locale; `apps/oneterm/tests/app.rs` (2) on `echo()` sessions. The place remembered came with the app's folders (M5), the minimum size later (2026-11-18). M2: tabs and spaces live in the window's model (`Model`, each space a scope of its own under it), the rail (a row a tab: the accent bar and fill of the one that shows, its title cut with an ellipsis, `n local · n spaces`; a click shows it, the middle button closes it; New terminal), spaces side by side in equal columns with their 18 px headers and the focused one's ring, the keys Ctrl+T, Ctrl+D, Ctrl+W, Ctrl+Alt+Left/Right (bound before the shell sees them, as the design asks), `[process exited N]` and a key that closes the space; `apps/oneterm/tests/tabs.rs` (3). Then the rest of M2 (M2 is done): two clicks rename a tab in place (Enter keeps, Escape leaves), its menu (Rename, Duplicate, Close, Close others), rows dragged onto another take its place, and the horizontal variant (a strip of 36 px, `+`, Vertical; the rail's button goes to it; a setting in M5); `tests/tabs.rs` (6). M3 is done: a tab sums its spaces' signals (`signals.rs`, `marks.rs`): OSC 7501 as a mark at the end of its row (working spins, blocked asks in warn, done is checked, error crossed) and the word on its second line, a chip in the space's header, a card when a program waits or fails; OSC 9;4 as the percentage on the line, a bar of 2 px under the row, the header's percentage, a meter of 56 px in the status bar; a bell in a space not looked at as a red dot on the tab's icon and the status bar's count (the space with the focus hears its own); OSC 9 as a card at the top right under the title bar for 6 s (`Notices::below`, `Place::TopRight`), counted on the title bar's Notifications button and in the status bar until it is pressed; showing a tab sees its bells and its done; `tests/signals.rs` (5). M4 is done: the `+` menu under New terminal and the strip's `+` (Local shell, the shells found on the machine, `shells.rs`: Command Prompt, Windows PowerShell, PowerShell 7, Git Bash, WSL; Split current space; no search field until P2's hosts), a split starts in the focused space's directory, the status bar's git (`git.rs`: the branch from `.git`, the changed files from `git status --porcelain` on the pool with no window, again on a new directory and every 10 s) and OneTerm's CPU and memory every 2 s (`quadra_shell::process_usage`, specs/16); `NewSession` takes a `Launch`; a test of the menu in `tests/tabs.rs`, unit tests of git's `HEAD`. `Session` calls its link without holding it, so what the link calls may close or connect again. M5 is done: the settings (`prefs.rs`: each a signal of the app's `Settings`, a choice kept as its words) on a page that takes the window's body (`page.rs`: the title bar's Settings button, "OneTerm Settings · <page>", the search, the eight pages; Security & lock waits for M6), the app's folders (`AppDirs::for_app("OneTerm")`, so `settings.json` and the window's place are kept), the theme (System, Dark, Light; `oneterm::theme`), the terminal's size of text (the terminals mount again), cursor shape and blink and scrollback for new spaces (`Session::configured`; scrollback also live, `Session::set_scrollback`), the default shell and where new tabs start, vertical tabs, how states, progress, bells and notifications show, and the status bar's parts; `tests/settings.rs` (3). M6 is done, and with it P1: the master password (`quadra_shell::hash_password`, `verify_password`: PBKDF2-HMAC-SHA256 from BCrypt, 600,000 rounds, a random salt, compared in constant time; specs/16, "Passwords") set, changed and removed on Security & lock, the lock (`lock.rs`, `locked.rs`) after the idle time (steps of 10 s on the app's timers) or at once (`Ctrl+Shift+L`, the title bar's Lock), the card with the focus in its field, five wrong tries and the back-off; locked, the workspace, the settings and the cards are not displayed (so not in the semantic tree either; specs/19, "M6 in detail"), the app's keys do nothing, the status bar says only "Locked · details hidden" with the progress and the clock; `ONETERM_HOME` keeps a trial run's settings in one folder; `tests/lock.rs` (5), unit tests of the hash (RFC 7914's vector). Terminal throughput is measured by the fire demo written again (`bench/doomfire`: `doomfire`, `doomfire-feed`, `doomfire-conpty`; `quadra_shell::console`; `tools/doomfire.sh` runs it in OneTerm; specs/14, docs/benchmarks.md): OneTerm takes 340 fps at 147 by 40 on the system's console host, that host's own ceiling, and 592 with Microsoft's newer host beside it (`tools/conpty.sh`, specs/19 "The console host"; its ceiling is 634; `tools/package.sh oneterm` ships it with its notice); the engine alone takes 1,040 at 200 by 50. P2 has started (SSH over `russh` and `tokio` with `ring`, the user's choice; specs/01 R15 admits them for `oneterm-ssh` alone). M7: the engine `apps/oneterm/ssh` (`Ssh::connect(options, output, exited)`, `write`, `resize`, `close`; a runtime of two threads; `Target::parse`; the known hosts checked, an unknown key asked and written, a changed one refused; the agent, the usual key files and the password by `Auth`; every prompt asked in the terminal, `prompt.rs`; the shell's PTY, window changes, exit status, keepalive; `tests/live.rs` (5) against a `russh` server in the test on 127.0.0.1), and in the app `remote_shell`, `oneterm::start`, `Launch::ssh`, Quick connect (`connect.rs`: the `+` menu and `Ctrl+Shift+K`; a tab, or a space in the current tab; a duplicate connects again; `tests/connect.rs` (4)). M8: saved hosts (`hosts.rs`: the settings `ssh.hosts` and `ssh.recent`, rows of words; `Model::connect_to`, `save_host`, `remove_host`, `recent`, `host_name`), the rail's SSH hosts (`hostlist.rs`: a filter, the groups, a dot while connected, Connect, Connect in a new space and Remove in a row's menu, Add SSH host…), the `+` menu's Recent and hosts, Quick connect's Save as host, the engine's latency (an SSH ping every 5 s, `Ssh::latency`) in the session's new `link_note` (specs/18), shown in the space's header and with the host in the status bar, and a tab's `n ssh · n local`; `tests/hosts.rs` (3). Then a row's Edit… (the add dialog filled, titled Edit SSH host; a new name takes the old one's place; a test in `tests/hosts.rs`), and the `+` menu's search field (the kit's new `searched_menu` and `searched_icon_button`, specs/design: the select's search head over a menu's rows, asked again as the field changes, Enter does the first row): it keeps the shells and hosts that hold what is typed, and a typed target is a row `Connect to user@host`; a test in `tests/hosts.rs`. M9: SFTP in the engine (`Ssh::files`, `Files`: `home`, `list`, `download`, `upload` through `.part`, `mkdir`, `remove`; `safe_name`, `join`; the test server and client are the feature `testing` of `oneterm-ssh`, `tests/files.rs` (2)) and the dock (`dock.rs`, `sftp.rs`: the title bar's SFTP button, the focused space's connection found by `Session::kept`, a new `keep`/`kept` of `quadra-term`; the path's parts, folders first, a click opens a folder, two download a file to the setting `sftp.downloads` or the user's Downloads, a row's menu downloads or deletes, a new folder, files dropped from the OS upload, transfers with their percentage and a cross that cancels; it asks again each second until the shell is open; `tests/dock.rs` (3) against the test server). Then the expanded view (`expanded.rs`, `panes.rs`: Expand in the dock, Collapse SFTP in the title bar; it takes the workspace's place while the spaces run on; Local and Remote panes with Up, Refresh, Name, Size and Modified (`Entry::modified`), a click selects and two open; Upload selected, Download selected into the Local pane's folder, Compare marks `older` and `newer`; Sync cwd with terminal; `.hidden` for dotfiles in both views; the transfers with a bar each; a test in `tests/dock.rs`). Then (2026-11-17) folders' trees both ways (`Files::download_folder`, `upload_folder`, `folders.rs`: 32 deep, links not followed; a folder's Download, a local folder's Upload, folders dropped), transfers one at a time in each space's dock (`queued`, the cross takes a waiting one off), and the panes sorted by Name, Size or Modified (`panes::sorted`, folders first); tests in `oneterm-ssh/tests/files.rs` and `tests/dock.rs`. M10: jump hosts (`Options::jumps`, `Hop`: at most 4, each reached through a `direct-tcpip` channel of the one before, its key checked and its prompts asked in the space; an error names the hop; `route.rs`) and local forwards (`Options::forwards`, `Forward::parse`: a listener on 127.0.0.1 a forward, a channel a connection; one that cannot listen is a line in the space); `tests/route.rs` (4) through two test servers. In the app `Remote` has `jumps` and `forwards`; a saved host's sixth word names its jump host (chained, `· jump` in the menu), its seventh its forwards; Quick connect and Add SSH host choose a jump host, Add SSH host takes forwards; a test in `tests/hosts.rs`. Then (2026-11-17) remote forwards (`R9000:localhost:3000`: `tcpip-forward` asked of the server, each `forwarded-tcpip` channel carried here) and dynamic ones (`D1080`: a SOCKS5 proxy, CONNECT only, its connections out from the server); `Forward` has a `Way`, the engine's `forward.rs`; a test in `tests/route.rs` through the test server. Then (2026-11-15) OneTerm was brought to the layout of its design canvas, in the kit's look (specs/19): the title bar's chip of what shows and its buttons (`chrome.rs`; Done in the settings), the status bar in parts with rules, one count of bells and notices, `lock in Nm` (`Lock::left`) and a 24-hour clock (`statusbar.rs`); the rail's rows of one line but the active one's, marks on discs, a 3 px progress bar, a window icon for remote tabs (the kit's icons `app-window` and `square-terminal`), a dashed New terminal, SSH hosts with an Add button and groups that fold; three spaces or more beside the rail as one on the left and a stack at its right, headers that say where a space is and its state on a disc, cards clear of the dock (`Notices::right`); the dock's header with a More menu and a bordered Expand, a local space's empty state with Show <host>; menu rows with icons, details and dots (`MenuItem::dot`, `MenuItem::detail`), the searched menu 360 wide beside the rail; Quick connect's Auth and Jump host side by side and Open SFTP alongside; the settings in the kit's layout of cards with pages SSH & hosts and SFTP; the lock card; the strip's marks and a Hosts panel when the tabs are in a strip; `bench/oneterm-shots` draws each screen offscreen to compare. Not yet: the OSC 7501 id in a header (the engine keeps only the strongest state), a show-password button, Windows Hello, a footer of the settings' column. Then the settings in a window of their own (`quadra::Windows`: a view opens a window that closes with its own, specs/15; `tests/windows.rs` of `quadra-app`), a click that reaches each space (the ring is the border), no chip in the title bar; a space's menu for the other button (Copy, Paste, Select all, Clear scrollback, Split space, Close space; nothing opens while the program reports the mouse) and the page Mouse & wheel, through the view's `TermSettings` (copy on select, middle-click paste, the wheel's speed, alternate scroll) and `Session::selection_text`, `paste`, `select_all`, `clear_history` (`crates/quadra-term/tests/settings.rs` (3), `apps/oneterm/tests/menus.rs` (3)); a context menu with no items opens nothing; SFTP menus in the dock and both panes of the expanded view (Open or Download or Upload, Rename… and New folder… in a dialog, Copy path, Delete… asked first, Refresh; what fails is a card), the engine's `Files::rename`; tests in `tests/dock.rs`. The terminal's look is a signal (`TermLook`, `TermColors`: family, size, line height, weight, padding, colours over the theme's; `Terminal::look`; `crates/quadra-term/tests/look.rs`): OneTerm's Appearance sets the font family, size, line height and four colours, and every terminal follows without mounting again (a test in `tests/settings.rs`). OSC 52 (the clipboard written always, read only when `TermSettings::clipboard_read`; Security & lock's switch; tests in `crates/quadra-term/tests/settings.rs`). A session's log (`Session::start_log(path, append, stamp)`, `stop_log`, `logging`; the engine's parser keeps the printable text on the feeder's thread; `tests/log.rs`), and in OneTerm Logging (new local shells, new SSH spaces, the folder, append or overwrite; Start or Stop logging in a space's menu; `logs.rs`, a test in `tests/settings.rs`); `quadra_app::time::utc_offset`, and `Rc<AppDirs>` in the context. The SSH keepalive is the engine's `Options::keepalive` and `keepalive_max` (SSH & hosts, Connections), and a tab's title is the program's (a path shortened to its program) or a fixed label (Tabs & spaces). The UI's text size (Appearance, the app's `TextSize::text`), frames a second in the status bar (`Window::frames`, `Handle::frames`), the About page, the rail's width and the rail folded to its icons (`rail/narrow.rs`; the kit's `tooltip` takes words that change), and the dock open or closed for each kind of tab; tests in `tests/tabs.rs` and `crates/quadra-design/tests/overlays.rs`. The gutter (`TermLook::gutter`, `gutter.rs`: each row's time, stamped by the feed, and its line number from the top of the history; four colours; `tests/gutter.rs`), in OneTerm's Appearance. The completion engine is the crate `oneterm-complete` (specs/20: the families, the bundled catalogs, the line at the cursor, history with redaction, prefix then fuzzy matches; 50 tests); its overlay in the terminal (`complete.rs`, `complete/overlay.rs`, `page/complete.rs`: under the caret or flipped above, Tab or Enter accepts, Up and Down choose, Escape closes until the line changes, history recorded redacted at a prompt only, the reference's fifteen settings on the Terminal page; `tests/complete.rs`, 8 tests) reads `Session::command_line()` (`line.rs`: the line after the prompt by OSC 133, or after a prompt sign, the caret's box; `tests/line.rs`) and takes keys by `Terminal::on_key_first`. For the keybindings page: `Keystroke` prints as it parses, and the app's keymap is a signal every window follows (`app_keymap()`; a test in `quadra-app/tests/keymap.rs`); the page (`page/keys.rs`: nine actions, each recorded by the kit's `key_input` with a warning when two share keys and a Reset, stored as `keys.<id>`, the keymap built again on a change; `tests/keys.rs`, 3 tests) rests on `Tree::on_key_first` (a key to the focused node before the keymap) and `quadra_headless::keys::key_input` (recording on focus, Escape gives up, Backspace unbinds, Tab still moves; `crates/quadra-design/tests/keys.rs`, a gallery page). Fallback families (`App::font_fallbacks`, `TextSystem::set_fallbacks`; DirectWrite's fallback builder; a test in `quadra-native-win/tests/dwrite.rs`). SFTP's Edit (`sftp/edit.rs`: downloaded to the app's data folder, opened by the system or a program, `Opener::open_with` in `quadra-shell`, sent back when saved, asked above a size; a test in `tests/dock.rs` with `FakeShell`), the face's weight, Lock now, the log folder chosen in the system's dialog, the log formats shown. Colour themes: 37 palettes in the kit's theme format (`apps/oneterm/themes`, `themes.rs`), chosen over the mode in Appearance. The look's settings of the reference terminal (specs/18, 19): padding a side (`TermLook::padding`, an `Edges`), the cell's width (`TermLook::cell_width`, grown with the text size), minimum contrast (`TermLook::min_contrast`; `contrast.rs`: `contrast_ratio`, `with_contrast`, a memo of pairs; colours named exactly and drawings left), fallback fonts (`TermLook::fallbacks`, set on the app's text system when they change; `TextSystem::fallbacks`, and setting the same list does nothing), and in OneTerm (`prefs/look.rs`, `LookPrefs`) these and the sixteen ANSI colours in the kit's new `color_input` (a swatch, the theme's colour while empty; a gallery page, a test in `crates/quadra-design/tests/input.rs`); tests in `crates/quadra-term/tests/look.rs` (4 more) and `tests/settings.rs`. Ligatures (`TermLook::ligatures`: a token of cells shaped as one word through the text system's cache, its glyphs put back by cluster on their cells, `shape.rs`) and semantic highlighting (`TermLook::highlighting`, `Highlight`, `TermColors::highlights`, else the palette's ANSI colour of each class; `highlight.rs` and `highlight/`: the reference's classes and matchers written by hand, roles of lines from OSC 133 zones or a prompt's look, the sign tinted by the last status, classes kept by row id and stamp so a paint scans only the lines that changed); in OneTerm's Appearance, both on (`terminal.ligatures`, `terminal.semantic_highlighting`); `crates/quadra-term/tests/ligatures.rs` (2), `tests/highlight.rs` (5), unit tests of the scanner (7) and of placing glyphs (2), a test in `tests/settings.rs`. Not yet: a real ligature seen in a window (a manual check with a coding font). Every control of the kit takes a reader's name (`.label(..)`, words that may change; `TextInput::label` in `quadra-view`; tests in `crates/quadra-design/tests/input.rs`, `button.rs`, `choice.rs`, `dates.rs`), and OneTerm's fields are named by their rows. (2026-11-16) The shell starts once the view has sized the grid (`Session::when_sized`; a test in `crates/quadra-term/tests/session.rs`): a program that reads its size once read 80 by 24 before. A number input never writes back what it shows (1.5 shown with no digits stayed 1.5 only on screen: every setting of a number with a fraction was rounded when its page was built; `crates/quadra-design/tests/forms.rs`), and a step shows its digits. OneTerm's settings page is built only while it is open. The font family is chosen from the machine's in a select with a search; the default shell can be a program typed with its arguments; the Keybindings page has fifteen actions (Settings, the SFTP dock, Select all, Clear, Duplicate, Quit besides) and Reset all (`tests/keys.rs`, `tests/settings.rs`). The fire demo runs at 591 fps at 143 by 39 with OneTerm's defaults (docs/benchmarks.md). Links open on Ctrl+click (specs/18, "Input"; specs/19, "Spaces"): `Terminal::on_link(|link: TermLink| ..)` is called with an OSC 8 hyperlink (its target, and its visible text when that differs) or a URL of the text (`quadra_text::find_urls` over the logical line, so one that wraps is one link) found in the snapshot the view drew (`link.rs`); the press then selects nothing and neither it nor its release reaches a program that reports the mouse; while Ctrl is held the link under the pointer is underlined under a hand (looked for only while Ctrl is held, again only when the cell or Ctrl changed); `Session::link_pressed()` is the link under the other button's press. In OneTerm (`links.rs`) a pure policy: https opens through the shell's `Opener`, http, a port not the scheme's or visible text naming another host asks in a dialog, any other scheme, controls, a user or password, or over 256 KiB is refused with a card; the space's menu has Open link and Copy link on a link. Tests: `crates/quadra-term/tests/links.rs` (6), `apps/oneterm/tests/links.rs` (2), unit tests of the policy (2). Find in the terminal (specs/18 and 19, "Find"): `Session::find(&FindQuery { text, case, word })` through the engine's literal matcher over the history and the screen, `find_next(back)` wrapping, `found()` as `(current, count)`, the view scrolled to a match out of view, the matches painted in the theme's find colours, found again after output (at most every 150 ms) and after a resize; in OneTerm a find bar over the focused space (`find.rs`; `Ctrl+F`, the sixteenth binding `find`; Enter and F3 next, with Shift the one before, Escape gives the focus back); `crates/quadra-term/tests/find.rs` (4), `apps/oneterm/tests/find.rs` (1). Not yet: regular expressions (the engine's need a dependency). SFTP Properties… in an entry's menu, in the dock and the expanded view (`sftp/properties.rs`: the entry asked again by the engine's `Files::stat`, a `Stat` with the mode, uid and gid named from the server's `/etc/passwd` and `/etc/group` where they can be read, and times; a dialog with a description list: name, type, size, times, permissions as `-rw-r--r-- (0644)`, owner and group, the path to copy; what fails is a card); a test in `oneterm-ssh/tests/files.rs` and one in `tests/dock.rs`. As in the reference it shows and does not change the mode. Splits and input channels (2026-11-16, specs/19, "Spaces"): a tab's spaces are the leaves of the kit's pane tree (`Tab::layout`, `quadra_design::work::panes`; seams drag, `Pane::even` shares a run of splits along one axis evenly, after a split and a close), split to a side by `Ctrl+Shift+Right/Left/Up/Down` (`Command::SplitTo(Side)`, the Keybindings page, the space's menu: Split right, left, up, down); `Ctrl+D` (Split space) works as before: a tab arranges its spaces by itself (`Tab::auto`, `split::arranged`: a row of equal columns, beside the rail from three on the first on the left and the others stacked), built again on a split, a close and a move between the rail and the strip, until a split to a side or a dragged seam makes the layout the user's, where Split space splits to the right (back at one space it arranges itself again); a press in a space makes it the tab's focused one. Input channels A to E (`Space::channel`, `channels.rs`): the space's menu joins, leaves and closes one, or the tab's all; a member's header has the channel's chip (`token::SERIES`) and its focus ring the channel's colour; what is typed goes to the other members through `Session::on_input` and `Session::deliver` (`Session::send` for what the app types, an accepted completion) (`quadra-term`, `src/input.rs`: keys, text, pastes bracketed as each program asked; never the mouse or replies); an ended program leaves. Tests: `apps/oneterm/tests/splits.rs` (5), `tests/channels.rs` (3), `crates/quadra-term/tests/broadcast.rs` (2), a unit test of `Pane::even`.

## R1

| Crate | Status | Evidence |
|---|---|---|
| `quadra-reactive` | Done (2026-11-11) | 31 tests covering every item of specs/03 "Tests"; no `unsafe`; 93 ns per binding in the `grid` shape (docs/benchmarks.md) |
| `quadra-layout` | Done (2026-11-11) | 39 tests: the 16 flex and 5 grid cases of the V suite, the costs of a change as counters, 1,500 random changes each checked against a layout from scratch; 0.82 µs for a change in a list of 10,000 fixed-height rows (docs/benchmarks.md) |
| `quadra-tree` | Passes done (2026-11-11): nodes, widgets, update, layout, paint into layers, compose; text painted from glyph tiles (`Fonts`). Pointer input: `hit_test`, `pointer`, handlers on nodes (`on_pointer`), hover and press state; a node's style as its own values plus what each state changes (`Patch`, `State`, `set_variant`, the style pass). Since 2026-11-15 a new band of a long page skips what lies outside it and emitted nothing (`FrameStats::nodes_walked`; specs/07). | 13 tests: scrolling and moving a layer paint nothing and upload nothing, the counts of a frame for each kind of change, one change among 10,000 nodes costs one node, and 1,000 random changes each checked against a paint from scratch |
| `quadra-text` with the fake backend | Paragraphs and rasterization through the backend done (2026-11-11); numbers composed from digit glyphs; the edit buffer of a text input (`EditBuffer`: selection as anchor and head, movement by graphemes and words, undo with typing one step a word); the URLs of a line of plain text (`find_urls`, `url_openable`: the one detector, and the gate for links in content that is not the user's). A change of how the system smooths fonts rasterizes glyphs again (`TextBackend::system_changed`). OpenType features per family (`Feature`, `App::font_features`). Bidirectional text, for display: levels from the backend (`AnalyzeBidi`), words shaped in visual order, the words of each line in visual order (`bidi.rs`, `paragraph/visual.rs`); caret and editing in right-to-left text stay After-v1. Large texts are the editor's (`quadra-code`'s `Buffer`: pieces summed in a Fenwick tree) | 12 tests: wrapping, hanging spaces, breaks between graphemes, a new width breaks lines without shaping, a changed text shapes only new words, the cache bound, caret stops; `crates/quadra-text/tests/normalize.rs` (3: marks compose onto their letter in either order, a search without accents, a mark typed after a letter joins it), three unit tests of the URLs (punctuation and brackets, the schemes that open, ranges in bytes) |
| `quadra-render` with the null backend | Layers, display lists and atlases done (2026-11-11); an image of any size is pieces of the colour atlas, so nothing needs a texture of its own | 12 tests: tiles packed in shelves, a tile in use is never evicted, the budget, in-place overwrites upload only their range, rebuilds copy clean ranges, child layers split runs, scrolling uploads nothing, culling, removed layers, device loss; the null renderer's copy is checked against the scene |
| `quadra-platform` | Started (2026-11-11): the `Platform`, window and handler traits, events, keys, and the headless platform (a clock the test moves, queued events, stepping), the clipboard as plain text (in memory on the headless platform). Composition events and the window's text-input state, its cursor, and the semantic events and updates of assistive technology. Where a window is (`Placement`, `WindowOptions::remember`, `placement`). The decoder of images (`ImageDecoder`, `Platform::image_decoder`, `admit` for the limits of a header, `FlatImages` for tests). The system's appearance (`Appearance`, `PlatformEvent::Appearance`, the headless `set_appearance`) and drops from other programs (`Dropped`, `DragEnter`, `DragOver`, `DragLeave`, `Drop`). Not yet: displays, added with what first uses them | a unit test of the limits and of flat images, and 6 tests: timers fire in order at their time on the virtual clock, a step delivers events then the frames asked for, a waker from another thread wakes the loop once, a dropped window is closed |
| `quadra-app` | Started (2026-11-11): `App` owns the platform, the renderer, the fonts and the windows; it draws a frame for a window when its tree has something to do, follows resizes, ends the loop when the last window closes, and makes a lost device again. It runs the tasks of its views after every entry point (11). Since R5: `App::dirs(AppDirs::for_app(..))` gives it folders, without which it writes nothing; settings (`app.settings().value(key, default)`: a signal a key, written 500 ms after the last change through a file renamed over the old one, a broken file moved aside, `migrate`); the log (`quadra_app::log`: a ring of 500 lines, a file a day, errors to the standard error stream when there is no file); a report of a panic in `crashes/`, offered once by `last_crash`; the command line (`args`), `quit_on_last_window`; `quadra_app::time` (`Date`, `Time`, `DateTime` from Unix time at an offset; the kit's `Date` is this one); the user's time zone (`Platform::utc_offset`, `time::local_now`); the app's language (`I18n` with tables of strings a language, `App::i18n`, `t`, `t_with`, `language()` for what runs again, plurals by language; `Locale` formats numbers, sizes, dates, times and times relative to now in Vietnamese and English, and names months and days; the kit's calendar follows it); window state (`WindowOptions::remember(key)`, `Placement`, kept as `window.<key>` when the window closes; Win32 by the window's placement, put back only on a display); pages (`Router`, `router(first)`, `.view(..)`; `GoBack`, `GoForward` on Alt-Left and Alt-Right and the mouse's buttons); closing (`.on_close(|| bool)`, `Tree::may_close`, `window().close_now()`) | 4 tests on the headless platform and the null renderer: a frame is drawn when something changed and not otherwise, windows share signals, closing frees what the renderer kept, a lost device is recovered with every glyph rasterized again; `crates/quadra-app/tests/frame.rs` and `crates/quadra-native-win/tests/frame.rs` (a window with its own frame: what a point is, the asks of a view; on a real hidden window, the content over the caption and the answers the OS gets) |
| `quadra-shell` | Started (2026-11-14, specs/16): the traits `Dialogs` (open, save, a folder, a message) and `Opener` (open, reveal), the Windows ones (`WinShell`: the common file dialogs, the message box, the shell), `FakeShell` for tests, `install(app)`, `dialogs()`, `opener()`; `App::provide`. The tray (`Tray`, `TrayIcon`, `TrayEvent` through an inbox, notifications; `WinTray` with a hidden window and the OS's menu, added again when the taskbar restarts), one instance (`single_instance(id, args)`: a named mutex and a hidden window, a later start's `Launch` by `WM_COPYDATA`), reports of crashes that are no panic (`crash_reports(folder)`: a minidump and the log's last lines); another process read from outside for the comparison bench's runner (`bench/runner`, 2026-11-16: `process_cycles`, `cycles_per_second`, `private_memory`, `window_shown`) | `crates/quadra-shell/tests/fake.rs` (3: a window not saved asks before it closes; the fake's answers; the tray's events), `tests/instance.rs` (a second start in the same process hands over its arguments), `tests/measure.rs` (the test binary run again as a child that spins and holds memory), a minidump of the process in `src/crash.rs`. The Windows dialogs, the tray and a real crash are manual checks |
| `quadra` (the facade) | Started (2026-11-11): gathers what an app names and builds an `App` on the Win32 platform, the D3D11 renderer and DirectWrite (`quadra::app()`) | The comparison bench's four scenes (`bench/compare`) run in real windows and show what the V build's show; first numbers in docs/benchmarks.md |
| `quadra-a11y` | Started (2026-11-11): roles, what a node says of itself, semantic actions, tree updates, the `Mirror` that applies them as an OS adapter does, the tree as text, relations (named by, described by, controls; `.labelled_by`, `.described_by`, `.controls`) | Used by the tree and the harness; see `crates/quadra-view/tests/semantics.rs` |
| `quadra-async` | Done (2026-11-12): timers (`timeout`, `interval`) on the platform's clock, owned by the scope they are set in; tasks (`spawn`, `Task`: futures on the thread of the windows, run once each after every entry point, cancelled by a drop or by their scope, awaited for their value; the `Executor` on the standard library's `Wake`); the pool (`background`, `background_with` and its `CancelToken`: the cores of the machine, four at most; a panic is the error of the future); inboxes (`inbox`, `inbox_with(limit, Full::Block or Drop)`, `Sender::push`, `extend`, `finish`; a burst is one delivery, into the vector delivered before); `sleep`, `debounce`, `throttle`. A scope pays one cleanup for its timers and one for its tasks | `tests/timers.rs` (4, in virtual time: timers fire in order, end with their scope or when cancelled, a callback may cancel its own timer and set others, and a debounced signal waits for a pause while a throttled one takes the first and then the latest at each step), `tests/tasks.rs` (3: a task starts between events, gives its value to who awaits it, is dropped where it waits by its handle or its scope, and ends when it disposes its own scope; work on the pool comes back as a value, a panic as an error, sixteen pieces of work never more at once than the pool has threads, and work whose future is dropped is told; a hundred pushes from another thread are one wake and one delivery, the code comes after the last data, an inbox at its limit drops or makes its producer wait, and a sender is told its scope ended), and in `crates/quadra-app/tests/app.rs` a task of a window's view that shows what the pool made |
| `quadra-testing` | Started (2026-11-11): a headless window that checks every frame against a paint from scratch, with a clock the test moves (timers fire on `advance`), pointer, wheel, key and text input, `mount`, which builds a view where it finds timers and a clipboard in memory, and queries over the semantic tree (`get_by_role`, `get_by_text`, `get_by_test_id`) whose result is driven by semantic actions; `fuzz` (`Rng`, `mutate`, `run`: what the seeded test of a parser is made of). Finding and driving controls through semantics waits for input and semantics | Used by the view tests |
| `quadra-vector` | Done (2026-11-11): paths with arcs, flattening, SVG path data, the SVG subset of icons (shapes, groups, fill and stroke), coverage by exact areas and round strokes, masks and RGBA. Ported from the V module with its tests | `crates/quadra-vector/tests/vector.rs` (5, one of them 400 rounds of changed and made-up drawings that must read and render) |
| `quadra-design` | Started (2026-11-11): the roles of the design language and its themes, read from JSON files, and one of the system's colours under its high contrast (`high_contrast`, `system_theme`, `KitTheme::with_accent`; since 2026-11-15); the bundled faces, the numbers of the design language, `install` and `root`; the kit's actions and key bindings; the button; icons (87 Lucide outlines, drawn as masks from the atlas in the text colour); the text input (frame, placeholder, password, invalid, disabled); the checkbox, the switch, the radio group and the segmented control; popups (focus in and back, Escape, a press outside), the menu and the select; the tooltip and the dialog; shadows by elevation; tabs and the list; headings, badges, tags, keys, avatars, cards, dividers; the accordion and the split; the title bar, the toolbar, the status bar, the breadcrumb, status marks and the sidebar; the slider, the number input and its stepper, fields with rules, the text area; the progress bar, the spinner and the skeleton; menus with headings, submenus and scrolling, the context menu; notifications and toasts; the command palette; the table (its columns scroll sideways when they are wider than it; pinned columns, `.pinned(n)`, cut to their box and scrolled by a bar along the bottom too, and hidden ones, `.hidden(..)`); the tree; the range slider and the combobox; the calendar, the date picker and the time input; the hover card and the drawer; the workbench (`quadra_design::work`): the activity bar, the panel header, the tab strip (since 2026-11-14 a tab's own menu, renaming, titles cut with an ellipsis: `text(..).ellipsis()`), the pane tree (plain data, shown flat: a leaf keeps its nodes while the tree changes around it) and docking (tabs torn off their strip, dropped on a strip, a pane or an edge of one); settings (pages in a column, rows of a title and a control, a search, in rules or in cards); live data in the table (`.keys(..)`: what is chosen stays with its rows through other data and another order, a cell that changed is tinted and fades, composed only); the colours of code as roles (`token::SYNTAX_*`, a theme file's `syntax`); a tree's labels in a colour, a detail after them, a denser tree; the sixteen colours of text as roles (`token::ANSI`); the diff (`diff(old, new).mode(..).context(n)`: the shortest script of lines removed and added, `diff_lines`, searched by distance up to 1 500 lines apart; one column or the old text beside the new, in the rows of a `uniform_list`; the part of an edited line that changed tinted more, with `span(..).background(..)`, new in rich text; lines far from a change folded into a row that opens; the tints are the roles `token::DIFF_ADD` and `DIFF_REMOVE`, a theme file's `editor.diff_add` and `diff_remove`). In a release build two texts of 100 000 lines are compared in 2 ms with three lines changed and in 13 ms with 700. Since 2026-11-14 the diff scrolls sideways (the wheel turned so or with Shift), selects and copies rows as a unified diff (Ctrl-C), goes to the next change by F7 and back by Shift-F7, and compares long texts on the pool. The lines take the colours of code (`.colours(quadra_code::line_colours(path))`). The lines of a run of changes are paired by how alike they are. Long lines wrap (`.wrap()`). The words that changed are marked in several places of a line (`diff/words.rs`) | `crates/quadra-design/tests/theme.rs` (3), `tests/root.rs`, `tests/input.rs`, `tests/settings.rs` (3: pages chosen by the pointer, a reader and the keys, groups shown under them, a search whatever the case and the accents; a chosen group at the top of its page; the cards), `tests/choice.rs` (2), `tests/popup.rs` (3), `tests/panes.rs` (2: leaves and seams placed, a seam dragged, kept off the ends, made even; leaves kept through a split, a fold and a removal), `tests/dock.rs` (3: what a drop asks of the groups and the tree; the zones of a pane; a tab torn off and dropped on a strip and on an edge), three unit tests of the pane tree, `tests/work.rs` (5: an activity bar with its counts and its tooltip at the right; a panel header; a tab chosen by a press, moved by a drag, closed by the middle button and its cross; the list of the tabs; a tab renamed in place, its own menu, a long title cut), `tests/cards.rs` (2: a hover card after a rest, kept by the pointer on it, gone after the grace; a drawer along each edge, given up by its cross, Escape and a press outside), `tests/small.rs` (3: a busy button and a checkbox that is neither; an input emptied and a tooltip that says more; a question and a bar of menus), `tests/dates.rs` (4: days of a month and of the week; a calendar; a date picker; the parts of a time), `tests/tree.rs` (2: rows open and close what they hold by the pointer; the arrows move, open and close), `tests/table.rs` (4: cells as their columns say, sorted by a click on a title; rows selected by the pointer and the keys, and copied; a column made wider; a pinned column that stays while the others scroll, and hidden ones neither shown nor copied), `tests/table_pinned.rs` (1: what scrolls is cut to its box, a drag of the bar scrolls it), `tests/live_table.rs` (2: what is chosen stays with its rows when rows go, come the other way round or are sorted again, and a row that goes is not selected; a cell that says something else is tinted over its own cell, an age and a new row are not, the tint fades by the clock with nothing laid out or painted, goes on through another change, and the frames stop when all have faded), `tests/palette.rs` (2: a query finds commands whatever the case and the accents, the arrows and Enter; how letters match and score), `tests/notices.rs` (2: cards stacked in the corner until closed or answered; a toast and a card that lasts go by themselves), `tests/menus.rs` (3: a menu at the pointer for the other button; a row that opens a menu beside itself, by the pointer and the arrows; a long menu that scrolls under a heading), `tests/progress.rs` (2: a bar filled as far as its part; what moves is composed, not laid out or painted), `tests/forms.rs` (5: a slider by the pointer, the keys and a reader; a number typed or stepped; a field that says what is wrong; the knobs of a range slider; a combobox), `tests/bars.rs` (3: the heights of the bars and the links of a breadcrumb; the marks of a status; a sidebar by the pointer and the keys, folded), `tests/titlebar.rs` (2: what each point of the bar is, what its buttons ask), `tests/parts.rs` (3: sizes and names of what shows, key labels; an accordion by the pointer and the keyboard; a split dragged, kept over its least, made even, stacked), `tests/lists.rs` (2: tabs by a click or the arrows; a row of a thousand selected by a click or the keys, kept in view, activated; the empty list), `tests/overlays.rs` (2: a tooltip after half a second of rest, gone on leave or press; a dialog answered or given up), `tests/button.rs` (3: an icon before the label or alone; sizes and the colours of variants and states; click, Enter, Space, disabled, toggle); the example `themes` shows both themes in a real window; seven unit tests of the diff (the script: the shortest, removals first, every line once, texts too far apart; the part of a line that changed; rows as one column and as pairs; folds) and `tests/diff.rs` (5: the lines of both layouts by their names and places, the fill of what changed, another text; a fold opened by a click and by Enter, closed by other texts; 100 000 lines compared on the pool cost what the rows that show cost; rows selected, copied and found by F7; the wheel sideways); the gallery is `examples/gallery` (71 of the reference pages so far, and the diff, math, diagrams and images, which have none: overview, actions, forms, display, layout, workbench, navigation, overlays, data, charts, stress), drawn to pictures by `tools/shots.sh`, the pages that open something a second time with it open, and compared with the V screenshots (73 pictures with a reference, each within 2.7 % of it in the dark theme but the variable list, 4.7 %, whose lines of text are whole pixels high). Settings go back to their defaults (2026-11-16, specs/13 and 17): `.row(..).resets(value, default)` or `.resets_with(changed, reset)` (`quadra_headless::settings::Reset`, `reset_all`) shows a reset left of a row's control while it differs (`Reset <title>`) and a page's "Reset all" while one of the rows a search keeps does, which resets those; OneTerm wires every setting of its pages but Keybindings, to the defaults of `Prefs::defaults()` (the same keys read from settings in memory). Tests: `crates/quadra-design/tests/settings.rs` (1, both layouts), `apps/oneterm/tests/settings_reset.rs` (1). The sidebar's search (2026-11-19, `.search(query)`: the places whose names hold it, their groups' titles, `No results`, Enter chooses the first, the arrows among the places kept; the gallery's sidebar has it): a test in `tests/bars.rs`. A tooltip's arrow (2026-11-19: a triangle 12 by 6 on the edge toward its view, at the view's middle, on the side shown, a flip seen; the overlays are placed before `after_layout` runs): 2 tests in `tests/overlays.rs` |
| `quadra-data` | Done (2026-11-12; since 2026-11-14 the log view selects and copies lines, scrolls sideways, and a log holds a limit of lines): `Value` (what JSON and YAML say: keys in their order, numbers as written; `get`, `at(path)`, `summary`), `parse_json` and `parse_yaml` (this crate's own readers; the YAML manifests are written in; neither goes deeper than 128 levels), `value_view(value)` (the kit's tree in the colours of code, the first two levels open, a bar with the path of the selected row and `Copy path`); `Log` (lines packed in one buffer with a time each; `append`, `push`, `clear`, `set_filter`) and `log_view(&log)` (a `uniform_list` of rows in the mono face, each line in the colours its escape sequences ask for, which are the theme's; it follows the newest line until the user scrolls away, as a place past the end that the list keeps at its end; a filter by a text or a pattern of `regex-lite`, hiding or dimming). In a release build with the fake text backend, a frame after 160 lines come to a million takes 0.7 ms, and a text filter over the million 87 ms. Since then: a line selected and copied, scrolling sideways, a limit on the lines held (`Log::limit`) | two unit tests of the JSON reader (order, spelling, escapes; errors with their line, a thousand brackets), `tests/yaml.rs` (3: a manifest; the kinds of scalars and the line of what is wrong; what no one would write is read to an end), `tests/value_view.rs` (2: rows two levels open that say what they hold; a row selected, opened, its path copied by the key and the button, and what is open kept through another value), two unit tests of the escape sequences (pens and runs; a line without any, a broken one), three of the log (lines packed and read back; a filter that hides or dims by a text or a pattern, lines tested as they come, a pattern that is not one; the oldest lines past a limit go), `tests/log_view.rs` (4: lines selected and copied, a long one scrolled sideways; the newest line stays in view until the wheel goes up, lines that come then move nothing, the end or the button takes it up again, and no row is taller for a long line; the filter of the bar; a million lines cost what the rows that show cost); the gallery's pages `value-viewer` and `log-view` differ from the V pictures in 2.16 % and 1.45 % of the pixels |
| `quadra-viz` | Done (2026-11-12): the plot (`Plot`: a line a device pixel column at a time, its cover of each pixel worked out by the shader, an area under it; values that change overwrite its instances in place), `sparkline(values)`, `line_chart([Series::new(..)])` (views around a plot a series: a shared scale rounded up, grid lines, times, a hairline and a box of the values at the sample nearest the pointer), `bar_chart(bars)` (views alone, rows kept by their label); colours are the theme's eight series roles (`token::SERIES`). Since 2026-11-14: a legend (`.legend()`), upright bars (`.upright(height)`), colours past eight (fainter), the peaks between two edges of a column, and a reader told the least, the most and the last value of each line | three unit tests of the geometry, `crates/quadra-viz/tests/charts.rs` (4: a sparkline is a line and an area a pixel column at 150 %, its places fractions of a pixel, and new values paint one node in place; a line chart says its scale and times, the values under the pointer on the side of the hairline that has room, costs no frame for a move that stays at a sample, and a series that changes moves the scale; a bar is its share of the rounded maximum, 2 px at least, its row kept by its label; upright bars on one line, a legend of twelve series), one in `crates/quadra-native-win/tests/pixels.rs` (a chart line's column has all its ink, its middle where the line's is to a twentieth of a pixel, and shades at its ends), one more in `crates/quadra-design/tests/theme.rs` (the series roles, and the accent where a file lists fewer); the gallery's pages `sparkline`, `line-chart` (and with the pointer on it) and `bar-chart`: 0.92 %, 1.97 % and 0.70 % of the pixels differ from the V pictures in the dark theme, 0.95 %, 2.03 % and 0.72 % in the light one; looked at ten times larger at 100 % and at 150 %, no slope shows a step |
| `quadra-code` | Started (2026-11-12): the tokenizers. `Tokenizer::tokenize(line, state, out)` reads a line and gives the state the next starts in (what is open at its end: a comment and how deep, a string and its quote, a fence); `TokenKind` is what a theme's `syntax` colours, `role(kind)` its role; `tokenizer_for(name)` finds one by a file's name or by what follows a fence. V, YAML and Markdown have code of their own; 27 more are rows of one table (`LangSpec`) read by one scanner, markup and diffs modes of it. Whatever a line holds and whatever state it is given, tokens are in order, none over another, between characters, in the time the line's length takes; nothing is allocated but the caller's vector. Two faults of the V scanner are gone: a colon with no value in a YAML flow hung it, and a lifetime took the character after it. The buffer is in (`Buffer`: pieces of 4 KB in a row, their bytes and line breaks summed in a Fenwick tree (`buffer/counts.rs`, since 2026-11-14), so a change and a search cost the log of the count of pieces; `insert`, `delete`, `line_start`, `line_end`, `line_of`, `slice`, `line`, CR LF kept for the file; ten megabytes open in 6 ms and two hundred in 120 ms, a letter typed in 0.8 to 1.2 µs at either size, a line read in under 1 µs, release). The document is in (`Doc`: cursors, edits made as one change, undo in groups that typing joins, motion by grapheme, word, line and column, and the commands: new line with indent, deleting, tab, lines indented, deleted, duplicated, moved and commented, selecting, cursors above and below, the next occurrence, the matching bracket, copy, cut and paste); `quadra-text` counts cells for it (`display_width`). The highlighter (`Highlighter`: end states of lines, read again from the first changed line as far as a view asks) and finding (`Query`, `next_match`, `Doc::replace`, `replace_all`; `regex-lite`), `tests/highlight.rs`, `tests/find.rs`; the editor (`editor(text).path(name)`: `EditorWidget` in a scroll view, painting the lines that show; `EditorCommand` in the context `editor`, `bind_keys`; typing, composition, the clipboard of the context, the pointer, the caret kept in view), `tests/editor.rs`. colours through the trait `Highlight`, so a parser of grammars can be another; the gallery's page `editor` (1.1 % of the pixels differ from the reference in both themes). the find bar (`editor/find_bar.rs`: Ctrl-F, Ctrl-H, the switches, n of m, replace and all; the shots draw it as `editor-open.png`; since 2026-11-15 a change searches again only its lines when the query is text, `Query::rematch`: 230 µs a letter on 400,000 lines where the whole text took 306 ms). A keystroke at the middle of 100,000 lines takes a frame of 0.39 ms (docs/benchmarks.md). Soft wrap (`.wrap(true)`, `editor/rows.rs`; since 2026-11-15 only the lines a change was are wrapped again, by the document's notes of them, `LineChange`: 24 µs a letter on 100,000 lines where all of them took 66 ms), links (`.on_link`), the marks of a diff (`.base(text)`, `marks`; since 2026-11-15 by hashes of lines, those a change was hashed again, `diff_ids`: 1.4 ms a letter and its frame on 100,000 lines where comparing the whole text took 16 ms; `tests/marks.rs`); a reader hears of the caret that moves, and a text of more than 256 KB is told as the 200 lines on each side of the cursor (`tests/reader.rs`). The line at the top and scrolling to a line: `.top_line(..)` (`tests/top.rs`) | `tests/doc.rs` (7: the cases of the V suite and more), `tests/doc_random.rs` (60 runs of 120 commands at random with every cursor checked after each and all of it undone and done again), `tests/buffer.rs` (3: 4 000 changes at random beside a plain string with every line compared; CR LF and the empty text; ten megabytes that know their lines, and a megabyte with no line break), `tests/tokens.rs` (7: the cases of the V suite for V, YAML, Markdown and the table's languages, the names a tokenizer is found by), `tests/fuzz.rs` (2: 1 500 texts of the corpus changed, and runs of what never closes, through each of the 30 tokenizers from states they give and states none gives, none over 2 s; a line of 40 000 bytes of each of 20 things that never close, each tokenizer under half a second) |
| `quadra-md` | Started (2026-11-12): the parser, plain functions over text, and the view. `parse(source)` gives the blocks with their source lines (`Kind`: paragraph, heading, item of a list with its number, depth and task, fenced code with its language, quote with its depth, rule, table with its alignment, math, front matter; comments show nothing), `parse_inline(text)` the spans (bold, italic, struck, code, math, links, images, autolinks, bare URLs by the detector of `quadra-text`, escapes), `outline`, `anchor`, `plain`, `block_at`, `words`. A text costs what its length costs: a search that failed is not made again, brackets are matched once, links in the labels of links stop at 16 deep. `markdown(source).on_link(..)` shows a document: the blocks are a keyed list, known by what they say, so a change builds again the block that changed and no other; headings, items with bullets, numbers and the boxes of tasks, quotes, rules, code in a box that scrolls sideways (`scroll_x`, new in `quadra-view`) and is coloured by the tokenizer its language names (`quadra-code`; a URL in it is a link whatever tokens it is in), tables as a grid whose columns share the width by what they hold (measured at mount); what is struck has a line through it (`span(..).strike()`) and links wait for Ctrl (`rich_text(..).ctrl_links()`), both new in rich text. `tex::parse(source)` reads a formula (the subset of behaviour/21): its nodes (atoms with their classes, groups, fractions, roots, scripts, operators with their limits, delimiters, accents, environments, stacks) and the first fault, after which it reads on; letters of an alphabet are their characters in Unicode; no deeper than 256. `tex::Lay` lays the nodes out as boxes in the way of TeX (styles, spaces by classes, fractions on the axis, scripts and limits, roots, delimiters that grow as glyphs then as strokes, environments), by the ink of glyphs (`TextSystem::ink`, new) in Cambria Math; a box holds its parts as a tree, so nesting costs nothing more. `formula(source).display()` is the view: the widget lays out once a size and paints glyphs (`quadra_tree::paint_glyph`, new: what paragraphs paint with), rules on whole pixels and strokes as masks of the atlas. A display formula of a document is in the middle of its own line; one with a fault shows its source and what is wrong. Math in a line (`$..$`) is a box of its paragraph, `formula(source).span()`: rich text takes what is no text (`inline(what)` in `quadra-view`, the trait `InlineBox` in `quadra-tree`, a run that is a box in `quadra-text`: `Shaping::advance`), in the word around it, on the baseline, said in its place to a reader. Diagrams are drawn (`quadra_md::mermaid::diagram(source)`): sequence diagrams (participants and actors, messages, notes, frames with their parts, numbers; columns as far apart as what is between them) and flowcharts, read by hand with what one may hold checked as it is read, laid out in layers (ranks, order by sweeps and swaps with subgraphs kept together, medians), and painted from a list of boxes, lines, polygons, text and heads with roles for colours; lines are strokes of the renderer (`quadra_render::stroke`, a segment an instance, shaded by distance), which the strokes of formulas are now too, so neither has a ceiling of the atlas. A paragraph of images shows them (`markdown(source).base(folder)`; what cannot be shown is its words and its path). A link says where it goes after a rest on it; a table wider than the page scrolls sideways. Since then: a line made higher by a box higher than it (`Shaping::reach`), an image in a line of words, one selection over the blocks (`view/select.rs`), where each block is and the blocks that show (`view/doc.rs`) | `tests/parse.rs` (7, the cases of the V suite and: what is never closed, links nested past the limit, letters that are not ASCII, the outline), `tests/fuzz.rs` (2: 3 000 documents of the V corpus changed, and the worst of their kind, none over 2 s; a megabyte of each thing that is never closed, read in the time its length takes), `tests/view.rs` (3: what a reader is told, the cells of a column in line, a link that opens with Ctrl only; a block keeps its nodes while its source does, through a change before it and a block more at the start; 200 documents changed and the worst of their kind mounted and changed, none failing); one test in `view/code.rs` (code cut at its tokens and at its URLs); `tex/tests.rs` (7: the nodes of what is written, the rows of an environment, 13 faults by their words, every name of the tables read and laid out, no name twice, the boxes of fractions, scripts, roots, delimiters, limits and matrices against a letter, the parts of a box where they are) and a third test in `tests/fuzz.rs` (1 500 formulas of the V corpus changed, matrices without end and what opens 20 000 times and never closes: a fault, no deeper than the limit, none over 2 s; each laid out in display and in a line, every size and place a number); a fourth test in `tests/view.rs` (a formula of a document is glyphs in the middle of the page and its source to a reader, one with a fault says what is wrong, one in a line is in the text); for diagrams, 4 tests in `src/mermaid/seq/tests.rs` (the cases of the V suite: participants and messages, notes and frames, frames never closed and faults with their lines and the limits, columns and frames in a layout), 8 tests in `src/mermaid/tests.rs` (the cases of the V suite: shapes, links, chains and groups, subgraphs, faults with their lines and the limits, no two nodes over each other and links the way of the chart, links that end on outlines, the four ways) and the seeded one in `src/mermaid/fuzz.rs` (1 000 sources of the V corpus changed and the worst of their kind, read and laid out, none over 2 s; a megabyte of dashes and a megabyte that never closes in under 2 s), a sixth test in `tests/view.rs` (an image wider than the page is as wide as the page in its own shape, one that is not there and one that is elsewhere are their words, one among words is its words), a fifth test in `tests/view.rs` (a diagram of a document is a picture in the middle named by its nodes, drawn with strokes; one that is not drawn is its source and why), and in `quadra-native-win/tests/pixels.rs` a stroke that is as wide as it says with its joints covered once; for the boxes of lines, a test in `quadra-text/tests/paragraph.rs` (a box is as wide as it says, has no glyph, and wraps with its word) and one in `quadra-view/tests/rich.rs` (a box is painted at the end of the words before it, stands where the letters do, in the colour of its span, and is said in its place); the gallery's page `math` (no reference: looked at in both themes and at 150 %); the gallery's page `markdown` differs from the V picture in 1.80 % of the pixels (1.83 % in the light theme) |
| `quadra-view` (views, enough for the four programs) | Started (2026-11-11): boxes and text with constant or reactive properties, `each`, `show`, `dynamic`, `scroll`, `uniform_list`, frame callbacks (`each_frame`), `mount_with` (a window's view, built where it can reach its tree), `on_click` and `on_pointer`. The image view (`image(source)`: a path, a file built in, or bytes; read and decoded on the pool at the size it shows at, painted from pieces of the colour atlas by the widget `Picture`; `Images` shares what was decoded and keeps what a view that comes back needs). Since 2026-11-14 a photograph is measured and shown as its orientation tag turns it (`Decoded::oriented`; the native crate's `tests/image.rs` decodes a JPEG tagged to be turned). An image that moves shows its frames in turn (`ImageDecoder::frames`; WIC makes a GIF's frames whole in `quadra-native-win/src/image/moving.rs`, tested on a GIF of three frames, one drawn on a part of the screen; each frame is an image of its own, so a change of frame uploads its pieces to the colour atlas again). | `tests/image.rs` (4: its size is its file's, follows one side given, is no wider than the room, and its pixels are those of the scale; a box of another shape fitted, covered and cut, or filled; an image of two pieces that meet, pieces far from the window not uploaded, and three files that cannot show with their reasons; two views of one file share a picture and its tile, and one that comes back shows in the frame it is mounted in) and 18 tests: the structural budgets of specs/14 that can be met so far (7 rows of its table, in a window with 10,000 other nodes), 100,000 rows mount 32, scrolling within them paints nothing, a row that leaves is given to the row that enters; a component runs once, a change runs one binding, a keyed list keeps its nodes, a part's state goes with it; the three scenes measured headless (docs/benchmarks.md) |
| `quadra-native-win` (Win32 window, D3D11, DirectWrite) | Started (2026-11-11): the D3D11 renderer draws a scene into an offscreen surface (layers, runs, scissors, opacity, uploads of ranges, recovery). Glyph and image atlases are texture arrays kept in step by `sync_atlas`. The Win32 platform (`WinPlatform`): windows sized in logical pixels at the display's scale, the message loop with frame pacing, timers, wakes, pointer, wheel, key and text events, frames inside the OS's resize loop. Window surfaces are flip-model swap chains whose frame-latency handle paces the loop (`D3d11Renderer::for_platform`); a lost device gives each window a new one. At start-up one thread makes the software device, which draws the first frames (copied to the window), then the hardware device, which the renderer moves to through the app's recovery (`D3d11Renderer::starting`). The DirectWrite text backend (`DirectWrite`): font matching, script analysis, fallback for what a face lacks, shaping with clusters, coverage per pixel or per subpixel (ClearType, as the Windows setting says), colour glyphs composed from their layers, fonts from memory. Since then, of text: OpenType features (`App::font_features`) and the bidirectional analysis (`AnalyzeBidi`); of the platform: the colours of the system's controls (`SystemColors`). The appearance from the user's settings (`appearance.rs`, read again on `WM_SETTINGCHANGE`), and the window's drop target (`drop.rs`, an `IDropTarget` of its own: files and text). The system's image codecs as the decoder (`WicImages`). The Win32 clipboard is written and never run by a test | `tests/image.rs` (2: a bitmap measured, decoded and scaled with its channels in order, and what is no image an error on any thread; a picture from its file to pixels on the software rasterizer, its two pieces meeting with no seam at its own size and at half of it), 6 pixel tests on the software rasterizer, no GPU or window needed: one goes from a view and a signal through the tree and glyph tiles to pixels. 6 platform tests on windows that are never shown: posted input arrives as events in logical pixels, a resize is reported, what a handler sets off is delivered after it returns, timers, a wake from another thread, a frame when asked. 2 surface tests on such a window: the back buffer holds what the scene says through resizes and a new device, and frames are given as the swap chain takes them; a starting renderer copies its frames, wakes the loop when the other device is made and moves to it. 7 DirectWrite tests against fonts every Windows has: metrics per em, clusters of Vietnamese text as bytes, fallback faces for CJK on one pen, an emoji as a premultiplied colour image, a font loaded from memory, and text from a view to pixels |

### The exit of R3 (specs/00, Phases)

"The gallery matches the V screenshots within the snapshot tolerance." The gallery
(`examples/gallery`) has 64 of the 73 pages of the V gallery; `tools/shots.sh` draws each
without a window, the pages that open something a second time with it open, and
`tools/compare_shots.py` says what share of the pixels of the page differ from the V picture
by more than 24 in a channel, a row of pixels allowed to sit two higher or lower (12,
"Snapshots").

| | Dark (`hl-dark`) | Light (`hl-light`) |
|---|---|---|
| Pictures with a reference | 73 | 73 |
| Mean share that differs | 1.00 % | 1.02 % |
| Over 2 % | 5: the variable list 4.69 %, icons 2.65 %, typography 2.58 %, the start page 2.38 %, the table 2.21 % | 6: the same five (4.74 % to 2.16 %) and the toolbar, 2.17 %, whose reference is a picture of the sidebar's page |
| Without a reference | hover card, drawer and dock, each with something open | the same |

What differs is where lines of text are: they are whole pixels high here and were not in the
V pictures (08), which the two rows of drift do not cover on a page of many lines (the
variable list, typography, the table), and the pages that list the gallery's own pages (the
start page, icons) list 64 of them. The dark pictures were each looked at beside their
reference as their pages were written; of the light ones the two that differ more than their
dark ones were (the toolbar, above, and the open palette, whose reference lists a fourth
command), and the rest go by their numbers, which are those of the dark theme.

These numbers held until 2026-11-14, when the design language took more contrast and colour
(specs/design: tone steps, an accent focus, tinted controls, marks of what is chosen,
hues by kind, rounder corners, deeper dark shadows). The pictures now differ from the V ones
by design, so `QUADRA_REFERENCE` is a check of layout, not of colour; the new look was
judged by eye in both themes.

The nine pages that are left are those of the domain crates and of the terminal, which is on
hold: markdown, the editor, the live table, the log view, the value viewer and the three
charts are R4's; their exit is that they match.

What the kit does not do yet, and no page of the V gallery shows (13 has each beside its
component): faces as tokens of a theme (a reader is told where the characters of an input are, a run a line, and a picture lets go of its decoded pixels once they are in the atlas, and the clause an input method converts is underlined thicker, since 2026-11-14; timelines have steps that run together, `.with`, timelines inside them, `.then`, and plays the app starts again, `.animate_on`, since 2026-11-14; a line of rich text with a larger run is higher, and the links of rich text are reached by Tab and known to a reader since 2026-11-14; the editor tells and takes the line at its top, `.top_line`, centres a glyph in its cells, reads what a `<script>` or `<style>` holds as its language and patterns between slashes in scripts, since 2026-11-14; the editor has a hint over a link and Shift-Enter in its find query, and a press and a release in one frame leave nothing held, since 2026-11-14; a diagram fills a shape of any size in pieces and describes its links and messages to a reader, and a coverage cuts edges at its sides exactly, since 2026-11-14; a diff searches texts far apart in linear space, `diff_lines`, and tints the words that changed in as many places of a line as there are, `changed_parts`, since 2026-11-14; a table has a cursor on cells, `.cells`, a menu of its columns on the header, and a live source may say which rows it changed, `.changes`, since 2026-11-14; a tree asks for what a thing holds when it opens and selects several since 2026-11-14, `TreeNode::lazy`, `.on_open`, `.many`; the calendar marks today and its days are moved over by the keys since 2026-11-14; the UI face is shaped without contextual alternates, which brought the gallery's pictures from 1.33 % to 0.88 % of the V ones on average; a document has one selection over its blocks, `crates/quadra-md/tests/select.rs`; a document says where its blocks are, brings a line into view and scrolls itself mounting the blocks that show since 2026-11-14, `.places`, `.reveal`, `.scrolls`, `crates/quadra-md/tests/places.rs`; an image among the words of a document is in their line, `inline_image`, `image_info`, `image_pixels`; a line is made higher by an inline box higher than its text, `Line::height`, `Line::baseline`; a theme for a subtree is in since 2026-11-14: `.theme(..)` on a box; so are measures, specs/17 "Open tokens": `Measure` and `Amount` in the tree, radii, spaces and times a theme gives, `token::RADIUS_*`, `SPACE_*`, `DURATION_*` and a theme file's `measures`; the kit's controls take their radii from them, and the gallery's 92 pictures stayed the same to the byte);
today's date and the names of days and months in the app's language (local time and i18n,
R5). Since 2026-11-14 a drawer slides in, a toast waits while the
pointer is on it, a tooltip is the description of its view for a reader, the menu key and
Shift-F10 open a context menu, and the palette picks other things than commands
(`.placeholder`, `.empty`, `Command::detail`).

### The exit of R2 (specs/00, Phases)

"The interaction tests of the V suite, rewritten against the semantic tree, pass." Each test
of the V repository's interaction files and where its behaviour is tested here
(`crates/quadra-view/tests/` unless a crate is named):

| V test | Here |
|---|---|
| `input_test.v`: tab order, click focus and text | `quadra-tree/tests/focus.rs`, `text_input.rs` |
| actions by context, and chords | `quadra-tree/tests/keymap.rs` |
| the wheel scrolls and clamps | `input.rs`, `list.rs` |
| an overlay is on top and stays in the window | `overlay.rs` |
| an animation follows the frame clock | `tween.rs`, `frames.rs` |
| `input_field_test.v`: typing, IME, selection, clipboard | `text_input.rs`, `composition.rs`, `clipboard.rs` |
| grapheme backspace, word delete | `quadra-text/tests/edit.rs`, `text_input.rs` |
| a password is masked | `clipboard.rs` |
| a text area's lines and vertical movement | `text_area.rs` |
| the pointer places the caret, a double click selects a word | `text_input.rs` |
| `drag_test.v` (3) | `drag.rs` (3) |
| `a11y_test.v`: roles, sent only while listening, actions | `semantics.rs`, `quadra-app/tests/app.rs` |
| `uia_windows_test.v`: a screen reader sees and operates the tree | `quadra-native-win/tests/access.rs` |
| `vlist_test.v` (3) | `virtual_list.rs`, `reveal.rs` |
| `selectable_test.v` | `selectable.rs` |
| `popover_test.v` (a popover, a hover card) | Kit components: R3, on overlays, timers and Escape, which are here |

Tests find and drive controls through the semantic tree where a control has a meaning
(`get_by_role`, `get_by_text`, semantic actions); the pointer and the keys are used where they
are what is tested.

### The exit of R1 (specs/00, Phases)

- **The four programs run**: `bench/compare` has the counter, a virtual list of 100,000 rows, a
  grid of 600 cells that all change every frame, and a document of 40 paragraphs wrapped to a
  width that changes every frame, each in a real window on the hardware device with
  DirectWrite text. `tools/shot.py` saves what a window shows.
- **The structural budgets** (specs/14) that need no input are tests: nothing, a text that
  changes with and without its size, one row among 10,000, scrolling plain content inside and
  past the painted band, a virtual list scrolled within and across rows. Hover, press, focus,
  transitions, the caret and a theme change wait for input (R2) and style variants (R3).
- **The comparison bench** (medians of three rounds, V then Rust): list 0.38 and 0.36 ms of
  CPU a frame, grid 1.31 and 1.29, document 0.73 and 0.70; start-up 44 and 41 ms; binary 1.63
  and 0.88 MB; idle memory 10.0 and 11.1 MB. docs/benchmarks.md has the table, and what it
  took to get there.

So the decisions the phase was to test stand (specs/01, R1 to R5): a retained tree, signals
with views built once, passes over what changed, display lists kept per layer, and the
layout engine of requests and answers. The one scene built to make this design lose, a grid
whose every cell changes every frame, costs what the V build's costs.

### The kit, from the issues of an app built on it (2026-11-21)

| Issue | Status |
|---|---|
| #2 the reasons a container is down | Done (2026-11-21): `Status::of` names the reasons a cluster writes (`OOMKilled`, `Evicted`, `…BackOff`, `Err…`, `…Error` failed; `ContainerCreating`, `PodInitializing`, `Init:n/m` pending; `Init:<reason>` that reason's). `tests/bars.rs` |
| #5 a line chart's limit and live times | Done (2026-11-21): `LineChart::limit(name, value)`, a dashed line across the plot that the scale reaches, said after the series in the box and to a reader; `.time(start, step)` takes the first sample's time as a prop, so a live chart's times follow its samples. The hover in `line_chart/hover.rs`. `quadra-viz/tests/charts.rs`, the gallery's Line chart page |
| #12 a column that fills the room left | Done (2026-11-21): `Column::fill()` and `fill_by(weight)`: by flex, each filling title and cell grows from its width by its weight (`Sheet::fills`), so the last column ends at the table's edge at any width, a hidden column gives its room, and with no room left nothing fills and the table scrolls. A press on a filling column's edge fixes it at its width before the drag. Pinned tables keep their widths, the pinned columns too. A live change's tint is now on the cell itself (it summed the widths, wrong past a hidden or a filling column). `tests/table_fill.rs`, the gallery's Table page |
| #40 a log's level words, the time gutter, seconds | Done (2026-11-21): a level from the line's first word in any case (`error: ...`, `Warning: ...`) and from klog's prefix (`E1121`: I info, W warn, E and F error); on `.ground()` the time at its edge and the text two letters after it; `.millis(false)` writes the time to the second, in the copy too. Not built: "failed, exit code N" in red. `quadra-data/tests/log_panel.rs` |
| #41 a log tab's level column, level set, marks, wrap, sources, menu, header, follow | Done (2026-11-21): `.levels()` puts a column of seven letters with the level's word (only warn, error and fatal colour a line, debug in `text_2`; `Level::Fatal` apart); `log.set_levels(&[..])`; the filter's matches on the accent at 32 %, radius 2 (`RichText::round_marks`); `.wrap(..)` on a `virtual_list`; `log.append_from(source, ..)` keeps streams in time order, an 8 px dot with two sources or more; `.line_menu(..)` with `ContextMenu::while_open` tints the line, `.notes(..)` a word at its end; `.header(..)` above the lines (not scrolled); `.follow(signal)` through `ListView::end`; the empty row; `.timestamps(..)` reactive. `quadra-data/tests/log_parts.rs` |
| #42 the sideways wheel the wrong way | Done (2026-11-21): a wheel's delta is positive towards the start on both axes (specs/09); Windows gives a tilt to the right as a positive `WM_MOUSEHWHEEL`, so the native layer turns it (`wheel_delta`). `quadra-native-win/src/input.rs`, `quadra-data/tests/log_view.rs` |
| #43 a line's level given, a view as tall as its rows, JSON as message and fields | Done (2026-11-22): `log.append_leveled(source, lines)` takes each line's level from the app; a JSON line's `level` field comes before the words of its message; `.fit(rows)` makes the view as tall as its rows up to that many (`ListView::extent`), wrapped lines counted; `.fields(..)` shows a JSON line as its message and `key=value` fields without level and time, the copy and marks following what shows. `quadra-data/tests/log_fit.rs` |
| #39 small status marks drawn for their box | Done (2026-11-21): below 12 px `status_mark_sized` draws the shapes of a mark made for 10, as parts of the box (square 0.7, hollow border 0.15, triangle 0.92 wide, diamond 0.88, check stroked 0.16, dash 0.8 by 0.2), not the 12 px mark made smaller; 12 and up unchanged. `quadra-design/tests/status.rs` |
| #38 a log panel of the app's own | Done (2026-11-21): a line's `Level`, read once when appended (a name in capitals, or after `level`/`lvl`/`severity`, which covers `level=warn` and a JSON `"level"`); `Log::set_level(least)` with the text filter, `Log::level(place)`; the bar's `Warn`. `log_view(..).bar(false).text(size, row).ground().time_color(token).offset(seconds).levels()`: info `ok`, warn `warn`, error `error`, a line's own escape colours first. Not built: a slot for the app's tools in the bar; the local offset is the app's to give. `quadra-data/tests/log_panel.rs`, the gallery's Log view |
| #37 YAML lines that fold, a YAML writer | Done (2026-11-21): `to_yaml(&value)` (block YAML, plain where it reads back the same, a map in a list on the dash's line); `parse_yaml` reads `- - x`; `Value::without(paths)`, `Value::holders(levels)`; `yaml_view(value).open(n).folds(signal).omit(paths)`: lines `key: value` on the kit's tree, folded to `{n}`/`[n]`, 15 px mono, 24 px lines, 18 px a level, on `canvas`, the key for copying puts the YAML. The kit tree's `TreeNode::folded` (shown only while closed); `detail` now always shows. A list item's dash is in the key's colour. `quadra-data/tests/yaml_view.rs`, the gallery's Value viewer |
| #36 an SVG export on the theme's ground | Done (2026-11-21): `Graph::to_svg` opens with a rectangle over the whole drawing in the theme's `canvas`; `to_svg_on(&theme, Some(paint))` takes another ground, `None` none. `quadra-viz/tests/graph.rs` |
| #35 a still graph | Done (2026-11-21): `graph(..).still()`: no drag, no Ctrl-wheel zoom, no fit by a double click, no capture of the pointer, every wheel on to the page; clicks and hover as before. Without a view it shows its drawing at zoom 1, 16 px in, and without a height it is as tall as that. `quadra-viz/tests/graph.rs` |
| #34 key hints, the query's note, the state after the name | Done (2026-11-21): `key_hint` in `text_2`, its chip 4 px either side and its words 6 px after; a palette's `.note` mono, 12 px, `text_2`; in a palette that is not dense a row's state follows its title as a 10 px mark and one muted phrase with the detail ("NotReady · 23 pods"). `quadra-design/tests/palette_parts.rs`, `small.rs` |
| #33 a graph's export of faces, a graph that fills its box | Done (2026-11-21): `Node::svg(|rect| ..)` writes a node's own SVG in the export in place of a box and its label; `graph(..).fill()` is as tall as its box, following it. Not built: drawing any face from its layout as SVG. `quadra-viz/tests/graph.rs`, the gallery's topology |
| #32 a graph's text at the zoom | Done (2026-11-21): `Div::sharp_scale()` (quadra-tree `Tree::sharp_scale`): a scaled layer painted at the window's scale times its own, its transform only the move that holds its middle (rounded), painted again when its scale changes; a graph's node faces and group titles use it, so names stay sharp and whole at any zoom and a title stays in its band. A pan paints only the links and the moved boxes. A layer inside a sharp one is not folded in (a `ponytail:`). `quadra-view/tests/sharp.rs`, `quadra-viz/tests/graph.rs` |
| #31 status marks of 8 and 10 px, and a row of them | Done (2026-11-21): `status_mark_sized(status, px)` and `status_label_sized(status, words, px)` scale the box and its shape together; `status_dots(statuses, px)` is a row of marks `px / 2.5` apart, to a reader an image named by a count a status ("8 succeeded, 2 failed"). `quadra-design/tests/status.rs`, the gallery's Status page (now `pages/status.rs`) |
| #30 a bare chart without the grid | Done (2026-11-21): a bare line chart has no grid, only one rule in `line` along its bottom; a limit's rule and swatch are in `text_3`. Not changed: the dashes of `BorderStyle::Dashed` (the kit's, not the chart's). `quadra-viz/tests/charts.rs` |
| #29 a meter that fills its box | Done (2026-11-21): `meter(..).fill()` is as wide as its box and follows it; the track is in `line_strong`; the tick reaches past the bar by half its height above and below (12 px on a bar of 6). `quadra-design/tests/progress.rs`, the gallery's Progress bar page |
| #28 a graph's links side to side, and straight | Done (2026-11-21): `graph(..).axis(LinkAxis::Across | Down | Auto)` sets the axis every link runs along (side to side for a map in columns), `.straight()` draws links straight from stop to stop. `quadra-viz/tests/graph.rs`, the gallery's Graph page |
| #27 a picker's rows: mono, dense, a footnote, key hints, no Enter | Done (2026-11-21): `Command::mono()`; `.dense()` (rows of 23 px, headings of 20, a state's mark first and its word muted at the end, a 12 px slot for the check); `.hint(false)`; `.footnote(words)` under the rows; `.foot_hints([(keys, does)], count)` with `key_hint`s. `quadra-design/tests/palette_parts.rs`, the gallery's cluster picker |
| #26 a breadcrumb on one line | Done (2026-11-21): the breadcrumb keeps one line; with less room its middle parts give way first, each cut with an ellipsis down to 24 px, and the first and the last stay whole while they fit. `quadra-design/tests/breadcrumb.rs`, the gallery's Breadcrumb page |
| #25 a line chart's unit on the scale, a dashed series | Done (2026-11-21): a unit that is a sign ("%") follows the scale's values, a word ("m") stays out; `Series::new(..).dashed()` draws dashes of 5 px and gaps of 3 measured along the line (the plot leaves a gap's columns clear), its legend swatch dashed in its colour. `quadra-viz/tests/charts.rs`, the gallery's Line chart page |
| #24 a graph's link through points of its own | Done (2026-11-21): `Link::new(from, to).through([points])` passes points of the plane (a layered layout's points in the columns a link skips): a cubic from one stop to the next, square to the axis at each; hit, dashes, head, SVG and the fit follow it. The routes are worked out once, at mount, not at each move. `quadra-viz/tests/graph.rs`, the gallery's Graph page |
| #6 the status marks of the dashboard design | Done (2026-11-21): running a filled square of 7 px, pending a hollow one (a 2 px border: told apart by the fill, principle 7 kept), failed a filled triangle, unknown a filled diamond; `Status::None` is the 12 px box with nothing in it. specs/design and specs/13; `quadra-design/tests/bars.rs`, the gallery's Status page |
| #23 a graph of nodes and links | Done (2026-11-21): `quadra_viz::graph(nodes, links)`: nodes are views at the app's points with their sizes (a button to a reader), links cubic curves from edge to edge with heads, `.link_style(|i| ..)` reactive (colour, width, dashes, head), `.group(title, ids)` frames; pan by a drag, zoom by Ctrl and the wheel about the pointer, `.view(signal)`, `.fit()` and a double click to fit; `.on_click`/`.on_hover` tell of a node or a link (5 px); `Graph::to_svg(&theme)`. Not built: a layered layout (the app places the nodes) and a raster capture. `quadra-viz/tests/graph.rs`, the gallery's Graph page |
| #20 a bare line chart, the limit in the legend, a note, local times, a fixed top | Done (2026-11-21): `line_chart(..).bare()` (the plot fills the box: no scale, no times), the limit in the legend beside a dashed swatch, `.legend_note(|| words)`, `.offset(seconds)` for local times, `.top(value)`. `quadra-viz/tests/charts.rs`, the gallery's Line chart page |
| #16 a dense table | Done (2026-11-21): `Table::dense()`: a header of 26 px with 12 px `TEXT_2` titles, 12 px from a row's start to its first words and between columns (6 at each side of a cell and each end of a row). A selected row's accent bar runs along all of its left edge, in every table. `tests/table_views.rs`, the gallery's Table page |
| #21 the sidebar's head strip and its rows' inset | Done (2026-11-21): `.head(view)` is a strip of 28 px above the places that does not scroll (12 px muted text, 12 px in and 4 at its end, a `LINE` under it, hidden while folded); a place's row is 12 px in from each side with an icon of 14 px, and the groups' titles start 12 px in. `tests/bars.rs`, the gallery's Sidebar page |
| #22 a count of problems on the activity bar, the status bar's rule | Done (2026-11-21): `Activity::badge_tone(Tone::Danger | Warning)`: a pill of that colour, 10 px, in the cell's corner (5 px down, 4 in), without a ring; the accent count stays the default. The status bar's top rule is `LINE`. `tests/work.rs` |
| #18 a badge in the UI face, without a border | Done (2026-11-21): `plain_badge(words, tone)` for a word that is no code (the kind of an object): 16 px, the UI face, no border, the tone at 16 %, 5 px each side, radius 4; `badge` stays mono for what is code. `tests/parts.rs`, the gallery's Badge page |
| #17 the breadcrumb's spacing and its last part | Done (2026-11-21): 8 px between a part and a mark, the last part (where the user is) in `TEXT`, the others muted. `tests/bars.rs` |
| #19 an icon after a button's label | Done (2026-11-21): `Button::icon_end(body)`, sized and coloured as the icon before the label, 4 px after it; a button may have both. `tests/button.rs`, the gallery's Button page |
| #15 the palette's groups, row parts, foot, and a picker under a control | Done (2026-11-21): `Command::group(heading)` (rows of a group together under a heading the arrows pass by, through the headless `Parts::head`), `.lead(|| view)`, `.status(status, words)`, `.current()` (first, with a check); "Enter" on the row the arrows are on; the search icon and `.note(|| words)` in the query; `.foot(keys, |n| words)`; `.under(&opener)` and `.width(px)`. A title is one line, cut with an ellipsis (`rich_text(..).ellipsis()`, so `marked` is one line in every list). `tests/palette_parts.rs`, the gallery's Command palette page (a cluster picker) |
| #13 a meter with a mark | Done (2026-11-21): `meter(used)` beside `progress`: how full something is, with `.mark(at)` (a tick at a second value over the fill), `.warn_at(part)` (warnings from there, errors past 1), `.width`, `.height` and `.names(..)`; a reader hears both values. The gallery's volumes use it. `tests/progress.rs`, the gallery's Progress bar page |
| #14 a hint under a radio option | Done (2026-11-21): `Choice::hints([..])` puts a muted line of 12 px under each option's label of a radio group ("" for none); the option is named by its label alone and described by its hint. `tests/choice.rs`, the gallery's Radio group page |
| #9 a dialog's action that is off, a confirm that destroys | Done (2026-11-21): `Dialog::disabled(|| ..)` turns the last action off while what it reads says so (neither pressed nor reached with the keys), and `Dialog::danger()` puts the last action in the colour of errors; `confirm` gives its `Dialog`, so `confirm(..).danger()`. `tests/overlays.rs`, the gallery's Confirm page |
| #11 a row that is activated | Done (2026-11-21): `table(..).on_activate(|row| ..)` opens a row as a list does: a double click on it, or Enter on the row the keys are on, gives the row as the data numbers it; the arrows only move; a button in a cell keeps its press and its Enter. The headless sheet holds it (`Sheet::activate`). A single click that opens, for a table with no selection, waits until it is asked for. `tests/table_views.rs`, the gallery's Table page |
| #7 a cell is one line | Done (2026-11-21): a table's text and age cells are one line cut with "…" where the column ends, and follow the column made wider; `status_label`'s word ends with "…" where its line does not fit (a status cell too). The whole value is still copied; a tooltip of the whole value waits until it is asked for. `tests/table_views.rs` |
| #10 an empty Age cell | Fixed (2026-11-21): an `Age` cell that says no number shows nothing (it showed "0s"), and a column of numbers or ages puts the cells that say no number after the others, whichever way it is sorted. `tests/table_views.rs` |
| #8 the space before a matched run | Fixed (2026-11-21): `marked` (the palette's, the combobox's and the searching select's rows) is one rich text with a span a run, not a row of texts: a run's trailing space kept no width there ("Podnode-agent"). `tests/palette.rs` |
| A window that drew nothing after it was minimized | Fixed (2026-11-21): a minimized window that asked for a frame waited for its swap chain and drew nothing, taking a frame no present gave back; restored, it waited for ever, its input and its work going on (OneTerm stood still, locked itself behind a frame that never changed, and closed its settings). The loop no longer waits for nor draws a minimized window, and a window that has waited 100 ms for its swap chain draws anyway, timed from when it began to wait, so messages meanwhile (a busy terminal's wakes) do not put it off. `quadra-native-win/tests/surface.rs` |
| #1 a table's cells that show views | Done (2026-11-21): `Column::view(|row, says| view)` builds a cell's view once for each mounted row from two memos (the row as the data numbers it, what the cell says), so the scroll and a change rebind it and build nothing; the string still sorts, copies and is compared by live data. `Column::mono()` and `Column::muted()`. A button takes its press: the headless press ends the click's dispatch, so a button in a row does not select it. Bars and dots are the app's views (the gallery's volumes show a bar), not `CellKind`s yet. `tests/table_views.rs`, the gallery's Table page |
| #3 a drawer beside the content | Done (2026-11-21): `Drawer::beside()` puts the drawer in the layout next to the content, on the panel's plane with its line on the content's side: nothing outside closes it and the focus is not kept in it; it still slides in, is a dialog to a reader, takes the focus as it opens, gives itself up on Escape while it has the focus, and gives the focus back unless it went elsewhere. `Drawer::head(|| row)` puts the app's row in the place of the title, before the cross. `tests/cards.rs`, the gallery's Drawer page |
| #4 a sidebar's counts and foot | Done (2026-11-21): `Sidebar::counts(|id| ..)`, a live muted count at the end of a place and after its name for a reader, not there while folded, not read by the search; `Sidebar::foot(view)` under the places, outside their scroll. `tests/bars.rs`, the gallery's Sidebar page |

### Markdown, the app (specs/21, from 2026-11-19)

| Milestone | Status |
|---|---|
| M1 the window, a file opened in Preview, the layouts | Done (2026-11-19): `apps/markdown` (`MdApp`, `Model`, the bar, the three layouts by the bar's segmented control and Ctrl+1, 2, 3; the window's title by `Window::set_title`, new: `WindowRequest::Title`). `tests/layouts.rs` (3); `bench/markdown-shots` draws the sample in each layout |
| M2 the editor, files | Done (2026-11-19): the editor (prose wrapped, its changes marked against the text saved, the unsaved dot), the preview 120 ms after the last edit, New, Open, Save (beside the file, renamed over it; CR LF kept), Save as, a file dropped, links (Ctrl: a heading, an address, a document beside), and the question before unsaved work is lost (New, Open, closing). New in `quadra-code`: `Editor::edited`, `EditorHandle` (`read`, `edit`; `tests/handle.rs`); new icons `folder-open`, `heading-1..3`, `list-todo`. `tests/files.rs` (4) |
| M3 panes by headings | Done (2026-11-19): side by side, the pane the wheel, a press or a key moved drives and the other follows, linear between the headings' pairs (`sync.rs`, unit-tested: `y_of`, `line_of`, `pairs`); a heading pressed in the preview puts the editor's caret on its line and both panes at it; the caret on a heading's line shows it in the preview; the heading chosen tinted 600 ms. New: `Editor::position`, `Editor::caret_line` (`quadra-code/tests/top.rs`), `Markdown::on_heading` (`quadra-md/tests/places.rs`, with a fix: a document removed before its layout wrote its places from a removed node). `tests/sync.rs` (4) |
| M4 the toolbar | Done (2026-11-19): under the bar while the editor shows, 16 buttons in four groups with their keys (`toolbar.rs`, `ApplyFormat`), each one change of the editor, the focus back in it after (`EditorHandle::focus`, new). What they do is `format.rs`: pure functions from the text and the selection to the edits and the selection after (8 unit tests: styles wrapped and taken off, headings, lists and quotes toggled, links, blocks with blank lines around). `tests/toolbar.rs` (2) |
| After a review (2026-11-19) | An editor built again (another layout) goes to its kept position once laid out, and tells no moves before (a loop between the view and the position, `quadra-code/src/editor/time.rs`); a heading shown is let go after its tint, so a new layout of the preview does not pull it back; a byte order mark is dropped when a file is read. `tests/sync.rs` (6), `tests/layouts.rs` (4) |
| M5 the logo | Done (2026-11-19): `assets/logo.svg`, the mark of Markdown in a blue square on dashed guides and hatching, the pen's curve with its nodes and handle, a yellow circle not yet placed; in the bar and in an empty window. New: a drawing with colours of its own is painted in them from the colour atlas (`Drawing`, `Svg::render_rgba`; `quadra-view/tests/drawing.rs`). The window's and the executable's icon wait for a window icon in `quadra-platform` |
| M7 settings and About | Done (2026-11-19): a settings window (Ctrl+, or the bar's button; `settings.rs`), the kit's pages: the theme, the editor's text size and wrapping, the preview's text width, the layout a file opens in, scrolling together; the outline and the split kept as the window's state (`prefs.rs`, `settings.json`). The code face's ligatures are off in the app. About: the version, the program, and Set as default, which registers Markdown for `.md` and `.markdown` under the user and opens the system's Default apps page (`quadra-shell` `Associations`, `registration`, new). `tests/settings.rs` (3), shots `settings` and `about`. Choosing Markdown on the system's page is to be tried by hand |
| M8 tabs and one process | Done (2026-11-19): the window's documents are tabs (`tabs.rs`, `view.rs`: the kit's tab strip under the bar from two tabs, the file's name with the unsaved mark, a cross; a file opens in its tab if open, in a blank untitled tab, else in a new one, and asks nothing; Ctrl+T, Ctrl+W, Ctrl+Tab and Ctrl+Shift+Tab; closing a tab or the window asks for each with unsaved work). One process: `quadra_shell::single_instance("markdown")`, a later start hands its files over and the window comes to the front (`Window::to_front`, `WindowRequest::Front`, specs/10; the later process allows the foreground, specs/16). `tests/tabs.rs` (9), shot `tabs`. Tried on Windows: a second start with another file exits in 73 ms and its file is a tab of the first's window |
| M9 status bar | Done (2026-11-19): the active tab's words, characters and reading time (counted on the text the preview shows), the editor's cursor and selection while it shows (`quadra-code` `Editor::caret`, `Caret`, new), the lines and the line ends (`status.rs`). `tests/status.rs` (1), a unit test of the counts |
| M10 find | Done (2026-11-19): Ctrl+F is the editor's find while the editor shows (`EditorHandle::run`, new; from either pane), else Preview's bar (`search.rs`: the query, Match case, Whole word, `n of m`, Enter, Shift+Enter, F3, Escape; each tab's own). The preview marks the matches in its texts, code and tables too (`quadra-md` `Markdown::find`, `Find`; `RichText::mark`, `text`, `rect_of` in `quadra-tree`, `rich/marks.rs`; `Query::find_text`; `NodeRef::focus_first`), the current one brought into view, found again after a layout that moved the blocks. Tests: `crates/quadra-md/tests/find.rs` (2), `apps/markdown/tests/search.rs` (3), shot `find`. A match across two styles (`**a**b`) is not found |
| Window frame and layouts | Done (2026-11-19): the main window and the settings window draw their own frame with the kit's title bar (the main one 40 tall, holding the bar; the settings' 34 with its caption). A layout changed builds nothing it built before (`Split::alone`; dist, 920 lines: 20 ms to Side by side, 11 back), and a focus under a hidden pane goes (`Tree::drop_hidden_focus`). Find bars' rows are 26. A test in `tests/layouts.rs` |
| The icon (2026-11-19) | `assets/logo.ico` at 16 to 256 px, the mark alone under 40 px (`logo-small.svg`), made by the `icon` scene of `bench/markdown-shots`; `build.rs` writes it as a resource file and links it (no resource compiler); the platform gives every window the executable's icon resource 1 (`quadra-native-win/src/icon.rs`). `tools/package.sh markdown`, which checks the icon is in the binary (`tools/exe_icon.py`). The window's icon on a real desktop is to be looked at by hand |
| Tables and large documents (2026-11-19) | A table is as wide as the page and prose in a cell wraps (before, a long cell made the table wider than the page); a column is at least its longest word up to 160 px, code measured in the code face, a longer word broken in its cell; fr tracks share as CSS does, one held at its least leaving the rest to the others (`quadra-layout/src/grid_tracks.rs`). `quadra-md/tests/tables.rs` (3), a grid case; shots `tables`, `tables-side`. The `large` scene of `bench/markdown-shots` (dist), each step to its last frame presented: 920 lines open in 24 ms, side by side 40 ms, the editor moved with the preview following 8 ms, a letter to the preview 16 ms; 9 200 lines 135 ms, 386 ms, 10 ms, 129 ms |
| M6 the outline | Done (2026-11-19): a sidebar of 240 px at the left in every layout (`outline.rs`), the kit's tree of the headings nested by level, a long one cut with an ellipsis; the row of the section at the top selected as the panes move, a row chosen taking each pane there; the bar's Outline button and Ctrl+Shift+O. The headings are read once a text and shared with the following (`sync::headings`, `sync::mapping`). Found on the way: a position asked of an editor waits for its layout without the caret's own scroll undoing it (`quadra-code/src/editor/time.rs`). `tests/outline.rs` (2) |

### OneTerm's remaining parts (2026-11-18)

- The working state turns: its icon is the kit's `spinner()` (named by `.label(..)`, new), in
  tabs, badges and headers (`marks::state_icon`; `tests/signals.rs`).
- The find bar again (specs/19, "Find"): one row of 28 px, the query small with the search icon
  and the error colour while nothing is found, Match case and Whole word as icon toggles (the
  kit's new `case-sensitive`, `whole-word`), the count in a fixed box, a rule, the arrows off
  while nothing is found; it shrinks to stay inside a narrow space. Picture: the scene `find`
  of `bench/oneterm-shots`.
- A window's minimum size (`WindowOptions::min_size`, `quadra_platform::at_least`; specs/10):
  `WM_GETMINMAXINFO` at the window's DPI of the moment, the frame by `AdjustWindowRectExForDpi`
  (a frame of the app's own adds only its borders); a smaller size, `fit` or remembered place
  opens at it; the headless platform holds `resize` at it (`quadra-platform/tests/headless.rs`).
  OneTerm's is 720 by 480. A real window's drag, DPI change and snap are manual checks.
- A bell's sound or flash (Notifications & signals: a dot and a count, a sound, a flash,
  ignored): the system's default sound for each bell, or the space tinted the selection colour
  for 120 ms (`Space::flashing`; `tests/bell.rs`).
- Where a new tab starts: the last directory used, the one a local shell said last, kept as
  `terminal.last_dir`, home once it is gone (`tests/start_dir.rs`).
- Links: the rows of a logical line above and below those that show (32 each way) are read
  from the engine, so a URL wrapped off the screen is whole (`crates/quadra-term/tests/links.rs`).
- Drawn by hand too: the sextants (U+1FB00 to U+1FB3B, rows at the whole pixels nearest a third
  and two thirds, columns the half blocks') and the separators E0B4 to E0BF (half circles and
  their arcs, corner triangles, diagonals), checked against their names at six sizes
  (`drawn/shapes.rs`, `drawn/tests/oracle/shapes.rs`); the legacy computing symbols keep their
  colour and are no part of a ligature.
- Shell profiles (`ShellProfile`: Unix, Cmd, PowerShell, Dumb; `Session::set_shell_profile`):
  the prompts and options only a shell has over the looks every one has, Command Prompt's `>`,
  PowerShell's `>>` and provider locations, a router's `core-sw1#`, `/` options
  (`highlight/profile.rs`, a unit test); OneTerm sets each space's by what it started
  (`tests/profiles.rs`).
- The lock blurs the workspace behind its card (24 px, the scrim over it) and makes it inert:
  `State::INERT` (`Tree::set_inert`, `Div::inert`; specs/12): shown, but out of the semantic
  tree, and no press or Tab reaches it (`quadra-view/tests/semantics.rs`; `tests/lock.rs`). The
  dock's Expand button shows sharp over the blur. What was open over the window (a menu, a
  dialog) closes as the lock falls (`Tree::dismiss_overlays`); locked, only the card's and the
  title bar's controls are in reach (`tests/lock.rs`).
- The gallery's Toast page shows a card with another icon and one with a picture (`Notice::icon`,
  `picture`; a test in `crates/quadra-design/tests/notices.rs`).
- On oneterm-vt v0.4.1: finding by a regular expression (`FindQuery::pattern`, `regex-lite`
  through `GridText::span`; the find bar's Regular expression switch; `tests/find.rs`), and a
  card that names the sequence it came from (`Notice::source`; `tests/notices.rs`).
- On oneterm-vt v0.4.2 (`Registered`, the console on its poller in safe code): `Pty` moved from
  `quadra-shell` into `quadra-term` behind its `pty` feature; `quadra-shell` no longer depends
  on `oneterm-vt` or `polling` (`crates/quadra-term/tests/pty.rs`).

### Saved passwords, Connect all, the editor's menu (2026-11-17)

- **Saved passwords** (specs/19, "Saved passwords"): a host's password or key passphrase is
  saved sealed (AES-256-GCM, `quadra_shell::seal`, specs/16) under a data key that the master
  password seals (`ssh.vault`, `ssh.secrets`); the key is in memory once the master password
  is typed (set, unlock, or the Unlock saved passwords dialog that connecting asks first). The
  engine's first question of that password or passphrase is answered with it. Changing the
  master password seals the key again; removing it forgets every secret, after a dialog
  that says how many (Remove the master password?; `tests/lock.rs`). Forget password in a
  host's menu. Tests: `apps/oneterm/tests/vault.rs`, the vault's and the seal's unit tests.
- **Connect all (n)** in a group heading's menu: a tab for each host, one master password
  asked for all.
- **Secrets wiped in memory** (`zeroize`, specs/01 R15, already in the tree): derived keys, the
  data key, secrets opened, the password fields once used, the engine's passphrase.
- **The host dialog**: the password field is always enabled; with no master password it sets
  one there (New master password, Repeat it); the key file has Choose… (the system's dialog).
- **A startup command** per saved host (`ssh.startup`), written once the shell opens (a Retry
  and Duplicate too). Test: `tests/vault.rs`.
- **Shift over a program that reports the mouse** selects, nothing reported: a test now holds
  it (`crates/quadra-term/tests/input.rs`).
- **The editor's menu**: Copy, Paste, Find, Close (Close only for a binary file or while the
  diff shows). Test: `apps/oneterm/tests/editing_menu.rs`.
- **Sixel and kitty images** in the terminal view (specs/18, "Images (Sixel, kitty)"): taken
  after each feed, premultiplied once, painted from the colour atlas at their cell under the
  text, clipped to their box and the grid and scrolling with their rows, freed on
  `GraphicReleased`; the cell's device pixels told to the engine. Kitty graphics (the engine's
  `kitty-graphics`: `a=q` answered, a placement's part of its image stretched to its cells,
  lower layers first). While `?2026` holds a frame, the engine keeps its placements and
  holds their releases back, so a video of images never shows an empty frame; the feeder
  feeds with `feed_frame` and copies each frame a program closes where it closes, so one
  that draws without a pause (each read closes a frame and opens the next) shows every frame
  (the engine at a commit of its main after v0.2.0, which fixed issues 1 and 2 of its
  repository). Test: `crates/quadra-term/tests/images.rs`.
- **Drawn characters and grayscale smoothing** (specs/18, "The view"; each of the 164
  checked against its Unicode name at six box sizes, `drawn/tests/oracle.rs`): box drawing, block
  elements and the separators U+E0B0 to U+E0B3 are drawn to their cells' box in device pixels
  (`crates/quadra-term/src/drawn.rs`), lines of the side's parity so every mask is the mirror
  of its mirrored character about both middles; glyphs are smoothed by pixel
  (`TermLook::grayscale`, `quadra_text::GRAYSCALE`, `paint_glyph_with`; OneTerm's Grayscale
  smoothing, `terminal.grayscale`, on). Tests: `src/drawn/tests.rs`, `tests/drawn.rs`,
  `apps/oneterm/tests/settings.rs`. A cell's height takes its width's parity (the nearer,
  the larger on a tie), so drawn lines are as thick down as across.
- **The vault at start and secrets bound to their target**: with a vault the window opens locked, and the password that opens it opens the data key (`tests/lock.rs`); a secret is `sv2:`, sealed with the host's target as AES-GCM's associated data (`quadra_shell::seal(key, plain, bound)`), so one copied into another row, or a host whose target was edited, does not open it (an edited target seals it again while the key is in memory, else forgets it with a card); `sv1:` rows still open.
- **Close to the right, text zoom, crash reports**: a tab's menu closes the tabs after it (asked first as any close; `Model::close_right`); `zoom_in` Ctrl+=, `zoom_out` Ctrl+- and `zoom_reset` Ctrl+0 (`Command::Zoom`) step the terminals' text size; OneTerm installs `quadra_shell::crash_reports` and shows the last run's report as a card with Show report (`OneTerm::crashed`). Tests: `tests/tabs.rs`, `tests/settings.rs`, `tests/links.rs`.
- **An input method's composition in the terminal view** (specs/18, "Input"): drawn at the
  cursor over its cells (wide characters on two, clipped at the row's end), underlined, the
  clause thicker, the cursor hidden; nothing sent until it is committed; an empty composition,
  a commit or the focus leaving ends it (`crates/quadra-term/src/ime.rs`). An Alt+drag block
  selection has a test now. Tests: `crates/quadra-term/tests/ime.rs`, `tests/input.rs`. A typed character
  is a key, with Shift when it needs it on a US layout, so a program that asked for kitty's
  report-all-keys keeps its capitals; an input method's commit of several characters goes as
  text (`quadra_term::input::typed`). A key's release goes to a program that
  asked for event types (`keys::release_event`); the window's keyboard leaving and coming back
  is told to a program that asked for focus reports (`Tree::on_active`, specs/09: the focus
  itself stays, so a selection or an edit survives switching windows). OneTerm's local shells
  are told `TERM_PROGRAM=OneTerm` and its version, and `FORCE_HYPERLINK=1` while the setting
  Force links is on (`terminal.force_hyperlink`, on), so programs that know terminals by name
  show OSC 8 links (`apps/oneterm/tests/local_env.rs`). The
  pointer takes the shape a program names (OSC 22, CSS names onto the system's cursors:
  `link::shape`), the hand still over a link under Ctrl, the text cursor back on the engine's
  `PointerReset`. Test: `tests/links.rs`. With the engine at v0.4.0 (2026-11-18): OSC 22
  stacks and queries answered from the cursors there are (`link::drawn_shapes`); the colour
  scheme, dark or light by the background, told to the engine with the theme's colours and
  reported to a program that subscribed (`?2031`; `CSI ? 996 n` answered); OSC 99
  notifications in full (`quadra_term::Notice`: id, buttons, urgency, expiry, what it asks;
  `on_notice_close`, `on_notice_alive`). Test: `tests/asks.rs`. The kit's cards open by their
  words (`Notice::on_open`), tell when they go (`on_close`, from `Queue::on_gone`) and take
  another's place (`Notices::replace`, `shows`; `quadra-design/tests/notices.rs`). OneTerm
  declares every ability (`oneterm::abilities`): a program's buttons on its card, opening
  shows its space, the reports it asked for, a card replaced by its id, its close and alive
  query answered; one wanted only unfocused or unseen dropped while its space is seen (the
  window's `is_active`, `is_minimized`, from `Tree::window_state`; `quadra-app/tests/frame.rs`);
  its icon by name, else its data kept by its id (8), on the card (`Notice::icon`, `picture`);
  a sound it names played (`quadra_shell::Sounds`, `MessageBeep`), none when it names none
  (`apps/oneterm/tests/notices.rs`). The real decoding of an icon and the real sound are
  manual checks.

### Editing in OneTerm, each space's dock (2026-11-17)

- **An editor space** (specs/19, "Editing in OneTerm"): Edit… on a remote file opens it in a
  space of the tab by default (the external editor stays a setting): the type found by name or
  `#!`, highlighted; a binary file shown as one and not edited; the header with the full
  remote path, the type, `● Modified`, Diff (the kit's diff against the text opened or last
  saved) and Save (Ctrl+S, uploaded on the file's own connection); closing with changes asks.
  `quadra-code`: `Editor::changed`, `EditorWidget::set_base` (specs/13). Tests:
  `apps/oneterm/tests/editing.rs`.
- **Closing with changes asks everywhere**: a tab's close, Close others, Quit and the window's
  close ask each editor with changes in turn, its tab shown (out of the expanded SFTP view)
  and the dialog keeping the focus; Cancel, the cross, Escape or a failed save stop and take
  back the Don't saves. The editor's document (text, cursors, undo) outlives its view when its tab is
  left (`Editor::keep`, specs/13): before, coming back to the tab lost the edits. The header
  cuts the folder and keeps the file's name; a tab's second line leaves editors uncounted.
- **The dock**: each space keeps its folder, entries, choice and transfers; two clicks open a
  folder; its left edge is dragged (220 to 800 px, `sftp.dock_width`); a transfer takes the
  whole width. Tests: `tests/dock.rs`, `tests/dock_tabs.rs`.
- A header names the shell until the program gives a title, a title's path shortened.

### SSH asks in dialogs (2026-11-17)

- **The engine asks the app** (specs/19, "M7 in detail"): `Ssh::connect(options, output,
  told, exited)`; `Event::Connecting`, `Ask` (the user, an unknown host key, the password, a
  passphrase, a failure), `Open`; `Ssh::answer`, `Ssh::cancel`; a failure answered retries.
  Nothing is asked in the terminal. A target without a user leaves it empty and asks it.
- **OneTerm's Connect dialogs** (`asking.rs`): Connecting with Cancel, Unknown host with its
  fingerprint (Trust and connect / Cancel, the focus on Cancel), Sign in, Could not connect
  (Retry / Close); the focus back in the terminal when the shell opens. Tests:
  `oneterm-ssh/tests/live.rs`, `apps/oneterm/tests/asking.rs`, all on 127.0.0.1; checked
  against the user's VM by a probe under `.tmp/`. Shots: `ask-*` scenes.
- The gutter keeps no digits ahead: it grows with the highest line number (specs/18).
- A tab draws nothing: the engine keeps a printed tab in its cell (a newer console host
  passes `ls`'s tabs through), and it was drawn as a missing glyph (specs/18).

### Fixes after a trial (2026-11-17)

- A click in a submenu reaches it (`press_outside`: an overlay beside a node of another is
  inside it); a field given the focus back after its view is gone starts nothing. Test:
  `quadra-design/tests/submenus.rs`.
- A seam of a pane tree takes one pixel, its grip over the panes' edges (specs/behaviour/16).
- The gutter's line number right-aligned by the grid, 3 digits at first (specs/18); its colour
  `#2b7f99` by default; an empty colour picker shows the code of its fallback (specs/13).
- OneTerm: a host connects on a double click; the host dialog's fields 28 px high.
- Checked against a real server (a VM the user gave, by password, files under `.tmp/`): the
  engine's shell and SFTP, and OneTerm's Quick connect, password prompt, `ls` and dock.

### Pickers, one overlay plane, OneTerm's bars (2026-11-16)

- **Kit** (specs/13, specs/design): `color_picker` (saturation and brightness, hue, hex, presets;
  empty is the default with its fallback swatch), `file_picker` / `folder_picker` (a path field and
  Browse, the dialog given as a callback; tests on `FakeShell`), gallery pages for them. Every
  overlay is the menu plane (`overlay_bg`, a `line` border, `radius_sm`); `overlay_bg` is `panel`
  in the kit's two themes. A submenu stands beside its menu's edge, on the left when the right
  has no room. The rows of `settings(..).cards()` are as wide as their column. A row's
  title names its switch, checkbox, input or slider for a reader when it has no name of its
  own (`Tree::meaning` reads a node's semantics, its widget's part included). Tests:
  `quadra-design/tests/pickers.rs`, `menus.rs`, `settings.rs`.
- **Shell** (specs/16): `Usage` with the private working set, the working set, its peak, private
  bytes, threads and uptime; the `Meter` trait, faked by `FakeShell::set_usage`.
- **Term** (specs/18): the gutter's time and line number one cell apart.
- **OneTerm** (specs/19): the bell in the title bar (Search, Bell, Lock, Settings); the clock
  `yyyy-mm-dd hh:mm:ss` from `OneTerm::now`; the status bar's icons in colours; RAM the private
  working set, a card of the process's usage on hover; the New terminal row in a dashed box; a
  line under each space's header; the terminal's padding 0 on every side and no other inset;
  numbers in steppers, colours in the colour picker, the Keybindings page with the kit's Reset.
  Tests: `apps/oneterm/tests/bars.rs`, the others brought along.
- A host's colour tag is chosen in the colour picker: any `#rrggbb`, the seven hues kept
  by name; `MenuItem::swatch` takes any paint. Tests: `apps/oneterm/tests/hosts.rs`.
- **A text keeps its answer over a range of widths** (specs/06, "Requests and answers"): a
  leaf says for which available widths its size holds (`Holds`, `Widget::holds`; the text widget
  from its widest line to where it broke), so the requests of nested auto-sized boxes hit its
  cache. A tick of OneTerm's clock lays out 4 nodes, not 168. Tests:
  `quadra-layout/tests/status_bar.rs`, `quadra-view/tests/text_widths.rs`, `oneterm/tests/bars.rs`.
- Open: rich text and other widgets keep exact requests; a squeezed bar whose texts wrap still
  climbs.

### The menu plane (2026-11-16)

Menus, context menus, selects, combobox lists and the command palette on the menu plane
(specs/design, "Elevation", "Menu", "Picker / command palette"): `panel` with a `line`
border and `radius_sm`, rows of 28 px with the text 12 px from the edge, separators of
`line_strong` 4 px off their rows, the palette's keys in 12 px (`popup::menu_surface`,
`popup::listing`). OneTerm's space menu without icons, its groups split by lines (specs/19).

### OneTerm after the revised canvas (2026-11-16)

The user's canvas "OneTerm" revised in the Quadra design language (specs/19, "Window", "Rail",
"Signals", "Lock", M7, M9): the title bar's three icon buttons; tabs as 36 px rows, collapsed to
48 px (Collapse tabs to icons) with each tab's signal as a badge; the strip with its accent line
on top; each state its own outlined icon, the header's label from the record's id
(`Session::status_name`); hosts with colour tags; the `+` menu; the status bar of 30 px with
the rail's and the dock's toggles; notification cards at the bottom right; the dock's path field
and rows; the expanded view's panes, columns and transfers; Quick connect; the lock card. Kit:
`Input::prefix/mono/small/height/fill/reveal`, `Button::shortcut`, `Dialog::shortcut`,
`Notice::note`, `MenuItem::swatch/mono`, `menu_beside`, `TextInput::reveal`. Pictures:
`bench/oneterm-shots` (scene `collapsed` added). Tests: `apps/oneterm/tests/rail.rs`, the
others brought to the new names. Open: Windows Hello, the machine's RAM and network rates (no
framework service yet), a transfer queue, selects in the input's look.

### OneTerm's collapsed tabs, option H (2026-11-21)

The canvas's "H · tab icon and a coloured state word" (specs/19, "Rail"): the folded rail is
56 px, a tab 46 px: its icon of 16 (`text_2`, the accent while it shows) and under it its
signal as one word of 10 px in the state's colour (`blocked`, `error`, `done`, the progress in
mono or `working`, `bell`, `idle` in purple), the seam bar for a progress. `marks::tab_word`
replaces the badge. Working with no progress makes the seam the kit's busy bar
(`progress_busy()` gains `.height(px)` and `.track(token)`). Test: `apps/oneterm/tests/signals.rs`,
`quadra-design/tests/progress.rs`; picture: scene `collapsed`.

### OneTerm's tooltips: folded tabs and usage (2026-11-22)

A folded tab's tooltip shows at its right, and under its words the focused space's working
directory and its shell's program (`cmd.exe`, `pwsh.exe`, `ssh user@host`; `marks::tab_where`,
`complete::program`). CPU and RAM in the status bar show the kit's tooltip instead of the hover
card, holding the card's two columns (the kit's tooltip `.content(|| view)`), and are told to
a reader as `OneTerm: CPU 3%, RAM 182 MB`. The kit's tooltip `.meta(..)` may change and holds a
line for each line of its words.

### OneTerm: the rail without lines, no search in the title bar (2026-11-22)

The rail's tabs, expanded and folded, have no line between them and fill `panel_hover` at
half under the pointer; a progress bar runs along a row's bottom edge. The folded rail's New
terminal is the expanded one's dashed box. The title bar's Search is gone: every `+` menu (the
rail's, folded or not, and the strip's) has the search field now, Quick connect by a typed
target with it. A terminal's padding is 5 at the top and the left by default, and its cursor
blinks.

### OneTerm: what a folded tab runs, the bars tidied (2026-11-22)

A folded tab's tooltip is two lines: the focused space's directory, then the shell's program
and the command it runs (`pwsh.exe · npm run dev`), or with none its full path. The command
is read from the OS when the tooltip shows: `quadra_shell::shell_process(pid)` (the newest
child of the shell but the console hosts, its command line by `NtQueryInformationProcess`),
the pid from `Pty::pid` kept on the session. An idle tab is purple expanded too, as folded.
The status bar drops `local shell` (the host group is a remote space's), puts git in a group
of its own after the directory, and the lock's minutes moved into Lock in the title bar.

### OneTerm: defaults, rename in a dialog, a dot while logging (2026-11-23)

New defaults: completion off, the rail 270 px, frames a second shown. The folded rail's
middle button closes a tab as the expanded one's does. In the rail at its width two clicks
(or Rename) open the Rename tab dialog; the strip still renames in place. A tab of one space
shows no space header. A space's menu drops Split space (Ctrl+D stays) and puts Start logging
in a group of its own; while the focused space logs, the status bar has a red dot that blinks
each half second (`Session::logging` looked at then, as a log can end on its feeding thread).
Tests: `apps/oneterm/tests/rail.rs`, `menus.rs`, `signals.rs`.

Then: OSC 7501 shows only what a program sent (no `idle` word of a tab that said nothing);
the Rename tab dialog laid out as SFTP's Rename, a Name field 12 px in. With no tab left the
window stays (Ctrl+W on the last space too) and the workspace says `Ctrl+T opens a terminal`;
Quit and the close button end OneTerm (`rail.rs`). A folded tab with no word has its icon in the
box's middle. Each kind of shell has an icon of its own (`shell_icons.rs`: Command Prompt,
PowerShell, Git Bash, WSL, Nushell, a bash), on its tabs and in the `+` menu; Nushell, MSYS2
and Cygwin are found too, and `CHERE_INVOKING=1` keeps a login bash where it starts.

SSH signs in as OpenSSH does: `none` first, so an account with no password is let in unasked;
the password, empty too, goes by `keyboard-interactive` when the server offers that and not
`password` (PAM), each prompt answered with it (`oneterm-ssh/tests/live.rs`).

A local shell gets the user's environment as the registry says it now
(`quadra_shell::user_environment`, `CreateEnvironmentBlock`): a variable set after OneTerm
started is in it. In the expanded SFTP view `..` goes up on a double click, as in the dock.
The status bar has no host group; a remote space shows its network instead, before CPU and
memory: `↓ 12 KB/s ↑ 1.1 KB/s`, the first hop's socket counted (`traffic.rs`). A server's
OSC 7 sets the status bar's directory (`asking.rs`).

A saved host connects from its Connect dialog (`host_connect.rs`): a click on it, or its menu's
Connect or Connect in a new space, shows `Connect to <name>` with what is saved (Username, which
takes `user@host:port` for this connection, Auth, the key file, the password), the focus in the
first field missing. Connect reads `Connecting…` and is off while it connects; the tab or the
space opens only once the shell is open; a failure or a refused password is a danger alert in the
dialog (`connect_host.rs`). The kit's dialog has `.stays_open()` and `.busy(label, ..)`, and the
drawer moved to `dialog/drawer.rs`.

The SSH dialogs follow the design canvas (2026-11-23, specs/19 "The rail"): the Connect dialog's
head with the colour tag and the group, Address, Jump host and Host key (`oneterm_ssh::known_key`
reads the known hosts), the fields two to a row with the eye, a new key trusted in the dialog,
Save … to this host and Open in the current tab, `Edit host…`; connecting, the steps (the engine's
new `Event::Authenticating`) and the server's questions answered in the dialog (`host_connect/`).
The host editor (`hostlist/layout.rs`): `Edit <name>` or Add SSH host with the tag, Address and
Port, Username and Jump host, Auth method and its password or passphrase, `Delete host…` asked
once more. The kit's dialog has `.head(..)`, `.aside(..)` and `.shown(..)`; `bench/oneterm-shots`
draws `host-connect` and `host-edit`.

The folded rail is the canvas's option I (2026-11-23, specs/19 "Rail"): 48 px, tabs of 44 with a
line between them, the icon and under it the signal as a coloured line of 22 by 4 (warn, error,
ok; the progress as a meter; the busy bar while working; a bell as two red pieces), the one
that shows with a short floating accent bar, New terminal a plain `+` (`rail/narrow.rs`;
`tests/signals.rs`, `tests/rail.rs`).

SSH keys are kept in the settings (2026-11-23, specs/19 "Keys"): the setting `ssh.keys`, the
group Keys of SSH & hosts (Add key…, Find in ~/.ssh, each key's kind and fingerprint by
`oneterm_ssh::key_info`, Remove; `sshkeys.rs`), and the Connect dialog and the host editor choose
a key by name in a select in place of a path and a file dialog (`tests/ssh_keys.rs`, the
`bench/oneterm-shots` scene `keys`).

The setting Card when blocked is gone: a program blocked or failed (OSC 7501) shows on its tab
and header only, with no card.

### Status marks a step up, tabs without a hover box (2026-11-22)

A status mark (and a status label's) is 14 px unless told, a step up from 12 beside 14 px
text: the 12 px shapes scaled, their edges on whole pixels (a square of 8, a dash of 9 by 2).
The gallery's status bars keep a 12 px mark beside their 12 px text. An underlined tab under
the pointer no longer fills a rounded box that sat on the line under the row: its label lights
up; a pill tab still fills.

### Badges and tags in 12 px text (2026-11-22)

A badge and a plain badge are 20 px tall, a tag 22 (the sm control height), all in 12 px
text 6 px from each side; they were 16 and 18 in 10 px, small beside 14 px text.

### The design language, reviewed and made a folder (2026-11-24)

`specs/design` replaces the one-file design language (its findings and decisions:
`specs/design/decisions.md`). Done and tested:

- **The language**: an identity in four statements, ladders of four steps for distances,
  heights, radii, text and icons, the rules of layout (within, between, around), one look for
  a family of controls, the surfaces with their insets, recipes with code, a checklist.
- **The kit carries the layout**: `vstack`, `hstack`, `form`, `columns`, `button_row`,
  `section`, `sections`, `spacer` (`layout.rs`); a field told no width is as wide as its
  column in a `form()` and in a dialog's body; the body of a dialog, a drawer and a popover
  is a column with its inset and gap; `Dialog::width`, a confirm 360 wide.
- **One scale**: `radius_lg` is 10 and `radius_xl` is gone; the measures `space_xxs` ..
  `space_xxxl`; popups have corners of 6 at level 1 and 10 at level 2; a select has the field
  look and its menu its width; data rows are 24 and item rows 28; marks are squares; the 63
  distances, radii and text sizes off the ladders are on them.
- **The check**: `tools/check_design.py`, run by the gate, rejects a literal distance, radius
  or text size off its ladder without a reason (`// design: why`).
- **The proof**: the gallery's "Patterns" pages (`pages/patterns.rs`), built with the helpers
  alone and quoted by `specs/design/patterns.md`; `tools/shots.sh` draws them.
- **The trial**: an agent given the folder alone wrote a dialog with a form and a card that
  compiled and were laid out right undrawn; what it found wrong or missing is fixed in the
  kit and the text (`decisions.md`, "The trial"; `tests/layout.rs`).
- **The website's colours and faces are the kit's** (2026-11-24): `tools/website.py` writes
  the two themes' roles (`crates/quadra-design/themes`) and the kit's font files before
  `site/site.css`, which names no colour of its own; they were typed by hand and had drifted.
- **The website** (`tools/website.py`, `site/`): an introduction, a guide whose code is quoted
  from compiled files, the design language, and a page for each of the gallery's pages with
  its pictures in both themes and its demos' code (the shots write `pages.json`); a markdown
  twin of every page, `llms.txt`, `llms-full.txt`. Built into `target/site`; not deployed.
- **What was owed is done** (`decisions.md`): OneTerm's dialogs take the kit's layout (no
  widths of their own); the kit writes its distances and radii as tokens; the check reads
  what a call is given; a combobox fills its column; `NumberInput::unit`, `card_head_with`.
- **Against paleness** (the user's decision, `decisions.md`): the primary button is the
  accent, solid, by the theme's roles (its tint while off; a `text` border when focused by
  keys); headlines, card titles and group titles are semibold; what is chosen (a row of a
  list, a tree, a navigation, the settings' pages; a toggle that is on; OneTerm's active tab
  and the dock's picked row) is the accent's tint. Every theme of OneTerm keeps the words on
  the accent at 3 to 1 or more (seven corrected, one test). The gallery and the apps write their distances as tokens; the check follows
  constants across files; a `Measure` carries its own number for a tree with no theme. Nothing of
  the design language is owed.
- **The apps read against the language** (`decisions.md`): OneTerm's small dialogs give the
  body alone in the dialog of 360; hosts, SFTP entries and completions are data rows (24);
  the find bars' query is the md step in the editor, OneTerm and Markdown; Markdown's toolbar
  is the kit's; a select in a settings row is 240 in both apps; a failure in a dialog is an
  alert, and what is wrong with a field is said under it. The apps and the gallery say
  `vstack()`, `hstack()` and `form()` where they wrote them out. A theme file with no
  `accent_tint` gets its own accent at 16 % (37 of OneTerm's themes showed the kit's blue).
- **OneTerm against the app it is to replace** (from 2026-11-24): what a review of the user's
  own terminal app found OneTerm lacks, part by part.
  - An upload onto a name the folder shown already has asks first, Skip or Replace
    (`sftp/upload.rs`; tests/dock_replace.rs). It went over the remote file unasked.
  - Shell integration for the local shells OneTerm finds (`local/integrate.rs`, the setting
    `terminal.shell_integration`, on): Command Prompt, both PowerShells and bash say their
    directory and mark their prompts (OSC 133; PowerShell and bash a command's start and
    its end with the status too), and write UTF-8. tests/shell_marks.rs runs the three
    against real consoles. A bash's own path of a folder (`/c/work`) is taken as the folder.
    Found on the way: a shell found was named with a `/` in its path, which Command Prompt
    read as `/C md .exe` (it made a folder `.exe` and ended); named with the OS's separator.
  - A terminal's history under the hand (`quadra-term` `bar.rs`): the scroll containers' bar
    down the view's right edge (`quadra_view::bar_by_hand`; told where the view is after
    what scrolled it and as the pointer moves over the terminal, never by output or a
    paint), and the keys Shift+PageUp/PageDown and Ctrl+Shift+Home/End, bound again on
    OneTerm's Keybindings page (term tests/bar.rs). A press on any scrollbar's thumb leaves
    the focus where it is.
  - Taking over from OneTerm 0.6 (`import.rs`; specs/19, "Taking over from OneTerm 0.6"): its saved
    sessions as hosts (names made unique, jump hosts by name, forwards, colours) and a few
    of its terminal's settings, once at the user's own start and by Import from OneTerm 0.6 in
    the settings (tests/import.rs).
  - The vault in OneTerm 0.6's format, and its saved passwords taken as they are (the user's
    decision, 2026-11-24; specs/19, "Saved passwords"): Argon2id (`argon2`, specs/01 R15)
    and `otv1:` envelopes bound to the target (`vault/format.rs`), checked against
    OneTerm 0.6's own fixture; no hash of the master password is kept, the vault is what it
    opens, so a start with one is locked. The takeover copies `vault.json` and each
    session's sealed password, so OneTerm 0.6's master password is OneTerm's with nothing asked;
    a vault of OneTerm's own is kept. Breaking for a build of before: its hash, vault and
    saved secrets of the first format are forgotten when the settings are read
    (tests/import.rs, tests/lock.rs).
  - `quadra-updater` (specs/22; the user's decision, 2026-11-24: a crate for any app that
    updates through GitHub): `check` (the latest release, newer, with this build's package
    and digest), `fetch` (checked against the digest GitHub publishes, unpacked by the
    crate's own zip reader over `miniz_oxide`: no program of the OS is started, as not
    every Windows has an archiver), `install` (the files of the folder of the app that runs moved aside and replaced,
    all or nothing), `restart`, `started`. Under it `quadra_shell::http_get` (WinHTTP: the
    system's proxy and certificates) and `sha256` (specs/16, "Fetching"). Tests against a
    server of the test on 127.0.0.1, with a program that runs while its file is replaced.
  - OneTerm's updates (`updates.rs`; specs/19, "Updates"): Settings, About has the group
    Updates, one row whose button is the next step (Check for updates, Install with its
    progress and Cancel, Restart OneTerm) and Check at start, which shows a card for a newer
    version; the releases are those of the repository of the app OneTerm replaces. A saved
    password behind another way in than the password is not taken over (tests/updates.rs).
  - The release asset (`tools/release.sh`; specs/19, "Updates"): the binary is `oneterm.exe`
    (the name the updater of the app before starts), version 0.7.0, with that app's icon
    (`tools/icon_resource.rs`, shared with Markdown's build script); the zip
    `oneterm-0.7.0-x86_64-pc-windows-msvc.zip` holds the folder of that name with the
    console host, LICENSE, NOTICE and the notices of 180 crates, the icons and the faces
    (`tools/notices.py`). Nothing is published by the tools.
  - The title bar's mark is the prompt glyph in the brand's colour `#58C4DC`, as the app it
    replaces has it (the user's correction, 2026-11-24: the picture of the icon, tried
    first, was not what was meant). About names the terminal engine's version from the lock
    file (`build.rs`): it said 0.2 while 0.4.2 was built in.
- **Startup commands are a list of the settings** (2026-11-24, the user's request): Settings,
  SSH & hosts has Startup commands (a name and a command each; Edit, Remove, Save command),
  and a host's dialog chooses one by name in a select where it had a field (`startup.rs`,
  `ssh.startup_commands`). The settings are version 3: a host's own command of before becomes
  a command of the list (`prefs/migrate.rs`). Tests: tests/startup.rs, tests/vault.rs.
- **The folded rail has no line between its tabs** (2026-11-24, the user's request), as the
  open rail has none; a line stands over the first tab and after the last (`rail/narrow.rs`).
- **The status bar's git says the lines added and removed** (2026-11-24, the user's
  request): `+12` in green and `-3` in red after the changed files, from `git diff --numstat
  HEAD` (`git.rs`: `Changes`, `lines`).
- **The network's settings** (2026-11-24, the user's request): `quadra_shell::Network`
  (a proxy named by hand, certificates unchecked) taken by `http_get` and held by `Updater`
  (specs/16, 22); OneTerm's Settings, Advanced has Network (Proxy: System or Manual, Proxy
  address, Verify certificates with `Not checked` while off), read by its updater at each
  check and fetch; the proxy of OneTerm 0.6's updates is taken over. Tests: a proxy named by
  hand is asked in the server's place (`quadra-shell/tests/fetch.rs`), and the updater is
  given what the page says (`apps/oneterm/tests/updates.rs`; no request of the app goes
  through a test's proxy: the OS asks a server of this machine without one, and the updater
  takes plain HTTP from no other). Unchecked certificates are built and not exercised:
  no test has a server with a certificate of its own.
- **Settings are written as objects in objects** (2026-11-24; the user asked why the file
  was flat): a key is still `editor.font_size`, and the file holds `"editor": { "font_size":
  15 }` (`quadra-app`, `settings.rs`: `at`, `place`, `nested`; the values a setting can hold
  moved to `settings/stored.rs`). A file of before, each key whole at the top, opens to the
  same values and is written the new way at its first change; OneTerm's settings are version
  2 and written so at its start. A migration is given the nested object.
- **The app is named OneTerm** (2026-11-24, the user's decision): 1Term everywhere became
  OneTerm (the window, the settings' words, `TERM_PROGRAM`, the specs: `specs/19-oneterm.md`),
  and OneTerm 0.6 names the versions before 0.7 that it takes over from. The folders a build
  of the name before kept (`%APPDATA%` and `%LOCALAPPDATA%`) become the app's at a start
  (`import::adopt`, tests/import.rs).
- **The options' words, and the editor's bar along the bottom** (2026-11-24). The options
  of OneTerm's choices are in a reader's words (`Dot and count`, `Tab only`, `Off`, `OneTerm's
  editor`, `Custom program`); one saved by an old word reads as the default (the user's
  decision). The code editor has the kit's sideways bar while a line is wider than it: the
  widget tells it where the text is and how far it goes in the update pass, never as it
  paints (a frame leaves no work), and its thumb moves the text (code tests/wrap_when.rs).
- **OneTerm, what the walk said to put right** (2026-11-24). A new host key is trusted at the
  step that shows whose it is and its fingerprint: the tick that trusted it unseen is gone
  (tests/connect_host.rs). A changed host key offers Close first (`oneterm_ssh::key_changed`;
  tests/asking.rs). The settings' words are a reader's (a default is "Default"; no row is
  explained by a sequence's number alone). The Advanced page says the file this run saves to
  and shows its folder (tests/settings_reset.rs). Lock after steps as every number does.
  Locked, the status bar's meter says no name (tests/lock.rs). The kit's colour picker says
  a preset by its name and, empty, the placeholder told (`By group`; design tests/pickers.rs).
  The cards over the settings were the harness's alone: the app's settings are a window.
- **OneTerm, a walk through its screens** (2026-11-24; `oneterm-shots` draws each page of the
  settings, `page-0` to `page-10`): the status bar showed a folder alone for a shell with no
  directory and two lines side by side for a remote space not connected; a space header said
  `user@host` twice when the title was that; the lock card's Unlock stood 4 px off its
  field's edge; Fold the rail still said 48 px. Test: tests/bars.rs.
- **OneTerm's editor wraps long lines; the editor is the editor under its last line** (2026-11-24):
  `editor(..).wrap_when(signal)` wraps an editor that shows (rows made again; text, cursors and
  undo kept); OneTerm's header has Wrap and its settings "Wrap long lines" (`sftp.editor_wrap`,
  off at first). The widget is as high as its view at least: under the last line of a short
  file the wheel with Shift scrolled nothing sideways and a press did not reach the editor.
  Tests: `quadra-code` tests/editor.rs and tests/wrap_when.rs, OneTerm tests/editing_menu.rs.
- **OneTerm: a folded tab with no signal has its icon in the middle; a space header's words too**
  (2026-11-24): the hidden line's box kept its gap and lifted the icon 3 px; a line of 18 in
  the header's 17 stood the mono words a pixel low and off the other face's baseline.
- **OneTerm's own heights on the ladders** (the user's decision, 2026-11-24): title bar 38 to
  32, status bar 30 to 32 (the kit's too), a tab of the rail 36 to 28 (52 to 40 while it
  shows), the folded rail 48 by 44 to 40 by 40; the command palette's query row 36 to 32.

### One height for the controls of one line (2026-11-22)

Every control of one line is the md control height, 28 with its border, unless told: a text
input and what is built on one (combobox, colour, path and number inputs, the stepper), the
key input, a segmented group (was 22, its cells now 26 inside the border), beside buttons,
selects and the date and time pickers that were 28 already. A text input was 37 (at least 32
and its padding), the key input and the stepper 32; the Browse button of a path picker is md.
Measured on each control mounted with its defaults (specs/design, Control heights).

### The move loop keeps drawing (2026-11-22)

Holding the caption froze the content: the OS's move loop turns inside `DefWindowProc` from
the press until the button is up, and frames were drawn there only on `WM_SIZE` and
`WM_PAINT`. `SC_MOVE` and `SC_SIZE` now run with a timer of the window that draws what is
pending, paced as the frame loop (specs/10). Seen on a real OneTerm: its clock was drawn again
inside the loop, and was not before. Tests:
`apps/oneterm/tests/signals.rs`, `apps/oneterm/tests/bars.rs`, `quadra-design/tests/small.rs`.

### Performance (2026-11-16)

- **The comparison bench's runner in Rust** (`bench/runner`, `tools/bench.sh compare`), over
  quadra-shell's `process_cycles`, `cycles_per_second`, `private_memory`, `window_shown`.
  Test: `quadra-shell/tests/measure.rs`.
- **The GPU's time only when asked**: `Renderer::measure_gpu` (specs/07); the bench with
  `BENCH_GPU=1`. Test: `quadra-native-win/tests/pixels.rs`.
- **The dist profile**: fat LTO for what ships and is measured (specs/14, "Shipped builds");
  `tools/bench.sh`, `tools/doomfire.sh` and `tools/package.sh` build in it.
- **A node keeps its room** (specs/07): a count of instances that shrinks is padded, so the
  layer is patched in place. Tests: `quadra-view/tests/room.rs`, the empty instance in
  `quadra-native-win/tests/pixels.rs`.
- **What a change costs**: the `cells` scene, `tools/bench.sh incremental` (specs/14), in the
  README with the rest (docs/benchmarks.md, 2026-11-16).
- Open: the editor's paint (444 µs typing, 367 on 2026-11-14); list 4 % over the build of
  2026-11-11 (hover follows a list that scrolls under the pointer).

### After the phases (2026-11-14)

The user's list of what the apps will need, and of what the components lack.

- **The system's appearance**: `App::theme_by(|appearance| theme)`, the kit's
  `install_system(app)`; the gallery starts as the system is. Test: `quadra-design/tests/root.rs`.
- **Drops from other programs**: files and text are a drag of `DroppedFiles` or
  `DroppedText` in the tree (`Tree::os_drag_enter` and the rest); the gallery's page
  `image` shows a picture dropped on it. Test: `quadra-view/tests/drag.rs`.
- **The editor**: soft wrap, links opened with Ctrl, the marks of a diff in the gutter;
  plain text and Markdown in the editor's own colour. The gallery's page `editor` shows a
  note wrapped and marked. Tests: `quadra-code/tests/editor.rs`.
- **Manual checks**: turn the system dark and light with the gallery open; drop a picture
  from the file manager on the page `image`, and text from an editor on nothing.
- **The text size** (specs/12, "Text size"): `TextSize { zoom, text, system }` in the
  context of views (`text_size()`), the system's `Appearance::text_scale`. Zoom draws the
  whole UI larger (`Tree::set_zoom`); text alone is larger by `Tree::set_text_scale`, and
  the kit's boxes of a line of text and the rows of uniform lists grow with it (`by_text`).
  The gallery at 1.5 times the text was looked at (`QUADRA_TEXT=1.5 tools/shots.sh`); at 1
  every picture is the same to the byte. Tests: `quadra-app/tests/text_size.rs`,
  `quadra-view/tests/text_scale.rs`, `quadra-design/tests/text_size.rs`. Manual check: change
  the system's text size with an app open; it follows when the system says a setting
  changed. The gallery zooms with Ctrl-=, Ctrl-minus and Ctrl-0. ponytail: fixed widths
  (the time input, a select) do not grow; a uniform list keeps its scroll in pixels when the
  scale changes; a document's table measures its columns at mount.

### The exit of R4 (specs/00, Phases)

- **Their tests and fuzzers pass**: `quadra-viz`, `quadra-data`, `quadra-md` (its parser
  held by the seeded test of `quadra_testing::fuzz`, its formulas and diagrams) and
  `quadra-code` (the buffer and the document under random edits and commands, every
  tokenizer under the seeded test, the highlighter against reading from the start, finding,
  and the editor driven through keys, text, composition, the clipboard of its context, the
  pointer and its find bar).
- **Their gallery pages match**: the pages of the domain crates differ from the reference
  pictures in 0.7 to 2.2 % of their pixels in both themes, as the kit's do since the
  design language changed by the user's choice (the exit of R3); `editor` 1.12 %. The
  pages with no reference (`diff`, `math`, `diagram`, `image`) were looked at.
- **The editor's budgets** (behaviour/18): a keystroke at the middle of 100,000 lines takes
  a frame of 0.39 ms against 2, scrolling 10 µs a frame, opening ten megabytes the buffer's
  6 ms and one frame (docs/benchmarks.md).
- **Manual checks on a real desktop** (the editor has only been drawn without a window), in
  the gallery's page `editor`:
  1. Composing Vietnamese (Telex) in the editor: the composition underlined at the caret,
     the input method's window beside it.
  2. The caret blinking; a drag that selects, Alt and a drag that selects a column, the
     wheel with Shift sideways on a long line.
  3. Ctrl-F: the query has the focus as the bar appears; Escape gives it back.
  4. Ctrl-/ makes lines comments on the keyboard's layout.

### The exit of R6 (specs/00, Phases)

- **`quadra-design` is built on the headless crate alone**: every component of specs/17's table is a look
  over `quadra-headless`, and what stays in the kit is listed there with the reason. The gate's
  step 5 fails when the headless crate names a colour, a role, a radius or a cursor.
- **The gallery looks as before**: each move was drawn in both themes and compared with the
  pictures taken before the first (`.tmp/same.sh`), byte for byte, and every test of the kit
  passes unchanged but for its imports. The renderer's new paints changed no picture either.
- **A second kit of another look is made from the headless crate without touching it**: `examples/plain`
  (`b7e5177`) changed nothing outside `examples/`, and its tests drive each control through
  the semantic tree.
- **The paints of design systems**: gradients of stops in Oklab, dashed and dotted borders,
  the GPU's time a frame, a blur of what is behind a box, a box cut to any shape. Shaders by
  class wait on the measure, which says no.
- **Manual check on a real desktop**: glass over content that scrolls, in a real window, to
  see that the blocks of the blur stay put while the content moves under them (specs/07).

### Since the exit of R5 (2026-11-14)

The kit, by the user's review: the ends of a segmented control are rounded as its frame
(`.radii(Corners)` on a box), the number input's chevrons are inside it, the stepper's value
is centred (`text_input(..).fit()`) and the gallery shows the stepper, `copyable(text)` in a
description list, `select(..).searchable(ask)`, and `settings_group(group)` on its own; tests
in `crates/quadra-design/tests/small.rs`, `forms.rs` and `settings.rs`.

### The exit of R5 (specs/00, Phases)

- **Done and tested**: the app's folders, settings, window state, the log and crash reports,
  dates and local time, the language and formats by locale, pages, closing; the shell's
  dialogs, opener, tray, one instance and crash reports, behind traits with a fake; the
  gallery packaged with its manifest; the API documentation built by the gate with warnings
  denied.
- **Manual checks on a real desktop**, which no test may do (no real dialog, tray icon,
  notification or crash in a test, and the clipboard never):
  1. Composing Vietnamese with a real input method (Telex) in `crates/quadra/examples/form.rs`.
  2. The dialogs of `WinShell`: open (one and several, filters), save, a folder, the four
     kinds of message; modal over the window that asked.
  3. The tray: the icon, a click, the menu, a notification; the icon gone when the app ends;
     the icon back after Explorer is restarted (from the Task Manager).
  4. One instance: a second start of an app hands the first its arguments and ends.
  5. A crash that is no panic writes a minidump and the log beside the panic reports.
  6. Window state across displays of different DPI, and a place on a display that was
     removed opening centred.
  7. `target/package/gallery/gallery.exe` (`tools/package.sh gallery`) starts, sharp at
     150 %.
- **Not done, by choice**: 1323 public items have no doc comment of their own (variants,
  fields, builder methods; `missing_docs` is not denied). ponytail: they are written as the
  crates are touched, the components' first.

## Notes

- R0: the gate's dependency check, unsafe check and 300-line rule run
  on a workspace of one crate. The release settings in `Cargo.toml` are left at cargo's
  defaults until phase R1 measures them.
- Not carried yet from the V repository: themes, fonts, icons, shaders, test corpora, the
  comparison bench. Each is brought by the phase that first needs it.

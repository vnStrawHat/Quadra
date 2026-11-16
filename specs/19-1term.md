# 19. 1Term

1Term is a terminal app for local shells, and later SSH and SFTP, in `apps/oneterm` (package
`oneterm`, binary `1term`). Started 2026-11-15 by the user's decision, before the other apps.
Its look is the user's design (the canvas "1Term": main window, horizontal tabs variant, the `+`
menu, settings, the lock screen, SFTP), drawn with `quadra-design` in the kit's language; the
user's own terminal app OneTerm (a clone to read in `.cache/vendor/oneterm-app`) is the
reference for behaviour and settings the design leaves open. The terminal is `quadra-term` (18).

## Phases

| Phase | Contents |
|---|---|
| **P1 Local** | the window, tabs and spaces, local shells, the terminal signals, the `+` menu, the status bar, settings, the lock screen |
| **P2 Remote** | SSH over `russh` on a `tokio` runtime of its own with `ring` for its crypto (the user's choice, 2026-11-15; specs/01, R15), saved hosts in the rail, quick connect, the SFTP dock and its expanded view |

P1 milestones, each committed green on `tools/check.sh prod` before the next:

| Id | Contents |
|---|---|
| M1 | the window and its title bar, one local shell filling the workspace, the status bar with the cwd and the clock |
| M2 | tabs in the rail and spaces side by side in a tab, their keys; the horizontal tabs variant |
| M3 | the signals: OSC 7501 status, OSC 9;4 progress, the bell, OSC 9 toasts |
| M4 | the `+` menu (local shells, split), the status bar's git, CPU, memory |
| M5 | settings (pages, saved as they change) and themes |
| M6 | the master password and the lock screen |

P2 milestones:

| Id | Contents |
|---|---|
| M7 | the SSH engine (`apps/oneterm-ssh`): connect, host keys, authentication, the shell; quick connect from the `+` menu |
| M8 | saved hosts: the rail's SSH hosts with a filter, the menu's search, Recent and groups, Add SSH host; the host and its latency in the status bar and the space's header; `n ssh · n local` |
| M9 | the SFTP dock (300 px) and its expanded view: listing, moving about, upload and download, transfers |
| M10 | jump hosts and port forwards |

## Window

Custom frame (`WindowOptions::custom_frame`), 1440×900 at most 85 % of the display, centred,
at least 720×480, its place remembered. Top to bottom: the title bar (38 px), the body, the
status bar (26 px). Colours are the kit's roles; the accent is the theme's.

- **Title bar**: the terminal glyph in the accent and "1Term", 12 px from the left edge (the
  user chose no chip of what shows); then, right, icon buttons Search sessions and hosts (M4; the `+` menu under it), SFTP (P2),
  Notifications with an unread badge (M3), Lock (M6), Settings (M5); the window buttons. In the
  settings only Done shows; locked, only "Locked".
- **Body**: the rail (248 px, vertical tabs), the workspace, and from P2 the SFTP dock (300 px)
  on the right.
- **Status bar**: in mono 11 px, items split by 1 px lines: the active space's cwd (a click
  copies it), git branch and changes (M4), the host and its latency (P2), then right the
  OSC 9;4 meter (M3), CPU and memory of 1Term (M4), one count of bells not heard and notices
  not seen (M3; a click marks the notices seen), `lock in Nm` (M6), the clock `We 28 Oct · 11:00`
  (the app's locale). Labels are the secondary colour, values the text colour.

## Rail (vertical tabs)

248 px on the panel colour (the setting, 140 to 400), a line on its right. Folded (the heading's
Fold the tabs, or the setting) it is 44 px: a box of 32 px a tab with its icon, filled with an accent
bar while it shows, its title as a tooltip that follows it, then a `+` that opens a tab; Unfold the
tabs at the top. A heading "Tabs" with the count and Fold the tabs at its right,
then a row per tab: an icon (a terminal, or a window when a space is remote; in the accent while
it shows), the title, and at the end the tab's signal (below). The active row is filled, has an
accent bar on its left and a second line in the secondary colour (`2 ssh · 1 local · 3 spaces`).
A tab with a progress has a bar of 3 px under its row. Then "New terminal" with `Ctrl+T`, its
border dashed. From P2 the SSH hosts follow, under a filter. The switch to horizontal tabs is in
the settings.

Click selects; middle closes; double-click renames; the right button opens the tab menu (Rename,
Duplicate, Close, Close others). Rows drag to reorder.

**Horizontal variant** (setting "Tabs: vertical | horizontal"): no rail; a 36 px strip under the
title bar, one tab per button (icon, title, then a state on a disc, else the progress in the
accent with a bar along the tab's bottom, else a bell, else for the active one its space count),
the active one on the editor colour with an accent line under it, `+` after the last, "Vertical"
(Switch to vertical tabs) at the end. The title bar's Hosts button opens the rail's hosts as a
panel of 248 px at the left.

## Spaces

A tab holds one or more spaces laid out by a pane tree (the kit's `work::Pane`, `panes`):
splits in two, nested, side by side or one over the other, with the kit's seams between (a
1 px line in a strip of 6 px that is dragged to give a side more room; a double click makes the
sides even). A tab arranges its spaces by itself until the user lays them out: arranged, its
spaces stand side by side (a row of equal columns); beside the rail, three or more stand as the
first on the left (half the width) and the others stacked at its right in equal heights. The
tree is built again by that rule whenever a space is added (Split space, after the focused one)
or closed, and when the tabs move between the rail and the strip. A split to a side, or a seam
dragged to another place, makes the layout the user's: the tree stays as it is, and is changed
only by what is done to it. A split to a side puts a new local shell in the focused space's cwd
beside it, on the side it names, in half of its room, and the focus goes to it; splits along one
axis that hold one another share their room evenly again (three side by side are thirds), and so
they do after a close. Split space in a layout of the user's is a split to the right. A tab back
at one space arranges itself again. The order of the spaces (the focus keys, the status bar) is
the tree's reading order (in an arranged tab, each added space comes after the focused one). The
layout is not saved. Each space has an 18 px header in mono 11 px on the panel colour: where it is on the left
(the cwd, after `user@host:` for a remote space; in the text colour while focused), its signal
on the right (a progress percentage; the state on a 12 px disc and its word in the state's
colour, then the latency; with no state a green dot and the latency, else the program's title).
The terminal has 10 px of room above and below and 12 at its sides. The focused space has a
1 px accent ring, its own border (nothing lies over the terminal for the pointer). Notification cards stand at the workspace's top right, clear of the dock. `Ctrl+D` splits
the active space (Split space: a new local shell in its cwd, after it, with the focus),
`Ctrl+Shift+Right`, `Left`, `Up`, `Down` split it to that side, `Ctrl+W` closes it (the tab with its last
space), `Ctrl+Alt+Left/Right` moves the focus; a press anywhere in a space makes it the focused one. A space whose program ended shows
`[process exited N]` and closes on a key. The other button opens a space's menu (when the
setting has it, the default): Open link and Copy link first when the press was on a link, Copy
(with a selection), Paste, Select all, Clear scrollback, then Split space, Split right, Split
left, Split up, Split down, Input channel, and Close space; a program that reports the mouse takes the press
instead.

Input channels (broadcast typing): five, A to E. A space that joined one sends what is typed in
it to the other members of that channel, in any tab: keys as its view encoded them, typed text
and pastes as each member's own program asked (bracketed or not), through
`Session::on_input` and `Session::deliver` (specs/18, "Input"); mouse reports, focus reports and
replies are not sent. The space's menu has Input channel, a submenu: Channel A to E (the
space's own checked; choosing one joins it, leaving any other), then while a member Leave
channel and Close channel X (every member leaves), then in a tab of more than one space Join
all spaces in tab to X (while a member) and Leave with all spaces in tab (while one is a member).
It is disabled once the space's program ended. A member's header has a chip before its signal:
a 14 px square, radius 3, its bold letter in 10 px on the channel's colour (the kit's
`token::SERIES`, 1 to 5), read as "Channel A"; while it is the focused one of several, its ring
takes that colour. A space made by a split or a duplicate is in no channel; a space leaves its
channel when its program ends or it closes. Membership is not saved.

Links (specs/18, "Input") open on Ctrl+click, or Open link in the menu, through the app's policy,
a pure function of the target and the visible text: `https` opens through the shell's `Opener`;
`http`, a port that is not the scheme's own, or an OSC 8 link whose visible text looks like an
address of another host (a word like "click here" is no address) first asks, in a dialog that
shows the target (and the text when they differ), Open or Cancel; any other scheme, an address
with no host after `//`, a control, space or direction character, a user or password in the
address, or a target over 256 KiB is refused, with a card saying why.

## Find

`Ctrl+F` (the binding `find` of the Keybindings page; bound for the whole window, so the keys
reach a shell only once unbound there) opens a find bar at the top right of the focused space,
over its terminal, and puts the focus in its query; pressed again it gives the focus back to the
query. The bar is the editor's in the kit's pieces: the query (`input`, placeholder and name
"Find"), the switches `Aa` (case) and `W` (whole words) as toggled ghost buttons, the count in
the third text colour ("3 of 12", "No results", nothing while the query is empty), Previous
match, Next match and Close as icon buttons, on the overlay colour with a strong line, the
middle radius and the first elevation. Typing finds at once (specs/18, "Find"); Enter and F3
go to the next match, Shift+Enter and Shift+F3 to the one before, wrapping; Escape or Close
ends finding and gives the focus back to the terminal. Closed, the bar keeps its query: opened
again, it finds it again. Open, it stays open with its query when the space's view is built
again (a tab shown again, the tabs moved to a strip): it is built from the session's finding
(`Session::find_query`).

## Completion

A space completes the command being typed (specs/20, the engine; the app makes one `Engine` in
its model when first asked, so the history is the session's, shared by the tabs). The family of a space is
`force_family`, else `Family::detect` of the local shell's program (the OS's: `cmd`), and Unix
for an SSH space. The overlay follows the terminal's `command_line` (specs/18) and asks
`Engine::suggest(text, text.len(), family)`: nothing while completion is off, on the alternate
screen (`disable_in_alt_screen`), away from a prompt (`require_prompt_region`), without the
caret, or after Escape until the line changes. It stands under the caret at the start of the
token (above it when there is no room below), on the plane of the kit's popups (its fill,
border, radius and level 1 shadow); rows of 23 px in the kit's mono face: the matched part in
the accent's text colour, the rest, the option's hint and the kind (`command`, `subcommand`,
`option`, `history`) in the second text colour; the chosen row filled as a menu's. At most
`max_visible_items` rows show, the window following the chosen row, with `↑ N more` and
`↓ N more` above and below.

Keys while it is open, taken before the shell (`Terminal::on_key_first`): nothing is chosen at
first, so Up, Down and Enter go to the shell; Tab (with `accept_tab`) chooses the first row, and
then accepts; with `accept_tab` off Tab is the shell's and Down chooses the first row. Once a row is chosen, Up and Down (and Ctrl+P, Ctrl+N) move it, and Enter accepts.
Escape closes it. Accepting writes `Suggestion::edit`: a DEL for each character to erase, then
the text; a suggestion that does not extend what is typed (`prefix` false) only closes it.
Enter that goes to the shell records the line first (`Engine::record`, which redacts), when
the line is at a prompt (or `require_prompt_region` is off) and not on the alternate screen:
a password typed for a program is no command.

Settings (Terminal page, group Completion; `completion.*` in `settings.json`): enabled, accept
with Tab, history kept (500, 0 to 5000), characters before suggesting (1, 0 to 10), rows shown
(8, 1 to 20), the history, hand-written catalogs and generated catalogs as sources, fuzzy
matching, the ancestors' options, off in full-screen programs, only at a prompt, Unix commands
in Cmd and PowerShell (off), the shell family (Detected, Cmd, PowerShell, Unix), redact secrets;
all on but where it says. A change applies at once (`Engine::set_config`).

## Signals

| Signal | Tab | Space header | Status bar | Else |
|---|---|---|---|---|
| OSC 7501 status | a mark at the end: blocked a `?` on a warn disc (18 px), done a check on an ok disc (16 px), error a cross on an error disc, working its word; else `idle` | a chip with the state word | | blocked and error raise a toast (setting) |
| OSC 9;4 progress | the percentage in mono and the accent at the end, a 3 px bar under the row | the percentage | a 56 px meter and percentage | |
| BEL in an unfocused space | a red dot on the icon and a bell in the error colour | | the count | |
| OSC 9 notification | | | the count | a toast at the workspace's top right (title "OSC 9", the tab, "now", the text), 6 s or until dismissed (setting) |

Seen clears: selecting the tab clears its bell dot and a "done" seen state.

## The `+` menu

Under "New terminal" (and `+` of the strip): a search field ("Search hosts, or type
user@host…"; at the rail's right, 360 wide), Local shell (the default shell, its name after it,
`Ctrl+T`), the other local shells found (Command Prompt, Windows PowerShell, PowerShell 7, Git
Bash, WSL), Split current space (`Ctrl+D`, Split space); from P2 Quick connect (`ssh user@host`), Recent and
the saved hosts by group, Add SSH host. Rows have icons; a host has a dot (green while a space is
connected to it) and its address at the right (Recent: the host and how long ago).

## M4 in detail

- **Shells found**: Command Prompt (`%SystemRoot%\System32\cmd.exe`), Windows PowerShell
  (`%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe`), PowerShell 7
  (`%ProgramFiles%\PowerShell\7\pwsh.exe`), Git Bash (`%ProgramFiles%\Git\bin\bash.exe`
  `--login -i`), WSL (`%SystemRoot%\System32\wsl.exe`), each where its file is. The default
  shell is Command Prompt until settings (M5) name another. A split starts in the focused
  space's directory.
- **The `+` menu** in P1 has no search field: it searches hosts, which come with P2. With
  them (`searched_menu`), what is typed keeps the shells, recent targets and hosts whose
  names or addresses hold it (whatever the case), and a typed target (`Target::parse`) is a
  row `Connect to user@host`: first when it names a user (`@`), else after the hosts found.
  Enter takes the first row.
- **Git**: the branch from the `.git` folder found up from the focused space's directory
  (`HEAD`; a detached head shows its short hash), and the count of changed files from
  `git status --porcelain` run on the pool with no window, again when the directory changes
  and every 10 s; nothing when there is no repository or no `git`.
- **CPU and memory**: of 1Term's process (`quadra_shell::process_usage`, 16), read every 2 s.

## Settings

A window of its own (980 by 700, its place kept; the title bar's Settings opens it or brings
it to the front; it closes when 1Term locks and with the main window; where no window can open,
the harness, the page shows in the main window with Done), the kit's `settings` in its layout of cards: pages with icons, each with
what it is about, groups with titles; the title bar names the page), a search, pages:
Appearance (and the UI's text size, 75 to 200 %, the terminals' with it: the app's
`TextSize::text`), Terminal, Tabs & spaces (and the tab's title: the program's, a path of a program
shortened to its name, or "Terminal" and the host), SSH & hosts (the count of saved hosts with Add SSH host,
the recent targets with Forget; Connections: the keepalive, on, every 30 s from 5 to 3600, given up
after 3 unanswered from 1 to 20), SFTP (the downloads folder), Notifications & signals, Status bar
(and frames a second, off: `window().frames()` counted each second), and whether the SFTP dock is
open is kept for each kind of tab that shows (a local shell's, closed at first; an SSH space's,
open): it follows the tab, and opening or closing it is remembered for the kind,
Security & lock (Lock now while a password is set; and whether programs may read the clipboard by OSC 52, off), Terminal's
Logging (log new local shells, log new SSH spaces, both off; the folder, typed or chosen in the
system's dialog, empty for `session-logs` in the app's data folder; the formats shown, fixed; append or overwrite; a file `<name>_YYYY-MM-DD_HH-MM-SS.log` a space,
its name the shell's or the host's, lines stamped in local time; a space's menu has Start or Stop
logging, and a log that cannot be opened is a card), Keybindings (a row an action, known by a stable id: `new_tab` Ctrl+T, `split` Ctrl+D ("Split the space", Split space), `split_right` Ctrl+Shift+Right, `split_left` Ctrl+Shift+Left, `split_up` Ctrl+Shift+Up, `split_down` Ctrl+Shift+Down, `close_space` Ctrl+W, `focus_next` Ctrl+Alt+Right, `focus_previous` Ctrl+Alt+Left, `lock` Ctrl+Shift+L, `quick_connect` Ctrl+Shift+K, `settings` Ctrl+, (the Settings window, or the page where none opens), `dock` Ctrl+Shift+B (the SFTP dock), `select_all` Ctrl+Shift+A, `find` Ctrl+F (the find bar, "Find") and `clear` (unbound: the scrollback) on the focused space, `duplicate` (unbound: the tab), `quit` Ctrl+Shift+Q (closes the window), and in a terminal `copy` Ctrl+Shift+C and `paste` Ctrl+Shift+V; each with its `key_input` and Reset (and Reset all at the top), and a warning on each of two actions that have one keystroke. A setting `keys.<id>` a row: absent, the default; empty, unbound; Reset writes the default back. The keymap is built again from the defaults and these whenever one changes and put in `app_keymap()`; a value that is no keystroke is the default. Ctrl+Insert, Shift+Insert and Tab in a terminal stay as they are), Advanced (the file, and a note that settings save as they change), About (the version, the engine, the license).
Saved to `settings.json` in the app's
folder as they change (specs/15). Every setting of a page but Keybindings (which has its own) has the
kit's Reset (specs/13, `settings`), back to the default the app reads it with (`Prefs::load`; the
defaults are those of settings in memory, written once), and each page its Reset all; the master
password, the saved hosts and the recent targets are data, not settings, and have none. Appearance: the theme (System, Dark, Light), and a colour theme over
it (37 of the user's terminal's palettes as files of the kit's theme format, `apps/oneterm/themes`, made by
`.tmp/gen_themes.py` from colours alone; the kit's when none); the terminal's text (font family,
chosen from the families of the machine in a select with a search, the first "The kit's mono" for none; fallback fonts, families separated by commas, at first "Symbols Nerd Font Mono, Symbols
Nerd Font"; size; weight, Normal and Thin to Black; line height, 1.5; the cell's width, 0 for the advance of `0`, up to
48 px; the padding a side, top 0, right 5, bottom 0, left 10, each 0 to 64 px; ligatures, on; semantic
highlighting, on: text in the default colour coloured by what it means, in the colour theme's ANSI colours;
the reference's Auto and On do the same, so it is a switch) and colours over the theme's
(foreground, background, cursor, selection, the gutter's, as `#rrggbb` in the kit's `color_input`, its swatch the
theme's colour while it is empty but for the gutter's; the minimum contrast, 4.5, from 1, off, to 21; the sixteen ANSI colours, a row a
colour with its normal and bright fields), the view's `TermLook` (specs/18). Keys: `terminal.font_fallbacks`,
`terminal.cell_width`, `terminal.padding.top` (`right`, `bottom`, `left`), `terminal.colors.min_contrast`,
`terminal.colors.ansi.red` (`black` to `white`, `bright_black` to `bright_white`), `terminal.ligatures`,
`terminal.semantic_highlighting`:
every terminal follows them as they are, its screen kept. Terminal: cursor shape and
blink, scrollback lines, the default local shell (the OS's, a shell found, or "A program": the
program and its arguments typed, `terminal.shell_program`, `terminal.shell_args`), where new spaces start (active space's cwd,
home, last used); Mouse & wheel: the context menu, copy on select, middle-click paste (all on),
the wheel's speed (1, from 0.1 to 10), alternate scroll (on: arrows to a full-screen program),
the view's `TermSettings` (specs/18). Not yet: "last used", a bell's
sound or flash. A choice is saved as its words. Signals: OSC 7501 shown on tab and header / tab only / ignored, toast on
blocked; OSC 9;4 on tab, status bar; bell: badge and count / badge and sound / badge and flash /
ignore; OSC 9: toast 6 s / until dismissed / silent.

## Lock

Only with a master password (Security & lock: set, change, remove; kept as a salted PBKDF2
hash from the OS's crypto, never the password). Locks after the idle time (10 min default, 1-480)
or at once with `Ctrl+Shift+L` and the title bar's button. Locked: the workspace blurred and
dimmed behind, the title bar usable and saying "Locked", a card in the middle (380 wide on the
panel colour, radius 12: the lock in a box of 48 px, "1Term is locked", "Locked after N min idle ·
sessions are still running", the field labelled Password, Unlock across the card, a rule, then
`n sessions connected` with a green dot and `n new while locked`), a status bar that shows only
"Locked · details hidden", the progress meter and the clock. Five wrong tries start a back-off
(2, 4, 8, 16, 30 s). Sessions keep running.

### M6 in detail

- **The hash** is `quadra_shell::hash_password` (16, "Passwords"), kept in the setting
  `security.password`; set, change and remove ask for the current password when one is set,
  and a new one twice. Hashing and verifying run on the pool.
- **Idle** is no key and no pointer press or move in the window; it is counted in steps of
  10 s on the app's timers, so it locks within 10 s after the time set.
- **Locked, the workspace is not displayed** (the design draws it blurred behind): a node
  that is displayed is in the semantic tree, which any process can read through the OS's
  accessibility, so blurred tab titles and terminal text would still be readable there. The
  rail, the spaces, the settings page and the notification cards are hidden; the title bar
  keeps its window buttons and says "Locked"; the app's keys do nothing but `Ctrl+Shift+L`;
  the card is a focus scope with the focus in the password field. Unlocking gives the focus
  back to the focused space.

## SSH (P2)

The engine is the crate `oneterm-ssh` (`apps/oneterm-ssh`), the one place `russh` and `tokio`
are named: a runtime of two worker threads, made when the first connection starts, behind a
blocking API of the shape of `quadra_shell::Pty`: `Ssh::connect(options, output, exited)`,
`write`, `resize`, `close`. The UI thread never waits on the network.

### M7 in detail

- **Target**: `user@host`, `host:port`, `[user@]host -p port`, `-l user`, and a leading `ssh`
  are read by `Target::parse`; no user is the OS's (`USERNAME`); no port is 22.
- **Prompts are the terminal's**, as the OpenSSH client asks them: what the engine asks is
  written to the space, and what is typed there, up to Enter, is the answer (a password is
  not echoed; Backspace takes back a character; Ctrl+C or Ctrl+D gives up). So a remote
  space is a session like a local one, and its link is the engine's.
- **Host keys**: `~/.ssh/known_hosts` (the path is an option). A known key goes on; an
  unknown one asks `Are you sure you want to continue connecting (yes/no)?` with its SHA-256
  fingerprint, and `yes` writes it to the file; a changed key is refused with a warning that
  names the line, never asked.
- **Authentication**, by the option `Auth`: `Auto` (the default) tries the agent
  (`\\.\pipe\openssh-ssh-agent`, at most 6 keys), then `~/.ssh/id_ed25519`, `id_ecdsa`,
  `id_rsa` where they are (a passphrase is asked), then the password, three times;
  `Agent`, `Key(path)` and `Password` try only theirs. An RSA key signs with the hash the
  server names in `server-sig-algs`.
- **The shell**: a PTY of `xterm-256color` of the space's size, `COLORTERM=truecolor`; a
  resize is a window change; the channel's exit status is the space's `[process exited N]`,
  and a connection that drops is `[connection closed: reason]` and exit status 255.
  Keepalive every 30 s, given up after 3 unanswered. Connecting gives up after 20 s.
- **Quick connect**: the `+` menu's item (`Ctrl+Shift+K`) opens a dialog: a note "Not saved to
  hosts unless you tick it", the target after `ssh` with what it accepts under it, Auth (Auto,
  Agent, Key file, Password) and Jump host side by side, then Open SFTP alongside (on), Save as
  host, "In current tab (new space)"; Connect starts a tab (or a space) whose header says
  `user@host`, and opens the dock when asked.
- **Tests**: a `russh` server in the test process on 127.0.0.1 (no other host, ever):
  password and key authentication, unknown, known and changed host keys, typing, resize, the
  exit status, a connection dropped.

### M8 in detail

- **Hosts** are the setting `ssh.hosts`, a list of `[name, group, target, auth, key file]`
  (target as `Target::label` writes it; auth `Auto`, `Agent`, `Key`, `Password`), so the
  file reads as the page does; `ssh.recent` holds the last 5 targets connected to, newest
  first, with the time in seconds.
- **The rail**, under "New terminal": "SSH hosts" with an Add SSH host button at its right, a
  filter (on the name and the target, whatever the case), then the hosts by group in the order
  first met (a click on a group's heading folds it), each a row of a dot
  (in `OK` while a space is connected to it), its name and its address. A click opens a tab
  connected to it; its menu has Connect, Connect in a new space, Edit… (the dialog of Add SSH host,
  filled, titled Edit SSH host; a new name takes the old one's place), Remove. The Add dialog
  takes the name (the host when empty), the group, the target,
  the auth and the key file. Quick connect has "Save as host".
- **The `+` menu** lists Recent (up to 3, with how long ago) and the hosts by group under
  Quick connect, and Add SSH host. It has no search field: the rail's filter and Quick
  connect take what one types.
- **Latency**: the engine pings the server (an SSH ping, a request that wants a reply) every
  5 s once the shell is open and keeps the last time it took; the app reads it every 2 s into
  the session's `link_note` (`124 ms`). The space's header shows it at its right, the status
  bar shows the focused space's host (its name, else `user@host`) and it, or `local`.
- **A tab's second line** counts its spaces by kind: `2 ssh · 1 local · 3 spaces`.

### M9 in detail

- **The engine's files**: `Ssh::files()` opens SFTP (version 3, `russh-sftp`) on the same
  connection once the shell is open, and gives `Files`, whose calls block (an app makes them
  on its pool): `home`, `list` (folders first, then names whatever the case; no `.` or `..`;
  at most 100,000 entries), `download(remote, local, progress, cancel)`, `upload(local,
  remote, progress, cancel)`, `mkdir`, `remove`, `stat` (a path's properties: folder or file,
  whether it is a symbolic link, the rest of what it points to, size, mode, uid and gid with their
  names read from the server's `/etc/passwd` and `/etc/group` where it lets them be read, times). A transfer writes `<name>.part` beside its
  target and renames it once whole, so a target is never half written; a cancelled or failed
  one removes its part. A remote name that would leave its folder as a local path (`/`, `\`,
  `:`, `..`, a control character) is refused.
- **The dock**: 300 px at the workspace's right, opened and closed by the title bar's SFTP
  button; it follows the focused space's connection. A local space: the header says
  `local space · n/a`, the body why it is empty and a button that shows the first tab with a
  remote space (`Show prod-api-01`). Its header: a folder icon, `SFTP`, the host, a More menu
  (Refresh, New folder…), Expand (a bordered button) and Hide; the path is a breadcrumb in mono
  whose parts go there (the last in the text colour); then `..` and the entries, a folder or file icon, the name,
  the size (`—` for a folder). A click on a folder opens it; two on a file downloads it to
  the user's Downloads folder (`name (1).ext` when the name is taken). The other button gives
  a remote row's menu (Open or Download; for a file Edit…: downloaded to `sftp-edit` in the app's data
  folder, opened in the system's app for it or the program the settings name with its arguments
  (`Opener::open_with`), sent back each time it is saved, a card saying so; above the size set, 1 MB,
  asked first; Rename…, Copy path, Properties…, Delete…) and, where no row is, the
  list's (New folder…, Refresh), in the dock and the expanded view's Remote pane alike; a local
  row's menu there has Open or Upload, Copy path, Refresh. New folder and Rename ask the name in a
  dialog (not empty, `.`, `..` or with a `/`); Delete asks first (a folder only when empty).
  Properties… asks the server for the entry again and shows a dialog, Properties, with a
  description list: Name, Type (`Folder` or `File`, `(symlink)` after a link), Size (`2.4 KB (2458
  bytes)`, `-` for a folder), Modified, Accessed (`2026-10-28 10:52` where the user is), Permissions
  in mono (`-rw-r--r-- (0644)`: the kind, rwx with `s`/`S` and `t`/`T` for the special bits, the
  octal), Owner and Group (`name (id)`, the id alone without a name, `-` without either), Path (in
  mono, with a copy button), and Close. It shows; it does not change the mode. A failed ask is a card.
  Files dropped from the OS upload to the folder shown. Transfers show
  under the list: an arrow, `name → remote` or `name ← local`, its state (`48%`, `done`), a
  cross that cancels, and a bar of 3 px; a failed one says why. Names that start with `.` show only while `.hidden` is on (both views).
- **The expanded view**: the dock's Expand button (and the title bar's Collapse SFTP, back)
  gives SFTP the workspace; the spaces stay mounted and running, not displayed, and the
  title bar's chip adds `SFTP expanded`. Its header: `SFTP`, the host and `ssh session keeps
  running`, Sync cwd with terminal (the remote pane goes to the focused space's directory
  when it is a remote path), `.hidden`, Collapse. Two panes, Local at the left and Remote at
  the right, each with its label, its path in mono, Up and Refresh, a header row (Name, Size
  80 px, Modified 120 px) and its rows; a click selects a row, two open a folder. The Local
  pane starts in the downloads folder (the setting or Downloads). Between them a column of
  56 px: Upload selected, Download selected (into the Local pane's folder, a free name), and
  Compare, which marks a remote file whose local namesake is newer `older` and one that is
  newer `newer`. The transfers under both: `Transfers`, `n active · n done`, a row each with
  `name → folder`, a bar of 200 px, its percentage and the cross. The kit draws all of it
  (tokens, `progress`, `checkbox`, icon buttons); the canvas gives the layout only.
  `Entry::modified` is the server's mtime (seconds; 0 when it has none).
- **Sessions keep app data**: `Session::keep(Rc<dyn Any>)` and `kept::<T>()` (specs/18), how
  the app finds a remote space's `Ssh`.

### The console host

1Term's folder holds `conpty.dll` and `OpenConsole.exe` beside `1term.exe` (specs/01, R15):
`oneterm-vt` uses them before the system's host, which passes about half as much output
(specs/14, "Terminal throughput"). `tools/conpty.sh` fetches the pinned package into
`.cache/conpty`, checks it and copies the two files to `target/debug`, `target/release` and
`target/release/examples`; `tools/package.sh oneterm` puts them in the package with the
license's notice. Without them every space still runs, on the system's host.

### M10 in detail

- **Jump hosts**: `Options::jumps`, at most 4 hops, each a target and its auth. The engine
  connects to the first by TCP, and to each next one through a `direct-tcpip` channel of the
  one before; every hop has its host key checked and its prompts asked in the space like
  the target's. An error on a hop is said `jump host user@host: ...`; the hops close with
  the shell, the target first. A saved host names another saved host as its jump host (a
  sixth word of its row; a host it names that is gone is no jump), shown in the menu as
  `· jump`; Quick connect and Add SSH host choose one among the saved hosts.
- **Local forwards**: `Options::forwards`, each `local port → host:port` as seen from the
  server; the engine listens on 127.0.0.1 and opens a `direct-tcpip` channel for each
  connection. A forward that cannot listen is one line in the space; the shell stays up.
  A saved host keeps its forwards as a seventh word, `8080:localhost:80` joined by spaces,
  typed so in Add SSH host.
  Remote and dynamic forwards wait.

## Tests

The app's tests drive it through the semantic tree on the headless platform with `echo()`
sessions: tabs and spaces made, split, closed, moved; the signals on the rail, headers and
status bar from bytes fed to a session; the menu; completion (`tests/complete.rs`: prompts
marked by OSC 133, the overlay under the caret, the keys, what is written, the history, the gating); find (`tests/find.rs`: Ctrl+F opens the bar, the count, Enter and Shift+Enter, Escape gives the focus back); links (`tests/links.rs`, the shell faked: https opens, http asks and opens on Open, `javascript:` is refused with a card); splits (`tests/splits.rs`: a tab arranged in the rail and the strip, each side, nesting, the order, a seam dragged, a close, Split space in a layout of the user's); input channels (`tests/channels.rs`: join, what a member and a non-member get, a paste bracketed by each, leave, close, an ended program); settings written; lock and unlock. Real
shells only in `quadra-shell`'s pseudo console test.

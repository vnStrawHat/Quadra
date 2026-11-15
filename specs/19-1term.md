# 19. 1Term

1Term is a terminal app for local shells, and later SSH and SFTP, in `apps/oneterm` (package
`oneterm`, binary `1term`). Started 2026-11-15 by the user's decision, before the other apps.
Its look is the user's design (the canvas "1Term": main window, horizontal tabs variant, the `+`
menu, settings, the lock screen, SFTP), drawn with `quadra-design` in the kit's language; the V
specs of OneTerm (V repository, specs 22-27) are the reference for behaviour the design leaves
open. The terminal is `quadra-term` (18).

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

248 px on the panel colour, a line on its right. A heading "Tabs" with the count at its right,
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

A tab holds one or more spaces side by side (a row of equal columns, 1 px lines between);
beside the rail, three or more stand as the first on the left and the others stacked at its
right. Each space has an 18 px header in mono 11 px on the panel colour: where it is on the left
(the cwd, after `user@host:` for a remote space; in the text colour while focused), its signal
on the right (a progress percentage; the state on a 12 px disc and its word in the state's
colour, then the latency; with no state a green dot and the latency, else the program's title).
The terminal has 10 px of room above and below and 12 at its sides. The focused space has a
1 px accent ring, its own border (nothing lies over the terminal for the pointer). Notification cards stand at the workspace's top right, clear of the dock. `Ctrl+D` splits
the active space (a new local shell in its cwd), `Ctrl+W` closes it (the tab with its last
space), `Ctrl+Alt+Left/Right` moves the focus. A space whose program ended shows
`[process exited N]` and closes on a key.

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
Bash, WSL), Split current space (`Ctrl+D`); from P2 Quick connect (`ssh user@host`), Recent and
the saved hosts by group, Add SSH host. Rows have icons; a host has a dot (green while a space is
connected to it) and its address at the right (Recent: the host and how long ago).

## M4 in detail

- **Shells found**: Command Prompt (`%SystemRoot%\System32\cmd.exe`), Windows PowerShell
  (`%SystemRoot%\System32\WindowsPowerShell1.0\powershell.exe`), PowerShell 7
  (`%ProgramFiles%\PowerShell\pwsh.exe`), Git Bash (`%ProgramFiles%\Gitinash.exe`
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

A page in the window (the kit's `settings` in its layout of cards: pages with icons, each with
what it is about, groups with titles; the title bar names the page), a search, pages:
Appearance, Terminal, Tabs & spaces, SSH & hosts (the count of saved hosts with Add SSH host,
the recent targets with Forget), SFTP (the downloads folder), Notifications & signals, Status bar,
Security & lock, Keybindings, Advanced (the file, and a note that settings save as they change).
Saved to `settings.json` in the app's
folder as they change (specs/15). Terminal: font family and size, ligatures, cursor shape and
blink, scrollback lines, the default local shell, where new spaces start (active space's cwd,
home, last used). In M5: size, cursor, scrollback, shell, active cwd or home; the font family,
ligatures, "last used" and a bell's sound or flash wait. A choice is saved as its words. Signals: OSC 7501 shown on tab and header / tab only / ignored, toast on
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
  remote, progress, cancel)`, `mkdir`, `remove`. A transfer writes `<name>.part` beside its
  target and renames it once whole, so a target is never half written; a cancelled or failed
  one removes its part. A remote name that would leave its folder as a local path (`/`, `\`,
  `:`, `..`, a control character) is refused.
- **The dock**: 300 px at the workspace's right, opened and closed by the title bar's SFTP
  button; it follows the focused space's connection. A local space: the header says
  `local space · n/a`, the body why it is empty and a button that shows the first tab with a
  remote space (`Show prod-api-01`). Its header: a folder icon, `SFTP`, the host, a More menu
  (Refresh, New folder), Expand (a bordered button) and Hide; the path is a breadcrumb in mono
  whose parts go there (the last in the text colour); then `..` and the entries, a folder or file icon, the name,
  the size (`—` for a folder). A click on a folder opens it; two on a file downloads it to
  the user's Downloads folder (`name (1).ext` when the name is taken); a row's menu has
  Download and Delete. Files dropped from the OS upload to the folder shown. Transfers show
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
status bar from bytes fed to a session; the menu; settings written; lock and unlock. Real
shells only in `quadra-shell`'s pseudo console test.

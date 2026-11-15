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
| **P2 Remote** | SSH (its engine is decided when P2 starts: R15 admits neither an async runtime nor C built at compile time), saved hosts in the rail, quick connect, the SFTP dock and its expanded view |

P1 milestones, each committed green on `tools/check.sh prod` before the next:

| Id | Contents |
|---|---|
| M1 | the window and its title bar, one local shell filling the workspace, the status bar with the cwd and the clock |
| M2 | tabs in the rail and spaces side by side in a tab, their keys; the horizontal tabs variant |
| M3 | the signals: OSC 7501 status, OSC 9;4 progress, the bell, OSC 9 toasts |
| M4 | the `+` menu (local shells, split), the status bar's git, CPU, memory |
| M5 | settings (pages, saved as they change) and themes |
| M6 | the master password and the lock screen |

## Window

Custom frame (`WindowOptions::custom_frame`), 1440×900 at most 85 % of the display, centred,
at least 720×480, its place remembered. Top to bottom: the title bar (38 px), the body, the
status bar (26 px). Colours are the kit's roles; the accent is the theme's.

- **Title bar**: the terminal glyph in the accent and "1Term"; the active tab's title, `·`, and
  its space count ("3 spaces") in the secondary text colour; then, right, icon buttons Search
  sessions (M4), Notifications with an unread count (M3), Lock (M6), Settings (M5); the window
  buttons.
- **Body**: the rail (248 px, vertical tabs), the workspace, and from P2 the SFTP dock (300 px)
  on the right.
- **Status bar**: in mono 11 px, items split by 1 px lines: the active space's cwd (a click
  copies it), git branch and changes (M4), the host and its latency (P2), then right the
  OSC 9;4 meter (M3), CPU and memory of 1Term (M4), the bell and notification count (M3), the
  time to lock (M6), the clock `Wed 28 Oct · 11:00` (the app's locale).

## Rail (vertical tabs)

248 px on the panel colour, a line on its right. A heading "Tabs" with the count, then a row
per tab: an icon (terminal, or the host's), the title, a second line in the secondary colour
(`2 ssh · 1 local · 3 spaces`, or the program's state word), and at the end the tab's signal
(below). The active row is filled and has an accent bar on its left. Then "New terminal" with
`Ctrl+T`. From P2 the SSH hosts follow, under a filter.

Click selects; middle closes; double-click renames; the right button opens the tab menu (Rename,
Duplicate, Close, Close others). Rows drag to reorder.

**Horizontal variant** (setting "Tabs: vertical | horizontal"): no rail; a 36 px strip under the
title bar, one tab per button (icon, title, its signal), the active one on the editor colour with
an accent line under it, `+` after the last, "Vertical" at the end to switch back.

## Spaces

A tab holds one or more spaces side by side (a row of equal columns, 1 px lines between).
Each space has an 18 px header in mono 11 px on the panel colour: its title on the left (the
program's title, else `user@host:cwd`, cut at its start), its signal on the right (a status chip,
a progress percentage, the latency). The focused space has a 1 px accent ring. `Ctrl+D` splits
the active space (a new local shell in its cwd), `Ctrl+W` closes it (the tab with its last
space), `Ctrl+Alt+Left/Right` moves the focus. A space whose program ended shows
`[process exited N]` and closes on a key.

## Signals

| Signal | Tab | Space header | Status bar | Else |
|---|---|---|---|---|
| OSC 7501 status | an icon at the end (working spinner, blocked `?` in warn, done check in ok, error cross in error); the state word on line 2 | a chip with the state word | | blocked and error raise a toast (setting) |
| OSC 9;4 progress | percentage on line 2 and a 2 px bar under the row | the percentage | a 56 px meter and percentage | |
| BEL in an unfocused space | a red dot on the icon and a bell mark | | the count | |
| OSC 9 notification | | | the count | a toast at the workspace's top right (title "OSC 9", the tab, "now", the text), 6 s or until dismissed (setting) |

Seen clears: selecting the tab clears its bell dot and a "done" seen state.

## The `+` menu

Under "New terminal" (and `+` of the strip): a search field ("Search hosts, or type
user@host…"), Local shell (the default shell, `Ctrl+T`), the other local shells found (Command
Prompt, Windows PowerShell, PowerShell 7, Git Bash, WSL), Split current space (`Ctrl+D`); from
P2 Quick connect, Recent and the saved hosts by group, Add SSH host.

## M4 in detail

- **Shells found**: Command Prompt (`%SystemRoot%\System32\cmd.exe`), Windows PowerShell
  (`%SystemRoot%\System32\WindowsPowerShell1.0\powershell.exe`), PowerShell 7
  (`%ProgramFiles%\PowerShell\pwsh.exe`), Git Bash (`%ProgramFiles%\Gitinash.exe`
  `--login -i`), WSL (`%SystemRoot%\System32\wsl.exe`), each where its file is. The default
  shell is Command Prompt until settings (M5) name another. A split starts in the focused
  space's directory.
- **The `+` menu** in P1 has no search field: it searches hosts, which come with P2.
- **Git**: the branch from the `.git` folder found up from the focused space's directory
  (`HEAD`; a detached head shows its short hash), and the count of changed files from
  `git status --porcelain` run on the pool with no window, again when the directory changes
  and every 10 s; nothing when there is no repository or no `git`.
- **CPU and memory**: of 1Term's process (`quadra_shell::process_usage`, 16), read every 2 s.

## Settings

A page in the window (the kit's `settings`), "1Term Settings · <page>", a search, pages:
Appearance, Terminal, Tabs & spaces, Notifications & signals, Status bar, Security & lock,
Keybindings, Advanced; from P2 SSH & hosts and SFTP. Saved to `settings.json` in the app's
folder as they change (specs/15). Terminal: font family and size, ligatures, cursor shape and
blink, scrollback lines, the default local shell, where new spaces start (active space's cwd,
home, last used). Signals: OSC 7501 shown on tab and header / tab only / ignored, toast on
blocked; OSC 9;4 on tab, status bar; bell: badge and count / badge and sound / badge and flash /
ignore; OSC 9: toast 6 s / until dismissed / silent.

## Lock

Only with a master password (Security & lock: set, change, remove; kept as a salted PBKDF2
hash from the OS's crypto, never the password). Locks after the idle time (10 min default, 1-480)
or at once with `Ctrl+Shift+L` and the title bar's button. Locked: the workspace blurred and
dimmed behind, the title bar usable and saying "Locked", a card in the middle ("1Term is
locked", "Locked after N min idle · sessions are still running", the password field, Unlock,
how many sessions are connected and how many notices came), a status bar that shows only
"Locked · details hidden", the progress meter and the clock. Five wrong tries start a back-off
(2, 4, 8, 16, 30 s). Sessions keep running.

## Tests

The app's tests drive it through the semantic tree on the headless platform with `echo()`
sessions: tabs and spaces made, split, closed, moved; the signals on the rail, headers and
status bar from bytes fed to a session; the menu; settings written; lock and unlock. Real
shells only in `quadra-shell`'s pseudo console test.

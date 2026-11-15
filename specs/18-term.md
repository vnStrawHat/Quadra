# 18. Terminal (`quadra-term`)

The terminal a Quadra app shows: the view of a terminal engine's grid, the keys and the pointer
going back to the program, and the delivery of the program's output. The engine is
`oneterm-vt` (the user's own crate, R15): parser, grid, scrollback, reflow, selection, damage,
key and mouse encoders, the pseudo console. Quadra writes none of that again; behaviour/17 stays
the reference for what the user sees, and its model, parser and limits sections are the engine's
(its `docs/user/limits.md`).

Resumed by the user on 2026-11-15 for the 1Term app (19). It was on hold since 2026-11-11.

## Pieces

| Piece | Where | Role |
|---|---|---|
| `Session` | `quadra-term` | the UI thread's handle on one terminal: the engine behind a lock, the snapshot the view draws, the link, the signals an app reads |
| `Feeder` | `quadra-term` | `Send`: what a transport's reader thread calls with output. It feeds the engine under the lock and returns the replies the transport must write; it posts what the app must hear to the UI thread |
| `Link` | `quadra-term` | what a transport gives a session: `write(bytes)`, `resize(size)`, `close()`. Closures: the transport crate needs no knowledge of this one |
| `terminal(&session)` | `quadra-term` | the view: one widget that paints the rows that show and takes keys, text, the pointer and the wheel |
| `echo()` | `quadra-term` | a link that writes what is typed back as output: tests and the gallery |
| `Pty` | `quadra-shell` (`pty`) | a local shell behind the pseudo console of `oneterm_vt::pty`, and the thread that polls it |

Dependencies: `quadra-term` uses `oneterm-vt` without its default features (no `pty`, no
graphics); `quadra-shell` uses it with `pty` and holds the `unsafe` registration with the poller
(R16). Neither knows the other; the app joins them: the shell's output callback is the
session's `Feeder`, the session's `Link` is the shell's handle.

## Delivery

- A transport's reader thread calls `Feeder::feed(bytes, now)`: the lock is taken, the bytes
  fed, the event batch read, the lock let go. Replies (`VtEvent::Reply`, colour answers, the
  OSC 7501 query's) are returned in order and the transport writes them after what it already
  queued, so a key half written is never cut.
- What the UI must hear goes through one inbox (specs/11): `Output` (a repaint is due: posted
  once while none is pending, so a burst costs one frame), `Title`, `Cwd` (OSC 7, and OSC 9;9,
  which Windows shells send), `Bell`, `Notification`, `Progress`, `Status` (OSC 7501, below; OSC
  133 A applies its prompt rule), and the end of the stream with the exit code. OSC 52 stores are
  dropped until an app setting lets them through. The UI thread drains it, updates the session's signals and marks the
  view for paint.
- The UI thread takes the lock for `snapshot_update` and `palette()` in the update (output,
  a resize, a key, the pointer, the theme), never in paint; drawing reads the snapshot alone (the engine's two-phase hand-off). Synchronized output (`?2026`) is the
  engine's: a held frame is `Unchanged`.

## The view

- One widget, a surface (02): it paints the viewport rows from the snapshot. Cell size from the
  mono face of the kit at the code size: the advance of `0` by the line height, each snapped to
  whole device pixels; padding 8 px around the grid.
- Its size sets the grid: `cols = (width - padding) / cell`, `rows` likewise, at least 1 by 1.
  A change resizes the engine and then the link with the same size, after layout, once a frame.
- Per row: one quad per run of equal background; a glyph per cell at its column (a wide cell
  spans two), from the mono face (bold and italic faces for those attributes); underline and
  strike as quads; inverse swaps, faint is the foreground at 60 %, hidden draws no glyph.
  Colours: the engine's palette with the theme as its base (`set_theme_colors`): 0-15 from
  `token::ANSI`, default foreground, background and cursor from the theme's roles.
- Cursor: a block when focused, an outline when not; bar and underline as the program asks
  (DECSCUSR); it blinks on the kit's caret timer, a paint only. Hidden when the program hides
  it or the viewport is scrolled back past it.
- Selection: the engine's (`Selection`), drawn as a fill under the text in the theme's
  selection role.
- ponytail: every row that shows is painted again on a change; per-row damage
  (`SnapshotUpdate::Partial`) when a full screen's repaint shows in the frame. Box drawing and
  block elements are the font's glyphs; geometry (behaviour/17) when gaps between cells show.

## Input

- Keys: the platform key and modifiers become a `KeyEvent` and `Terminal::encode_key_event`
  turns them into bytes; `None` sends nothing. Text from typing or an input method is a
  `Character` key with its text. A key the app's keymap binds (copy, paste, new tab) is the
  app's first: the view is a focus scope's leaf, and bindings run before it.
- Paste: `Terminal::encode_paste(text)` (bracketed when asked, controls dropped). Copy: the
  selection's text. Both through the clipboard the app provides (specs/09); never the real one
  in a test.
- Pointer: with mouse reporting on and Shift not held, presses, releases, drags and the wheel
  go to the program (`encode_mouse_*`); otherwise a press and drag select (double-click a word,
  triple-click a line), and the wheel scrolls the history (three rows a notch), or sends
  arrows on the alternate screen when the program asked for alternate scroll.
- Typing scrolls back to the bottom. Focus in and out is told to the program when it asked
  (`encode_focus`).

## Signals an app reads

`title`, `cwd`, `bells` (a count), `progress` (OSC 9;4: none, a part, indeterminate, error,
paused), `status` (OSC 7501), `exited` (the code once the program ended), and each OSC 9
notification as an event (`on_notification`). `link_note` is what the link says of itself in
a few words, written by whoever made the link (a remote one's latency, `124 ms`); empty for a
local program. An app keeps its own data with a session (`keep`, `kept::<T>()`): a remote
space's connection, for one.

**OSC 7501** (program status, rev 0.3): routed to the session (`OscRoute::Forward`). The body is
`key=value` pairs joined by `:`: `state` (`idle`, `working`, `done`, `blocked`, `error`, `clear`),
`id` (up to 8 `/` segments of 1-32 of `[A-Za-z0-9_.+-]`, 128 bytes), `title` and `msg` (base64
of at most 192 and 2048 bytes of text without controls, checked and not kept). A report that
breaks a rule is dropped whole. Records by id, at most 64 (the oldest goes); `clear` removes an
id and those under it, or all. The summary is the strongest state: blocked, error, working,
done, idle. OSC 133 A (a new prompt) drops working, blocked and idle. `?` is answered with
`OSC 7501 ; ? ST` in the query's terminator.

## Tests

- `tests/session.rs`: on `echo()`, typing shows the text; output paints glyphs at their cells;
  replies are written in order; a resize reaches the engine and the link; the signals follow
  OSC 0, 7, 9, 9;4, BEL and the end of the program.
- `tests/input.rs`: keys in normal and application cursor mode, Ctrl and Alt; paste bracketed;
  a drag selects and copy gives its text (the harness's clipboard); the wheel scrolls history,
  and goes to a program that reports the mouse.
- `src/status.rs` unit tests: OSC 7501 parsing, records, the summary, the prompt rule, the query.
- `quadra-shell` `tests/pty.rs`: `cmd /c echo` through the pseudo console, its output and its
  exit; skipped when no console can be made.

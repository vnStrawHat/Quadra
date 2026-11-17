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
  whole device pixels; padding around the grid, a value a side (8 px each unless the look says).
- The cell's width can be set (`TermLook::cell_width`, in pixels at the look's size; `None`, the
  advance of `0`): it grows with the text size as the face does, and is snapped the same. A glyph
  stays in the middle of its cells, so a wider cell spaces the text out.
- Its size sets the grid: `cols = (width - left - right) / cell`, `rows = (height - top - bottom)
  / line`, at least 1 by 1.
  A change resizes the engine and then the link with the same size, after layout, once a frame.
  `Session::when_sized(f)` runs `f` once with the grid's size when a view has sized it (at
  once if one has): a transport that tells its program the size only at start (a pseudo
  console) starts there, so a program that reads its size once reads the real one.
- Per row: one quad per run of equal background; a glyph per cell at its column (a wide cell
  spans two), from the mono face (bold and italic faces for those attributes); underline and
  strike as quads; inverse swaps, faint is the foreground at 60 %, hidden draws no glyph.
  Colours: the engine's palette with the theme as its base (`set_theme_colors`): 0-15 from
  `token::ANSI`, default foreground, background and cursor from the theme's roles.
- Minimum contrast (`TermLook::min_contrast`, a ratio of WCAG 2; 1, the default, is off): a
  glyph's colour is moved toward white or black, the nearer that reaches the ratio against its
  cell's background (the most it can when neither does), after inverse and before faint. It is
  worked out where a cell's colours are resolved, once a pair of colours (a small memo the paint
  keeps), never a pixel. Left as they are: colours a program named exactly (24-bit, and the
  palette past 15), the text on a block cursor, and box drawing, block elements, geometric shapes
  and the separators of patched fonts (U+2500 to U+25FF, U+E0B0 to U+E0D7), whose colour is
  their drawing's.
- The look (`TermLook`, a signal given with `Terminal::look`; `font(size, line)` is a look that
  never changes): the family (empty, the kit's mono face), its fallback families, size, line
  height (times the size), weight, the cell's width, the padding (`Edges`: top, right, bottom,
  left), the minimum contrast, and colours that stand over the theme's (`TermColors`:
  foreground, background, cursor, selection, the sixteen ANSI). A new look sets the node's text
  values, the padding and the palette, and sizes the grid again after layout: the view is not
  mounted again, the session keeps its screen.
- Ligatures (`TermLook::ligatures`, off by default; off, a glyph a cell): a row's cells are shaped
  in tokens, so the font's ligatures and contextual forms (on by default in a face, `liga` and
  `calt`) show: a token is a run of narrow cells of one style run, a scalar each, and no blank;
  wide cells, clusters, drawings (as for contrast) and the cell of a block cursor stay a glyph a
  cell. The grid stays: a token's glyphs fall in groups by the character they start at (the
  shaped glyph's `cluster`), and each group is put at its character's column, in the middle of
  the cells it covers (to the next group's column): a ligature of two characters spans their
  two cells. A glyph's colour is its group's cell's. Tokens go through the text system's cache
  of words, so a row that did not change costs lookups, not shaping; nothing is shaped by row.
- Semantic highlighting (`TermLook::highlighting`, off by default): text in the default
  foreground is coloured by what it means, per class (`Highlight`): the prompt's sign, the
  command and its options; the words of errors, success, warnings, information and debugging
  (whole words, any case, a fixed list; `not supported` is an error); paths, IPv4 and IPv6
  addresses, MAC addresses, dates and times, numbers, quoted strings, operators, brackets, URLs
  (`quadra_text::find_urls`), and the permission bits of `ls -l` (type, read, write, execute,
  special, none). A cell a program coloured keeps its colour; URLs and addresses are also
  underlined. The unit is a logical line (rows joined by soft wraps, a viewport's first row
  starts one). Its role comes from the shell's OSC 133 zones on the cells: a line whose first
  marked cell is a prompt, after a line that was not a prompt or that closed its prompt (has
  input cells), is a prompt line (the sign is the last sign glyph before the first input cell,
  or the last non-blank there; what follows is the command); one whose first marked cell is
  output is output; anything else, or the viewport's first line, is told by the prompt's look
  (`user@host:~$ `, `[user ~]# `, `C:\path>`, `PS C:\path>`). On a prompt line the path in the
  prompt is a path, the first word after the sign is the command (again after `;`, `|`, `&&`,
  `||`), words with `-` are options, the rest is left. On an output line the matchers run in
  order, each only on what the ones before left: strings, permissions, keywords, MAC, IPv6,
  dates and times, IPv4, paths, numbers, options, operators and brackets; URLs over all. After a
  command ended (OSC 133 `D` with a status), while the viewport is at the bottom, the sign of the
  prompt above the newest one takes success's colour for 0, error's otherwise. Colours:
  `TermColors::highlights`, a colour a class; unset, an ANSI colour of the palette each class
  names (`Highlight::ansi`), so a colour theme colours it too. Cost: classes are kept by row id,
  with the row's stamp, its line's rows and the mark of the line above; a paint scans only lines
  whose key changed, and a line with no default-foreground character is not scanned.
- Fallback families (`TermLook::fallbacks`, tried in order for a character the face lacks, before
  the system's; specs/08): the text system has one list for the whole app, so a view sets it
  when its look's list differs from the one it set last (at first none, so a view with no list
  leaves `App::font_fallbacks` as it is), and the text system leaves its words alone when the
  list is the one it has. Setting them empties the shaping cache; the view shapes its cells as it
  paints, so its repaint shows the new faces. The last list set wins over `App::font_fallbacks`.
- The gutter (`TermLook::gutter`, off by default): a column at the left of the grid with, per
  row that shows, the time the row was first written and its line number, `[HH:MM:SS] 12`,
  in the mono face: the number right-aligned in `digits` cells after one, so it ends 4 px
  before the grid. Its width is whole cells, `11 + digits` of them, and 8 px (4 each side);
  `digits` is those of the highest line number shown so far: nothing is kept ahead, it
  grows when a line number needs more (at line 10, 100, 1,000) and the grid is sized again
  then. The grid, the pointer's cells and the caret start after it. Colours
  (`TermColors`): `gutter_fg` (else the foreground mixed half with the background), `gutter_bg`
  (else the terminal's background), `clock_fg` and `line_number_fg` (else `gutter_fg`).
- Times live with the engine, stamped by the feed with its `now` (so a view not shown keeps
  them): seconds after the first stamp, a deque for the primary screen's rows from the id of
  its first row. A row is stamped once, when the cursor reaches it or a row below it has
  content, and never again (a prompt keeps the time it was drawn); rows that leave the history
  drop their stamps; `ScreenCleared` drops the stamps of the screen's rows; a shrink drops those
  past the newest row. A column change gives every row a new id: the stamps are kept, aligned
  to the bottom (told from a burst of output by the lines the engine produced since). A row with
  no stamp shows the newest, one above the first the oldest; `[--:--:--]` when nothing was
  stamped. Line numbers count from the top of the history: the oldest row kept is 1, so they
  move once the history is full, and a clear of the history starts them again. On the alternate
  screen the gutter is empty. The clock is the local time: one anchor (an instant and the
  system's clock at the platform's offset) taken at mount turns instants into times.
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
- Settings (`TermSettings`, a signal the app gives with `Terminal::settings`, read as events
  come): `copy_on_select` (a selection is copied when the button is released), `middle_paste`
  (the middle button pastes, unless the program takes it), `scroll` (times the rows of the
  wheel), `alternate_scroll` (on the alternate screen the wheel sends arrows even when the
  program did not ask). A right press the program does not take is left to what is around the
  view: the app's context menu (`quadra_design::context_menu` around the terminal).
- What the user sends, for an app that repeats it elsewhere (input channels, specs/19):
  `Session::on_input(f)` (one hook; another replaces it) is told each `Input` after it is
  written: `Keys(bytes)` as the view encoded a key, typed text or Tab, `Text(text)` typed or
  committed by an input method, `Paste(text)` from the view's paste or `Session::paste`;
  `Session::send(&input)` writes what an app types for the user (an accepted completion) and
  tells it too. The
  mouse, focus reports, replies, the wheel's arrows and what `deliver` writes are not told.
  `Session::deliver(&input)` writes one to another session as its own: keys as they are, text
  and a paste encoded by its engine (a paste bracketed only when its program asked), and the
  view back at the bottom; its hook is not told, so two sessions never echo each other.
- What a menu does: `Session::selection_text`, `paste(text)`, `select_all`, `clear_history`
  (the scrollback, as ED 3 would).
- OSC 52: a program's text for the clipboard (`c`) is written to it, always; a program that asks
  for it (`?`) is answered (`52;c;<base64>` and BEL) only when `TermSettings::clipboard_read`
  says so, off by default. The view gives the session its clipboard and settings as it mounts.
- Links: an OSC 8 hyperlink (the engine's id on each cell, its target from the snapshot) or a
  URL in the text (`quadra_text::find_urls` over the logical line, so a URL that wraps is one
  link), looked for in the snapshot the view drew last. An OSC 8 link runs over the cells of its
  id along the logical line. With `Terminal::on_link(f)`, a press of the left button with Ctrl
  on a link calls `f(TermLink { url, text })` and does nothing else: no selection, and neither
  the press nor its release goes to a program that reports the mouse. `url` is an OSC 8 target
  as the program gave it (no scheme is filtered here: what opens, what asks first and what is
  refused is the app's); `text` is an OSC 8 link's visible text when it is not its target.
  While Ctrl is held over a link (the pointer moving, or Ctrl pressed or released with the view
  focused), its cells are underlined and the pointer is a hand. The link is looked for only
  while Ctrl is held, and again only when the cell or Ctrl changed; a change paints the view
  (it has no paint of its own by row) without taking a new snapshot. Without `on_link` the
  pointer behaves as before. `Session::link_pressed()` is the link under the last press of the
  other button (none after a press of another), for the app's menu.

## The command line

What any completion needs, with no knowledge of commands (specs/20 is 1Term's):

- `Session::command_line()` is a signal of `CommandLine { text, at_prompt, alt_screen, caret }`,
  followed from the first call on: after each burst of output, a focus change and a resize it is
  read again under the lock and set when it differs. `text` is what is typed, from where the
  input starts to the cursor. With OSC 133 (a shell that sent `B` once) the input starts after
  the last prompt cell (`A` to `B`) on the cursor's row, or the first of the rows that wrap into
  it, and `at_prompt` says the pen is in the input zone (`B`, no `C` since). A shell that marks
  nothing: the row up to the cursor after the first prompt sign (`>`, or `$ # ❯ ➜ λ` followed by
  a space or the cursor), and `at_prompt` says one was found; else the row, not at a prompt.
  `alt_screen` is the alternate screen being up (then the text is empty). `caret` is the cursor's
  cell in the view's box (its width a cell), while the view has the focus and shows the bottom.
- `Terminal::on_key_first(|key, mods| bool)` is asked before the program for each key press the
  view would send, Tab and Shift+Tab with them: true takes the key, and nothing is written. Typed
  text is not asked; it goes to the program as before. An overlay of suggestions takes Up, Down,
  Tab, Enter and Escape this way while it is open.

## Find

- `Session::find(&FindQuery { text, case, word })` looks for `text` in the active screen, its
  history with it, through the engine's matcher (`vt::search`: a copy of the text taken under
  the lock, matched after it; a literal, lower-cased unless `case`, and only whole words with
  `word`; a match never crosses a hard line end, a wrapped line is one). No regular
  expressions: the engine's are behind a feature with a dependency of its own. An empty text
  ends finding. The current match is the first at or below the top of the view, else the last
  one; `find_next(back)` steps to the next (or, `back`, the one before), wrapping. The view
  scrolls to the current match when it is out of view, so that it stands in the middle (as
  near as the history allows); a match in view does not move it.
- `Session::found()` is a signal of `(current, count)`, the current counted from 1; `(0, 0)`
  when nothing is found or nothing is looked for. At most the engine's `MAX_MATCHES`.
  `find_query()` is what is looked for.
- Cost: matches name rows by id, so scrolling and output that only scrolls keep them right
  with no new search. The text is matched again on a new query, after a resize (a reflow
  renumbers rows), and after output, at most once in 150 ms (a timer started by the first burst
  while one is looked for); never in a paint. A search again keeps the current match where it
  still is, and does not scroll.
- Painting: each match is a quad of the theme's `editor_find_match` over the cells it covers
  (on each row of a match that wraps), the current one `editor_find_current`, above the cells'
  backgrounds and under the selection and the text. A row finds its matches by a binary search.

## Logging

- `Session::start_log(path, append, stamp)` logs the output from then on: the folder is made,
  the file opened after what it holds or over it. The feeder's thread runs the engine's own
  parser over the same bytes and keeps the printable text: a line ends at CR or LF, backspace
  takes a character back, sequences are dropped; each finished line is written as
  `[stamp()] text` and flushed. `stop_log` writes the line not ended and closes the file;
  `logging()` says whether one is open (false again after a write failed).

## Signals an app reads

`title`, `cwd`, `bells` (a count), `progress` (OSC 9;4: none, a part, indeterminate, error,
paused), `status` (OSC 7501) and `status_name` (the id of the record the summary is, the latest
of the strongest: `deploy`), `exited` (the code once the program ended), and each OSC 9
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
- `tests/line.rs`: the command line from fed OSC 133 marks and from a prompt's sign, the
  alternate screen, the caret, keys taken first.
- `tests/ligatures.rs`: on, a token is shaped once and its glyphs keep their columns (a cell
  wider than the advance included); off, a glyph a cell; a unit test places hand-made glyphs (a
  ligature over two cells, glyphs of their own clusters).
- `tests/highlight.rs`: words, paths, numbers and URLs take their class's colour, a coloured
  cell keeps its own, a prompt by OSC 133 and by its look, the exit status on the sign, off
  paints as before, and an unchanged screen scans nothing again; unit tests of the scanner.
- `tests/look.rs`: a new look sizes the grid again in the same view; the padding a side, the
  cell's width and the fallback families reach what is drawn; minimum contrast moves a glyph's
  colour until the ratio holds and leaves a 24-bit one alone.
- `tests/input.rs`: keys in normal and application cursor mode, Ctrl and Alt; paste bracketed;
  a drag selects and copy gives its text (the harness's clipboard); the wheel scrolls history,
  and goes to a program that reports the mouse.
- `tests/links.rs`: an OSC 8 link (its visible text when it differs) and a plain URL, one that
  wraps, Ctrl+click calls the handler and selects nothing nor reaches a program that reports the
  mouse, a click without Ctrl selects as before, the hand and the underline while Ctrl is held,
  the link under a press of the other button.
- `tests/find.rs`: matches in history and on screen, case and whole words, the count, next and
  previous wrapping, the view scrolled to a match in history and left alone for one in view,
  output found again after the wait with the current match kept, the quads painted, an empty
  text ending it.
- `src/status.rs` unit tests: OSC 7501 parsing, records, the summary, the prompt rule, the query.
- `quadra-shell` `tests/pty.rs`: `cmd /c echo` through the pseudo console, its output and its
  exit; skipped when no console can be made.

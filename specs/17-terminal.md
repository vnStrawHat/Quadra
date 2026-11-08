# 17. Terminal (`quadra/kit/term`)

Implementation spec for the terminal of specs/16. The budgets there set the data layout.

## Layers
| Piece | Where | Role |
|---|---|---|
| `Cell`, `Screen`, `Scrollback` | `kit/term` | the model: grid, cursor, modes, history |
| `Parser` | `kit/term` | VT500 state machine, drives a `Screen` directly |
| `Session` | `kit/term` | screen + parser + PTY + selection; the model a view shows |
| `terminal(session)` | `kit/term` | one element that paints the visible rows and handles input |
| `platform.Pty` | `quadra/platform` | `write`, `resize`, `close`; made by `Platform.spawn_pty` |
| ConPTY | `quadra/platform/win` | pseudo console + reader thread (the only C code) |
| `FakePty` | `kit/term` | scripted output and echo, for tests and the gallery |
| `core.Inbox` | `quadra/core` | bytes from any thread, drained once per wake on the main thread |

## Data layout
- **Cell**: 16 bytes: `ch u32` (code point, 0 = blank), `fg u32`, `bg u32`, `flags u16`, `width u8`, one spare byte. A color is `0` (default), `1 + index` (0–255 palette) or `0x1000000 | rgb` (true color). Flags: bold, faint, italic, underline (single, double, curly), blink, inverse, hidden, strike, wide, wide-spacer, wrapped (row continues on the next).
- Combining marks attach to the previous cell through a side table keyed by (row id, column); they are rare.
- **Screen**: a fixed `rows × cols` grid of `[]Cell`, each row with a generation number bumped on change (the view caches by it).
- **Scrollback**: a ring of rows trimmed to the last non-blank cell, default 10 000, capped by `max_lines`. 100 000 lines of 80-column shell output stay under 40 MB.
- The alternate screen has its own grid and no scrollback.

## Parser
- Byte loop, no allocation per byte; printable ASCII in the ground state takes a fast path that writes runs of cells.
- UTF-8 is decoded in the parser; invalid bytes print U+FFFD.
- Supported: C0 (BEL, BS, HT, LF, VT, FF, CR), ESC (7, 8, D, E, M, c, =, >), CSI (CUU/CUD/CUF/CUB/CNL/CPL/CHA/CUP/HVP/VPA, ED, EL, ECH, ICH, DCH, IL, DL, SU, SD, DECSTBM, SGR, DSR 6, DA, DECSET/DECRST, SCOSC/SCORC), OSC 0/2 (title), OSC 52 (clipboard, off by default); OSC 8 is consumed (the text shows, links are not clickable yet).
- DEC private modes: 1 (application cursor keys), 7 (auto wrap), 25 (cursor visible), 1000/1002/1006 (mouse, SGR encoding), 1049/47/1047 (alternate screen), 2004 (bracketed paste).
- Replies (DSR, DA) go back to the PTY. Unknown sequences are consumed and ignored.
- Wide characters (East Asian wide, emoji) take two cells; a table of ranges decides width.

## Resize
- Columns change: the normal screen reflows wrapped lines (rows marked `wrapped` join and re-split). The alternate screen truncates or pads.
- Rows shrink: lines above the cursor move to scrollback. Rows grow: lines come back from scrollback.
- The PTY is resized after the grid.

## View
- One element. Paint walks the visible rows. Backgrounds are merged into one quad per run of equal `bg`. Text is one `layout_line` per run of equal style, placed at `col × cell_w`. A non-ASCII character is its own run at its column, so fallback fonts never shift the grid.
- Cell size comes from the mono face: advance of `M` × line height (theme `code` size).
- Colors: palette 0–15 from `theme.code.terminal.ansi`, 16–255 from the xterm cube and gray ramp, default fg/bg/cursor/selection from the theme. Inverse swaps; faint is fg at 60 %.
- Cursor: block when focused, outline when not; bar and underline via DECSCUSR. It blinks through `request_paint_after` (no re-render). The wheel and shift+PgUp/PgDn scroll into the history; typing returns to the bottom, and new output keeps a scrolled-back view on the same lines.
- Selection by cell: drag selects, double-click selects a word, triple-click a line, shift-click extends. `ctrl-shift-c` copies (lines joined, wrapped rows without a newline), `ctrl-shift-v` pastes (bracketed when mode 2004 is set).
- Rows are not cached: painting a full 230×64 screen takes 0.55 ms (bench/term), well inside the 2 ms budget.
- Keys: printable text as UTF-8; Enter `\r`, Backspace `0x7f`, Tab, Esc; arrows `ESC [ A` (or `ESC O A` in application mode); Home/End/PgUp/PgDn/Insert/Delete/F1–F12 as xterm; ctrl+letter as C0; alt prefixes ESC. Mouse reporting (SGR encoding) sends press and release when a mode asks for it, except with shift held (shift keeps local selection); motion (1002) is not reported yet.

## Delivery
- The PTY's reader thread pushes chunks into a `core.Inbox`. The inbox posts one drain to the main thread while none is pending; the drain swaps the buffer, feeds the parser and marks the session changed. Output bursts therefore cost one render per frame at most.
- `FakePty` pushes synchronously, so harness tests need no waiting.

## Tests
- `kit/term/*_test.v`: table-driven parser and screen tests (bytes in, screen text, cursor and cell attributes out), reflow tests, scrollback trimming and size, key encoding.
- `tests/term_test.v`: the view in the harness on a `FakePty`: typing echoes, output renders, wheel scrolls history, selection and copy to the headless clipboard, paste with bracketing.
- `bench/term`: parse throughput (plain text and SGR-heavy) and a 200×60 repaint.
- ConPTY smoke test (`tests/conpty_test.v`): spawns `cmd /c echo`, skipped when no console session is available.

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
- **Cell**: 16 bytes: `ch u32` (code point, 0 = blank), `fg u32`, `bg u32`, `flags u16`, `width u8`, one spare byte. A color is `0` (default), `1 + index` (0–255 palette) or `0x1000000 | rgb` (true color). Flags: bold, faint, italic, underline (single, double, curly, dotted, dashed), blink, inverse, hidden, strike, wide, wide-spacer; the top two bits are the OSC 133 zone of the cell (none, prompt, input, output). `pad` is 1 + an index into the screen's interned extras table (OSC 8 link, SGR 58 underline color, SGR 53 overline), so one id per cell covers all of them.
- Rare per-row data lives behind one pointer, `Row.extra` / `Line.extra`: OSC 133 marks with the D exit code, and the Sixel images that cross the row. It moves with the row through scrolling, `IL`/`DL`, the scrollback and reflow.
- Combining marks attach to the previous cell through a side table keyed by (row id, column); they are rare.
- **Screen**: a fixed `rows × cols` grid of `[]Cell`, each row with a generation number bumped on change (the view caches by it).
- **Scrollback**: a ring of rows trimmed to the last non-blank cell, default 10 000, capped by `max_lines`. 100 000 lines of 80-column shell output stay under 40 MB.
- The alternate screen has its own grid and no scrollback.

## Parser
- Byte loop, no allocation per byte; printable ASCII in the ground state takes a fast path that writes runs of cells. In the ground state a CSI sequence or a UTF-8 sequence that lies whole in the chunk is parsed in one call (`fast.v`), SGR ahead of the CSI dispatch; a sequence split across chunks, or a CSI with a control or a string byte inside, goes through the byte loop, which leaves the same state. The print path keeps the width of the last code point, so a run of one character (box drawing, blocks) skips the width tables.
- UTF-8 is decoded in the parser; invalid bytes print U+FFFD.
- Supported: C0 (BEL, BS, HT, LF, VT, FF, CR, SO, SI), ESC (7, 8, D, E, H, M, Z, c, =, >, n, o, N, O, charset designators for G0 to G3, `# 8`), CSI (CUU/CUD/CUF/CUB/CNL/CPL/CHA/CUP/HVP/VPA, ED, EL, ECH, ICH, DCH, IL, DL, SU, SD, DECSTBM, SGR, DSR 5/6, DECXCPR, DA1/2/3, DECSET/DECRST, DECRQM, DECSTR, SCOSC/SCORC, HTS/TBC/CHT/CBT, `? 5 W`, REP, DECSCUSR, `t` 14/18/22/23, kitty keyboard (`? u`, `= u`, `> u`, `< u`), modifyOtherKeys (`> 4 m`, `? 4 m`), XTVERSION, DECRQCRA), DCS (Sixel, DECRQSS, XTGETTCAP), OSC 0/1/2, 4, 7, 8, 9, 9;4, 10/11/12, 17, 19, 22, 50, 52, 104, 110/111/112, 133, 7501.
- SGR: 1, 2, 3, 4 (with 4:0-5), 5, 6 (one blink flag), 7, 8, 9, 21 (cancel bold), 22-25, 27-29, 30-37, 38 (5 and 2, semicolon and colon forms), 39, 40-47, 48, 49, 53, 55, 58, 59, 90-97, 100-107.
- DEC private modes: 1, 3 (acts, reports not supported), 5 (reverse video), 6, 7, 9 (X10 mouse), 12, 25, 45 (reverse wrap), 47, 1000, 1002, 1003, 1004 (focus reports), 1005, 1006, 1007, 1015, 1042, 1047, 1048, 1049, 2004, 2026 (synchronized update), 2027 (grapheme clusters), 9001 (win32 input, accepted and inert). ANSI modes: 4 (IRM), 20 (LNM, tracked and inert).
- Mode 2026 (synchronized update): while it is open the parser holds the bytes back and the screen stays at the last complete frame; `? 2026 l` releases them, and so does a 150 ms timeout.
- Replies go back to the PTY in byte order. DA1 is `CSI ? 62 ; 4 ; 22 c` (VT220, Sixel, ANSI color). XTVERSION names `Screen.product_name`. Unknown sequences are consumed and ignored.
- Wide characters (East Asian wide, emoji) take two cells; tables of ranges (`width_table.v`, generated from Unicode's East Asian Width and general categories) decide width, per scalar by default, matching `PSEUDOCONSOLE_GLYPH_WIDTH_WCSWIDTH`. C1 controls decoded from UTF-8 print nothing. With mode 2027 set the print path measures grapheme clusters: ZWJ sequences, skin tones and flags share one wide cell, VS16 widens the base and VS15 narrows it.
- A string (OSC, DCS, APC) ends with ST, BEL or ESC; CAN and SUB abort it. DCS payloads are capped at 16 MiB, query payloads at 8 KiB.

## Sequences the app reads (TerminalInfo)
The engine stores what it parses on `Screen` and queues `Screen.events`; the view drains the queue after every chunk and emits one `TerminalInfo` per event. The fields stay on the screen for code without a view.

| Sequence | Stored | `TerminalInfo.kind` |
|---|---|---|
| OSC 133 A, B, C, D | marks on the row, the pen's zone, `prompt_count`, `last_exit` | `.shell_mark` |
| OSC 9 | - | `.notification` (control characters stripped, 8 KiB, at most 10 per second) |
| OSC 9;4 | `progress` | `.progress` |
| OSC 7501 | a table of up to 64 records by `id`, its summary in `program_status` | `.program_status` (a report that breaks a rule is dropped whole; OSC 133 A drops working, blocked and idle) |
| OSC 1 | `icon_name` | `.icon_name` |
| OSC 22 | - | `.pointer` |
| title, cwd, and the like | as before | `.changed` |

Queries: `Terminal.zone_at(pos)`, `Terminal.row_zone(line)`, `Terminal.input_start(line)` (first input column of a row, -1 when none), `Terminal.in_command_input()` (the cursor is in the input region: pen zone input, main screen), `Terminal.shell_marks_at(line)` (marks on a row, with the exit code).

## Dynamic colors
`Screen.over` holds overrides for palette 0-255 (OSC 4), foreground, background, cursor (10, 11, 12), selection background and foreground (17, 19). Resets are 104 (all or by index), 110, 111, 112 (17 and 19 have none); RIS clears the whole table. `?` queries go through a queue answered after the chunk by the view, which owns the theme fallback, in `rgb:RRRR/GGGG/BBBB`, ended like the question. The grid paints with `Screen.over` over the theme. Reverse video (mode 5) swaps the default foreground and background at paint time.

## Sixel
DCS `q` opens a decoder (`sixel.v`, VT340 palette and semantics, registers 0-255, up to 4096 px each way). When the string ends the image goes to `Screen.graphics`, and every row it crosses gets an `ImageRef` in `Row.extra`: which image, which part. The cursor moves down the image's height in bands of six pixels, scrolling like line feeds. Cell pixels come from the view (`Screen.cell_px`), default 10x20. A graphic dies when no row refers to it any more; the view then reuses its atlas tile. Limits: 256 images and 64 MiB of pixels per screen, oldest out first; the view downscales past 2047 px to fit the atlas.

## Mouse and keyboard protocols
Mouse: X10 (press only), normal, button and any-motion reporting, in the default, UTF-8 (1005), SGR (1006) and urxvt (1015) encodings; `? 1004` sends `CSI I`/`CSI O` on focus change. Keyboard: the kitty flag stack (progressive enhancement, per screen), modifyOtherKeys, otherwise the legacy table. Release events go through `Div.on_key_up`, only for a key whose press was sent.

## Resize
- Columns change: the normal screen reflows wrapped lines (rows marked `wrapped` join and re-split). The alternate screen truncates or pads.
- Rows shrink: lines above the cursor move to scrollback. Rows grow: lines come back from scrollback.
- The PTY is resized after the grid.

## View
- One element. Paint walks the visible rows. Backgrounds are merged into one quad per run of equal `bg`. Text is one `layout_line` per run of equal style, placed at `col × cell_w`. A non-ASCII character is its own run at its column, so fallback fonts never shift the grid. `Behavior.fallbacks` are tried, in order, before the system fallback for a glyph the terminal font lacks; `Behavior.ligatures: false` shapes the family without contextual alternates (`text.font_features`); `Behavior.features` adds OpenType features to the same call, an entry beating `ligatures` for `calt`.
- Box drawing, block elements, U+25AC, braille and powerline (U+2500-259F, U+25AC, U+2800-28FF, U+E0B0-E0BF) are geometry, not glyphs (shapes*.v): computed once per (character, device cell size, stroke weight) into device-pixel rects, coalesced across touching cells of a row, multiplied into the text color. A bold cell draws them 300 weight heavier. Strokes meet across cells.
- Glyphs of a run are placed on the cell grid (cell width snapped to device pixels), so `layout.cell_width` and a font narrower or wider than the snap never drift across a run.
- Text keeps a minimum WCAG contrast against its background (`Behavior.min_contrast`), except text colored exactly by the program (palette 16+, 24-bit) and decorative characters (U+2500-25FF, the powerline symbols).
- Cell size comes from the mono face: advance of `0` (or `Behavior.cell_width`) × line height (theme `code` size), each snapped to whole device pixels; the text origin is snapped too. Stroke positions, widths and quad edges are exact in device pixels.
- Colors: palette 0–15 from `theme.code.terminal.ansi`, 16–255 from the xterm cube and gray ramp, default fg/bg/cursor/selection from the theme. Inverse swaps; faint is fg at 60 %. Underlines draw in the SGR 58 color (or the text color): single, double, curly (a stroked sine wave), dotted and dashed; SGR 53 draws an overline.
- Cursor: block when focused, outline when not; bar and underline via DECSCUSR. It blinks through `request_paint_after` (no re-render). The wheel and shift+PgUp/PgDn scroll into the history; typing returns to the bottom, and new output keeps a scrolled-back view on the same lines.
- What an app can change (font, padding, cursor, wheel speed, copy on select, bell) is `Behavior` (specs/22). Links, search and the mouse modes 1003 and 1007 are there too.
- Selection by cell: drag selects, double-click selects a word (`select_expand.v`), triple-click a line across soft wraps, shift-click extends; a selection is anchored to its rows (Screen.sel): a row leaving the history or scrolled out of a region clears it, IL/DL/SU/SD move it, a width change clears it, and EL/ED/alternate-screen/RIS clear it by the rows they blank (EL: the cursor row; ED 0: from the cursor down; ED 1: from the top to the cursor; ED 2 and 3 always). `ctrl-shift-c` copies (lines joined, wrapped rows without a newline), `ctrl-shift-v` pastes (bracketed when mode 2004 is set).
- Rows are not cached: painting a full 230×64 screen takes 0.55 ms (bench/term), well inside the 2 ms budget.
- Keys: the kitty and modifyOtherKeys protocols take their keys first (printable keys too with kitty flag 8, which then suppresses the text event); otherwise printable text as UTF-8; Enter `\r`, Backspace `0x7f`, Tab, Esc; arrows `ESC [ A` (or `ESC O A` in application mode); Home/End/PgUp/PgDn/Insert/Delete/F1–F12 as xterm; ctrl+letter as C0; alt prefixes ESC; shift or ctrl+Enter as `CSI 13 ; m u`. Mouse reporting goes to the program except with shift held (shift keeps local selection).

## App hooks
`Terminal.hooks` is a struct of closure-free function pointers with a `holder` (the ThumbTarget shape), so the framework gains no closure:

| Hook | When | Purpose |
|---|---|---|
| `key(holder, mut t, e, mut cx) bool` | first thing in `Terminal.key` | an app can consume a key (completion) |
| `overlay(holder, mut t, mut cx) ?Element` | end of `render` | a child drawn on top of the grid, refreshed on every render |
| `rows(holder, mut t, top)` | start of `Grid.paint` | fill `t.classes[r]`: one class per column of each visible row, or empty |
| `output(holder, mut t, data)` | after each chunk of program output is parsed, even off screen | app bookkeeping that must not wait for a paint (OneTerm's gutter timestamps) |

`TerminalOptions.quiet_exit` drops the `[process exited]` line, for an app that reports the end itself (a dropped SSH link).

`Terminal.tints` is the style table for those classes (index = class byte). Merge, per cell: a class foreground replaces only the default foreground (or any, with `override_ansi`); background, underline, bold and italic are additive. Runs of text also break at a class change. Cost with no hooks: one nil test per row. `Terminal.link_cols(row)` lists the columns of detected URLs and OSC 8 links in a row, for an app's URL coloring.

## Delivery
- The PTY's reader thread pushes chunks into a `core.Inbox`. The inbox posts one drain to the main thread while none is pending; the drain swaps the buffer, feeds the parser and marks the session changed. Output bursts therefore cost one render per frame at most.
- Nothing on the path allocates per read: the reader lends its 64 KiB buffer to `PtyOptions.on_data`, a plain function that gets `PtyOptions.ctx` (the inbox) back, so a terminal makes no closure (valid for the call; a consumer that keeps bytes copies them), the inbox appends it to its buffer under the lock, and a drain swaps that buffer with a spare it gets back once the listener returns (`Chunk.data` is lent the same way). A buffer grown past 4 MiB by a burst is left to the collector, and so is one over 256 KiB that carried less than an eighth of its size (the burst is over) and both once the program has exited. Under a 55 MiB/s stream the collector then runs about once a second instead of fifteen times, and each of its stop-the-world pauses would also stop the reader and back the program up.
- `FakePty` pushes synchronously, so harness tests need no waiting.

## ConPTY host
Two hosts serve a pseudo console: the system one (`kernel32!CreatePseudoConsole`, served by the inbox `conhost.exe`) and the redistributable Windows Terminal pair, `conpty.dll` with `x64/OpenConsole.exe` beside it. The inbox host swallows Sixel DCS strings and lags the redistributable by months, so the pair is preferred.
- `spawn_conpty` looks for `conpty.dll` in the directory of the running executable. When the file exists, loads by absolute path (`LoadLibraryExW` with `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32`, never a search of `PATH` or the current directory) and exports `CreatePseudoConsole`, `ResizePseudoConsole` and `ClosePseudoConsole`, those three come from it; anything else (no file, a load failure, a missing export) falls back to kernel32 for all three. The DLL starts `x64\OpenConsole.exe` from its own directory. The attribute-list functions always come from kernel32. A handle is resized and closed through the host that created it.
- The file is not hashed at run time: the build step that stages it verifies it (OneTerm, specs/22).
- The output pipe is created with a 1 MiB buffer, as the original does, so the host is less often blocked on a full pipe; the input pipe keeps the default.
- The bundled host gets `PSEUDOCONSOLE_GLYPH_WIDTH_WCSWIDTH` (0x10), the width rule the parser uses, so it never depends on that host's default. The system host gets no flags: an older inbox host may reject bits it does not know.
- **Close**: `Pty.close` closes the pseudo console, which asks its program to exit. A program whose start-up had not finished by then never sees that request and never exits (its console host lingers with it), so a thread of its own waits up to 2 s on a duplicate of the process handle, then terminates that program with a line on stderr. Only this process's own child, through its own handle; a program that ends by itself skips the wait.
- `Platform.pty_host()` names the host the next `spawn_pty` gets: `conpty: bundled`, `conpty: system`, `''` without a PTY (headless). `win.resolve_conpty(dir)` is the same choice against an explicit directory, for tests.

## Tests
- `kit/term/protocol_test.v`: OSC 133 marks and zones through scrollback and reflow, OSC 7501, 9, 9;4, dynamic colors and queries, underline color and links, Sixel placement and sweep, DEC modes and DECRQM, DA, XTVERSION, XTGETTCAP, kitty keyboard, mode 2026 hold, mode 2027 clusters.
- `kit/term/*_test.v`: table-driven parser and screen tests (bytes in, screen text, cursor and cell attributes out), reflow tests, scrollback trimming and size, key encoding.
- `tests/term_test.v`: the view in the harness on a `FakePty`: typing echoes, output renders, wheel scrolls history, selection and copy to the headless clipboard, paste with bracketing.
- `bench/term`: parse throughput (plain text and SGR-heavy) and a 200×60 repaint.
- `tests/term_corpus_test.v`: replays the corpus (specs/22) and compares every cell (underline color, hyperlink, grapheme marks) and the cursor, shape, pending wrap, modes, palette overrides, tab stops, scroll region and title event count.
- ConPTY smoke test (`tests/conpty_test.v`): spawns `cmd /c echo`, skipped when no console session is available.
- `quadra/platform/win/conpty_host_windows_test.v`: the host choice both ways (a directory with the staged `conpty.dll`, skipped when the pair is not staged in `.tmp/`, and an empty one).

# 16. Developer-Tool Kit

The v1 kit (specs/09) covers forms and general UI. This spec adds what terminals, code
editors and cluster managers need: a workbench layout, a terminal, a code editor and live
data views. Everything follows the One design language (specs/15), stays headless
testable, and keeps per-frame builders closure-free (specs/14, Memory).

## Modules

| Module | Contents |
|---|---|
| `quadra/kit/work` | workbench: activity bar, tab strip, pane tree (split, resize, collapse), panel header |
| `quadra/kit/term` | VT parser, screen model with scrollback, terminal view, PTY interface (ConPTY, fake) |
| `quadra/kit/code` | piece-table buffer, line index, tokenizer interface, editor view, find |
| `quadra/kit/data` | (exists) table, tree; adds live table, log view, value viewer (JSON/YAML) |
| `quadra/kit/viz` | sparkline, line and bar charts on `ui.canvas` |

## Workbench (`kit/work`)
- **Activity bar**: a 40 px column of ghost icon buttons; the active one has an accent icon. Badges for counts, tooltips with shortcuts.
- **Tab strip** (32 px): dirty dot (always shown), icon, title, a close button shown on hover, middle-click closes, drag to reorder, overflow scrolls with the wheel and a "more" menu. The active tab has the canvas tone and no bottom line, so it joins its content; seams sit on the side away from it (specs/15). Tabs show a close button only when the strip has an `on_close` listener (a fixed set of views, like detail tabs, is not closable).
- **Pane tree**: nested horizontal and vertical splits whose leaves are tab groups. Dividers are 1 px with a 6 px grab area, sizes in fractions, a minimum size per pane, and double-click on a divider to reset. Leaves of a stacked split fold to their header (`PaneNode.toggle`, `.header(h)` sets the kept height; never both sides). The tab strip opts into `wheel_x()`, so the plain wheel scrolls it sideways (other horizontal scrollers keep shift+wheel, so a page never loses its wheel). The tree is plain data (`PaneNode`) that apps persist through settings.
- **Panel header** (28 px): the title in muted text, actions as icon buttons, an optional filter input.
- State lives in the app's model; components fire events (`on_select`, `on_close`, `on_move`, `on_resize`).

## Terminal (`kit/term`)
- **Parser**: a VT500-style state machine over bytes, UTF-8 aware. It covers C0 controls, CSI (cursor moves, erase, insert/delete, scroll regions, modes), SGR (16, 256 and true color, bold, faint, italic, underline styles, inverse, strike), OSC 0/2 (title), OSC 8 (hyperlinks) and OSC 52 (clipboard, off by default). Also DEC private modes for the alternate screen, cursor visibility, bracketed paste, application cursor keys and mouse reporting (SGR 1006). Unknown sequences are ignored, never printed.
- **Screen**: a grid of cells (codepoint cluster, fg, bg, flags) with a scrollback ring (default 10 000 lines, configurable) and an alternate screen. Rows reflow on resize. Wide (East Asian) and combining characters take the right number of cells.
- **View**: renders only visible rows. Each row becomes background quads merged by run and glyph runs by style; rows are cached until their cells change. A cursor (block, bar or underline) blinks through a paint-only timer. Selection is by cell (word on double-click, line on triple-click), copy on ctrl-shift-c or the menu, paste is bracketed. Colors come from `theme.code.terminal`.
- **PTY**: `interface Pty { write(data []u8) ! resize(cols int, rows int) ! close() }`, with output delivered on the UI thread in batches. `conpty` uses Windows pseudo consoles; `fake` replays scripted output for tests.
- Budgets: 50 MB/s of plain output parsed; a 200×60 screen repaints in < 2 ms; 100 000 lines of scrollback in < 40 MB; typing echo < 1 frame.

## Code editor (`kit/code`)
- **Buffer**: a piece table over the original file and an append buffer, with a line-start index updated incrementally. Undo and redo hold grouped edits (typing runs, paste, multi-cursor edits as one group).
- **View**: virtualized lines. It has a gutter with line numbers (active line brighter) and diff marks (add, change, remove) from `theme.code.editor`. It also shows the current-line highlight, selection, find matches, bracket match, indent guides and optional whitespace. Long lines scroll horizontally; soft wrap is optional.
- **Tokenizer**: `interface Tokenizer { tokenize(line string, state int) ([]Token, int) }`. Each line keeps its end state, so an edit retokenizes until the states match again. Token kinds map to `theme.code.syntax`. It ships with a V tokenizer and a plain-text one; others plug in.
- **Editing**: grapheme-aware movement, word and line operations, multiple cursors (ctrl-click, ctrl-d for next match, alt-shift-arrows for a column), auto-indent, tab or spaces, IME composition. Shortcuts come from the keymap (context `Editor`).
- **Find**: an inline bar with match case, whole word and regex options, plus replace and replace all as one undo group.
- Budgets: open a 10 MB file in < 200 ms; a keystroke on a 100 000-line file in < 2 ms; scrolling at display rate.

## Data views (`kit/data`, `kit/viz`)
- **Live table**: rows identified by key. An update changes only the affected rows; changed cells flash once through a paint-only animation. Sort and filter keep the selection. Columns can be pinned and hidden. It ships with status-cell and age-cell renderers (`kit.status_label`, relative time).
- **Log view**: an append-only virtual list with ANSI colors through the terminal's SGR parser. It has follow mode (stops when scrolled up, resumes at the bottom), a filter (text or regex) that dims or hides lines, line wrap on or off, and timestamps in a gutter. Budget: 1 000 000 lines and 10 000 lines/s appended without dropped frames.
- **Value viewer**: a collapsible tree for JSON and YAML values. Keys use the syntax colors, and the path of the current node can be copied.
- **Charts**: sparkline (inline, 1 px line plus an area at 12 %), line chart (time axis, up to 8 series, hover crosshair with values in mono) and bar chart. Series colors come from a fixed categorical palette tuned per theme.

## Reference app: `apps/cluster`
A cluster dashboard on a fake data source (no network). It exercises everything above:
- workbench with an activity bar (Workloads, Config, Network, Events) and a resource table per kind (live table, status cells);
- detail tabs: Overview, YAML (value viewer), Logs (log view), Shell (terminal on a fake or local PTY);
- a metrics panel with sparklines and charts;
- an editor tab for a resource manifest (code editor with a YAML tokenizer).

## Testing
- The parser and screen have table-driven tests: byte sequences in, screen text plus cell attributes out.
- The terminal view runs against `fake` PTYs in the harness; ConPTY gets a smoke test that is skipped without a console session.
- The editor has buffer property tests (random edits against a plain string model), harness tests for movement, multi-cursor and undo, and a 100 000-line performance test in `bench/`.
- Every component gets a gallery page and harness interaction tests (specs/13 rule).

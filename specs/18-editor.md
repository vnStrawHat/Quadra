# 18. Code editor (`quadra/kit/code`)

Implementation spec for the editor of specs/16. Budgets there: open a 10 MB file in
< 200 ms, a keystroke on a 100 000-line file in < 2 ms, scrolling at display rate.

## Layers
| Piece | Role |
|---|---|
| `Buffer` | piece table + line-start index; byte offsets, UTF-8 |
| `History` | undo and redo of grouped transactions with cursor states |
| `Doc` | buffer, history, cursors and the editing commands; no UI, fully unit tested |
| `Tokenizer` | per-line tokenizing with an end state; V, YAML, Markdown, about 30 languages through one table-driven tokenizer, plain text |
| `Highlighter` | line end states, recomputed lazily from the first edited line |
| `Find` | query, options, matches |
| `Editor` | the view: one element paints gutter, text, selections, cursors; input and IME |

## Buffer
- Pieces point into the original text or an append-only add buffer. An insert at the end of
  the last added piece extends it, so typing runs stay one piece.
- `starts []int` holds the byte offset of every line start. An edit shifts the starts after
  it and splices in the starts of inserted newlines: O(lines) integer work, about 50 µs at
  100 000 lines.
- Piece lookup by offset is a binary search over piece start offsets (rebuilt per edit,
  O(pieces)).
- API: `len`, `line_count`, `line_start(n)`, `line_end(n)` (before the newline), `line(n)`,
  `line_of(offset)`, `slice(a, b)`, `insert(off, s)`, `delete(a, b)`, `text()`.
- Line endings: CRLF files are read as LF and written back as CRLF (`Buffer.crlf`).

## Editing model (`Doc`)
- A cursor is `anchor` and `head` (byte offsets) plus a goal x for vertical moves. There is
  always at least one; overlapping cursors merge after every command.
- An edit command applies to every cursor, last offset first, as one transaction. Other
  cursors are shifted by the edits before them.
- Undo groups: consecutive typing at the cursor merges into one group until a newline, a
  cursor move, a different command or a 1 s pause. Paste, cut, line operations, indent and
  replace-all are one group each. Undo restores the cursors from before the group.
- Movement is by grapheme (`quadra/text` boundaries) inside a line; word moves use a
  character class (word, space, punctuation); Home toggles between the first non-blank and
  column 0.
- Commands: insert text, newline with auto-indent (copies the leading whitespace, adds one
  level after `{`, `(`, `[` or `:` at line end), backspace and delete (also by word), tab and
  shift-tab (indent or outdent the selected lines; spaces or tabs per `Doc.indent`), delete
  line, duplicate line, move lines up or down, toggle line comment, select all, select line,
  add cursor above or below, select next occurrence, column selection, copy, cut, paste
  (one line per cursor when the counts match).

## Highlighting
- `interface Tokenizer { tokenize(line string, state int) ([]Token, int) }`;
  `Token{start, end int, kind TokenKind}` with kinds that map to `theme.code.syntax`.
- `Highlighter` keeps the end state of every line and `valid` (lines below it are stale).
  An edit lowers `valid` to the edited line. Painting asks for the start state of the first
  visible line, which tokenizes forward from `valid` only as far as needed. A keystroke deep
  in a large file therefore costs the visible lines, not the file.
- V tokenizer: keywords, types (capitalized), functions (name before `(`), strings with
  `${}` interpolation and escapes, runes, numbers, comments (`//`, nested `/* */` across
  lines), attributes. YAML tokenizer: keys, strings, numbers, booleans and null, comments,
  anchors and aliases, block scalars across lines, document markers.
- Other languages share one scanner (`LangTokenizer`, lex_lang.v) driven by a `LangSpec` row
  per language (langs_c.v, langs_script.v, langs_data.v); a new language is a row, not a
  lexer. A row holds keyword, control, type, constant and builtin word sets (case-folded for
  SQL, PowerShell, batch, Dockerfile, INI), line comment markers (`//`, `#`, `--`, `;`, `rem`,
  `::`; shell `#` only at a word start), one block comment pair (`/* */`, `<# #>`, Lua
  `--[[ ]]`, nesting for Rust, Swift, Kotlin), quote sets (one-line, multi-line, raw), triple
  quotes, string prefixes (`r b f u`, C# `@ $`, Rust `r#"..."#`), interpolation openers per
  quote (`${`, `#{`, `\(`, `$(`), Lua long strings, and flags: `$var` / `%var%` variables,
  `@name` as attribute, variable or keyword, C preprocessor lines, Rust `#[...]`, lifetimes
  and `name!` macros, capitalized types and ALL_CAPS constants, dash words (cmdlets,
  `-Param`, CSS), keys before `:` or `=`, `[section]` lines, make targets and batch labels,
  CSS selector lines. Two modes are not rows: markup (tags, attributes, strings, entities,
  `<!-- -->` and tags across lines) and diff (whole lines by their first characters).
  States: 0 code, 1..999 block comment depth, 1000 + quote + flags (raw 256, triple 512, hash
  1024) an open string; markup 1 comment, 2 inside a tag. One-line quotes close at the line
  end, so a stray apostrophe colors one line, not the rest of the block.
- Kinds follow the V tokenizer, so a fence reads like the editor: keyword, control, typ,
  function (a word before `(`, after a dot with `(`), property (after a dot, or a key), number
  (hex, binary, underscores, floats, exponents, units), str, escape (and interpolation marks
  around a variable), comment, constant, attribute, variable, tag, operator, punctuation.
- `tokenizer_for(name)` takes a file path (`src/a.rs`, `Dockerfile`) or a fence info string
  (`python`, `js {.class}`, `rust title=main.rs`), lowercased, with `{...}` attributes and the
  folder dropped; it tries the whole name, its first word, then their extensions against
  every row's names and aliases. Unknown names stay plain.
- Cost: no substring per word: word sets are compared in place (`Lex.word_in`, by length
  then bytes, folded when the row says so), the row is a pointer into the const table, and
  the token array is made once per line (`line.len / 3 + 2` tokens). `bench/highlight`
  (2,000-line blocks, clang -prod): V 0.31-0.41 us and 208 B a line (was 0.43 us, 524 B with
  a substring per word and a growing array), JS 0.41-0.43 us / 224 B, Rust 0.55-0.65 us /
  228 B, Python 0.40-0.42 us / 200 B.

## View
- One element. Paint walks the visible lines only. Each line has one cached line layout. The
  text is painted per token by a glyph range of that layout (`PaintCx.glyphs_range`), so
  colors never re-shape.
- Gutter: line numbers right-aligned in the mono face (the cursor lines brighter), then a
  diff mark column (add, change, remove from `Editor.marks`).
- Under the text: current-line band, selections, find matches, the matching bracket. Above
  it: indent guides, optional whitespace marks, cursors (bar, blinking via
  `request_paint_after`).
- Tabs are drawn to the next tab stop (`Doc.tab_size`). Long lines scroll horizontally; the
  view keeps the primary cursor visible. Soft wrap is opt-in (below).
- Mouse: click places the cursor, drag selects, double-click selects a word, triple-click a
  line, ctrl-click adds a cursor, alt-drag selects a column, shift-click extends. The wheel
  scrolls; shift-wheel scrolls sideways.
- IME: the composition is drawn inline at the primary cursor, underlined, and the IME
  window is placed there.

### Soft wrap
- `EditorOptions.wrap` / `Editor.wrap` (off by default; prose documents turn it on). A line
  shows as visual rows broken at the text width (one column per character, mono face),
  after the last space that fits; a word longer than a row breaks inside it; spaces may
  hang past the edge. Continuation rows are not indented; no horizontal scroll.
- A row table (`wrap.v`: the first row of each line, the start of each row) is rebuilt when
  the document version, the column count or the tab size changes: one pass over the bytes,
  no text layout. Paint still walks only the visible lines.
- Rows drive y everywhere: paint (band over all of a line's rows, number on the first),
  hit-testing, reveal, the scroll extent. Up/down/page move by rows and keep the column
  inside the row; home/end and the other moves stay per line.
- `top_line()` (the line at the top, fractional by rows scrolled past) and
  `scroll_to_line(n)` let an app keep a preview in step; both work with wrap off.

## Find
- An inline bar at the top right: query, match case, whole word, regex (`regex` module),
  previous and next (enter, shift-enter, F3), replace and replace all (one undo group).
  Matches are found over the whole document when the query changes and are shifted by later
  edits; the bar shows "n of m".

## Keys (context `Editor`)
Arrows, home/end, page up/down and ctrl-home/end move (shift extends). Ctrl-left/right move by
word, ctrl-backspace and ctrl-delete delete by word. Ctrl-z / ctrl-y / ctrl-shift-z undo and
redo, ctrl-a selects all, ctrl-l selects the line, ctrl-d adds the next occurrence,
ctrl-alt-up/down add a cursor, alt-shift-arrows select a column. Ctrl-shift-k deletes the line,
alt-up/down move lines, ctrl-shift-d duplicates, ctrl-/ toggles a comment, tab and shift-tab
indent. Ctrl-f finds, ctrl-h replaces, escape closes the bar or collapses to one cursor.
Ctrl-c, ctrl-x and ctrl-v use the clipboard.

## Tests and budgets
- `buffer_test.v`: random edits against a plain string model (property test), line index
  checks after each edit, CRLF.
- `doc_test.v`: movement, every command, multi-cursor edits, undo grouping.
- `tokenizer_test.v`: token kinds for V and YAML, multi-line states, lazy highlighting.
- `lex_lang_test.v`: per language family (C, JVM, Swift, Go, Rust, JS/TS, scripts, shells,
  SQL and build files, data formats, markup and diff) keywords, strings, comments, numbers
  and states across lines; alias and extension resolution; bytes per line (token array only).
- `tests/editor_test.v`: the view in the harness: typing, clicks, selection, multi-cursor,
  find and replace, clipboard (headless), IME, scrolling.
- `bench/editor`: open 10 MB, a keystroke plus repaint on a 100 000-line file, a scroll frame.

## Status (M9 done; deviations)
- Undo lives in `Doc` (`undo_stack`, `redo_stack` of groups), not in a separate `History`.
- Find rescans the whole document once per document version while the bar is open, instead
  of shifting the old matches by the edit. Replacement text is literal (no regex groups).
- Brackets: the match is searched within 20 000 bytes and strings and comments are not
  skipped. Indent guides are not drawn on blank lines. Whitespace marks are not drawn.
- `Editor.marks` paints diff marks, but nothing produces them yet.
- Horizontal scrolling reaches the widest line painted so far (lines are not all measured).
  Dragging past the edge scrolls only while the mouse moves (no repeat timer).
- Tokenizers: the expression inside `${}` is colored as one variable; YAML quoted strings do
  not continue across lines. A ligature that spans two tokens takes the first one's color.
- Table tokenizer ceilings: JS regex literals and JSX tags, heredocs, Python f-string
  fields, Ruby `=begin` blocks, C++ raw strings, Rust `r##"` (two or more hashes) and
  `<script>`/`<style>` bodies in HTML are not recognized (they read as code, string or text);
  Dockerfile instruction words match anywhere on a line.
- Column selection by keys is alt-shift-up/down (add a cursor at the anchor column) plus
  alt-shift-left/right (extend every cursor).
- In the find bar, enter goes to the next match (the input submits) and shift-enter to the
  previous one.
- The manifest editor in apps/cluster is read-only until live data can apply edits (M10).

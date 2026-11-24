# Data

Lists, trees, tables, status and charts: what there is much of. The rule for all of it: rows
are quiet and alike, so that the one thing that differs (a failure, a selection, a change)
is what the eye lands on.

## Rows

| | Data row | Item row |
|---|---|---|
| Height | 24 | 28 |
| Used for | lists, trees, tables, logs, variables | menus, navigation, options, palettes |
| Inside left and right | 8 | 8 |
| Text | 14, one line, ellipsis | 14, one line |
| Icon | 14, 6 before the name | 14 or 16 (navigation) |
| Hover | `panel_hover` | `panel_hover` |
| Selected | `accent_tint`, a 2 px `accent` bar on its left edge, its icon `accent_text` | the same |
| Keyboard's row | `press` | `press` |
| Lines between rows | none (a table: a `line` under each) | none |

- A row against the edges of a square container (a panel, a table) is square and edge to
  edge. A row inside a rounded container with an inset (a menu, a card) has radius 4.
- Tree levels indent 20 with a 1 px `line` guide at each level; a row with children has a
  12 px chevron in the icon's column before the icon.
- What a row can do shows at its right end on hover or selection, as 22 px icon buttons;
  the room for them is kept, so names do not move.
- Counts and sizes at a row's end are mono `text_3` (counts) or `text_2` (sizes), right
  aligned.
- An empty list says so in its middle (patterns.md, Empty state); it is never a blank box.

## Table

- A header of 28 on `table_head_bg`: 12 px `table_head_fg` labels, a `line` under it; a
  sorted column has a 12 px arrow after its label; columns resize on a 6 px grab at their
  right edge.
- Rows of 24 (`row_height(..)`: 28 for rows with controls), a 1 px `line` under each, cells
  8 inside left and right; text left, numbers mono and right, a status as its mark and word.
- The selected row is `accent` at 10 % with the 2 px bar. A row under the pointer is
  `panel_hover`.
- Cells never wrap: a long value ends with `…` and a tooltip says it whole.

## Status

A state of something that runs has a colour and a shape of its own, so it reads in
greyscale:

| Status | Shape | Colour |
|---|---|---|
| running | filled square | `ok` |
| pending | hollow square | `warn` |
| failed | filled triangle | `error` |
| succeeded | check | `cyan` |
| terminating | dash | violet (`hue_purple`) |
| unknown | filled diamond | `text_3` |
| none | nothing, its room kept | |

The mark is in a box of 14 px (8 px in a dense table's row, 10 before a word in 12 px
text), 6 before its word, which is in the status's colour. Many marks in a row
(`status_dots`) are a third of their size apart.

## Badges and tags

- A **badge** says a kind or a state in a word: 20 tall, 12 px mono in a tone, a faint tint
  of the tone behind it (10 %), a border of it at 20 %, radius 4, 6 inside. A plain badge
  (a word that is not code) is in the UI face with no border.
- A **tag** is a name among others (a label, a version): 22 tall, 12 px mono `text` on
  `raised` with a `line_strong` border, radius 4, 6 inside.
- Badges and tags in a row are 4 apart. A count beside a name is a badge 6 after it.
- A **key hint** is a shortcut in a box (radius 4, 4 inside, a faint border) in 10 px mono
  before or after the words it belongs to.

## Feedback in place

- **Alert**: a line or two in a view that says something went wrong or needs care: a 14 px
  status icon, a title (14, semibold) and a sentence (`text_2`), on the tone's tint with a
  border of the tone at 30 %, radius 6, inset 12. It is in the flow where the problem is,
  not floating.
- **Progress**: a bar 4 tall (radius 2), `accent` on `line_strong`, 200 wide alone; busy
  with nothing to measure, a piece that crosses it. A spinner is 14 px in the text's colour.
- **Skeleton**: `panel_hover` blocks where text will be, the height of its lines, radius 4.
- **Empty state**: patterns.md.

## Charts

Charts are built from the same tokens: axes and labels 12 px `text_2`, grid lines `line`,
series in the theme's series colours (the hues, in an order that keeps neighbours apart), a
line 1.5 px with no steps, values in mono, the hovered point's values in a tooltip. A chart
has no border and no plane of its own; it sits in a card or a panel.

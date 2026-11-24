# Surfaces

What holds other things: what floats over the window, what groups on a plane, and the window's
own bars. Each kind has one inset, one radius and one arrangement; a view fills it, it does
not restyle it.

| Surface | Plane | Border | Radius | Inset | Between its things | Level |
|---|---|---|---|---|---|---|
| Menu | `overlay_bg` | `line` | 6 | 4 (rows 4 inside it) | 0 | 1 |
| Tooltip | `overlay_bg` | `line` | 6 | 8 × 4 | 2 | 1 |
| Popover, hover card | `overlay_bg` | `line` | 6 | 12 | 8 | 1 |
| Dialog | `overlay_bg` | `line` | 10 | 12 | 12 | 2 |
| Drawer | `overlay_bg` | `line` on its inner edge | 0 | 12 | 12 | 2 |
| Notification | `overlay_bg` | `line_strong` | 10 | 12 | 8 | 2 |
| Toast | `overlay_bg` | `line` | 10 | 6 × 12 | 8 | 2 |
| Card | its plane | `line_strong` | 6 | 12 | 8 | none |
| Large card (a settings group) | its plane | `line` | 6 | 16 | 16 | none |
| Panel | `panel` | a `line_strong` seam to the content | 0 | 0 (its rows have theirs) | 0 | none |

## Dialog

```
+-- 440 --------------------------------------------+
|  12                                               |
|  Title (16, semibold)                       [x]   |  head: 12 around, the cross 22
|                                                8  |
|  body: a column, 12 between its things            |  body: 12 left and right
|                                                   |
|                                               12  |
|---------------------------------------------------|  a `line`
|  [ Delete… ]                 [ Cancel ] [ Save ]  |  foot: 8 above and below, 12 at the ends,
+---------------------------------------------------+        buttons 8 apart
```

- 440 wide (`.width(..)`: 360 for a question, 560 for two columns of fields), 80 from the
  window's top, centred, no backdrop; a click outside and Escape dismiss it.
- The head is the title, or a row of the app's (`.head(..)`: a mark and the title, a tag); the
  cross is `text_2` in a 22 px ghost box.
- The body is a column 12 between its things, its fields as wide as it is. Its text is
  `text`; the one sentence of a `confirm` is `text_2`. A body taller than the window scrolls between the
  head and the foot.
- The foot: `.action(label, variant, f)` in order, the main one last; `.aside(label, f)` at
  the far left; `.shortcut("Enter")` on the main one. With an aside the foot is 4 in at
  its left, so that the quiet button's label (8 inside it) stands on the body's left edge. A
  button closes the dialog unless it is told `.stays_open()` (Test connection, Run now). A dialog that only tells has one
  button, "Close".
- A confirm (`confirm(..)`) is a dialog 360 wide: the question as the title ("Delete
  prod-api-01?"), what will happen as one sentence, Cancel and the verb ("Delete host").

## Popover and hover card

A popover hangs 6 from what opened it, its start on the opener's start, 280 wide unless its
content says otherwise. Its content is a column 8 between its things: an optional title
(14, semibold), then controls or text. A hover card is a popover that shows on rest and holds
no controls. Neither has a foot: what is in it acts at once.

## Menu

- At least 200 wide, at most 360; 4 above and below its rows. A row is 28 tall, 4 from the
  menu's sides, 8 inside (text 12 from the menu's edge), radius 4; hover `panel_hover`, the
  keyboard's row `press`.
- Icons 14 in `text_2`, a column of them only when some row has one; in their place a 10 px
  square of a colour (`swatch`) or a 7 px square of a role (`.dot(..)`).
- A shortcut is right-aligned, 12 px mono `text_2`, 16 from the label. A check is 14 px
  `accent_text` in the icon column. A submenu has a 12 px chevron at its end and opens
  beside the menu, its first row level with the row that opened it.
- A separator is a 1 px `line` edge to edge, 4 above and below. A heading is 24 tall,
  12 px `text_2`.
- A destructive row (`.danger()`) is `error`. A row may say a few words after its label in
  12 px `text_2` (`.detail(..)`).
- A long menu is capped at half the window or 450 px and scrolls.
- A searched menu has a head of 32 over its rows (a 12 px search icon in `text_3`, the
  field, a `line` under it), is 360 wide, and Enter does the first row that can be done.

## Tooltip

14 px text, at most 288 wide, shown after 500 ms of rest, 6 from what it is about, an arrow
12 across and 6 deep on the side toward it. A shortcut follows the words in `text_2`, 16
after them; more lines under them are 12 px `text_2` (`.meta(..)`). `.content(|| view)` puts
a view in the bubble with the popover's inset. A tooltip names an icon button, says the
whole of what was cut, or adds what a sighted reader would want to know; it never holds
what is needed to use the thing.

## Notification and toast

Notifications stack at the bottom right over the status bar, 12 from the edges, 8 apart, the
newest lowest; each 448 wide: a 16 px status icon, the title over a `text_2` body and an
optional 12 px `text_2` note of where it came from, a 22 px cross, optional sm buttons at the
right under them. They stay until closed unless given a time. A toast is one line at the
bottom centre, a status icon and words, gone after 10 s.

## Card

A card groups a few things on a plane: a 1 px `line_strong` border, radius 6, inset 12, its
things 8 apart. `card_head(title, more)` is its title (16) over a 12 px `text_2` line that
says more, 2 apart; its facts are lines of a name and a `mono(..)` value (`hstack()` with a
`spacer()`), or a `description_list`; a status is a line of its own under them; a
`button_row()` ends it. `card_head_with(title, more, end)` puts one thing at the end of the
head's first line: a status, a tag, a quiet button. A card holds no card. A large card (a group of settings) has inset 16 and rows 16
apart, its title (16, `text_2`) over it, 8 apart.

## Panels and bars

- **Panel**: a column on `panel` beside the content, a `line_strong` seam between them. Its
  header is 28 tall: a 12 px semibold `text_2` title 12 from the left, quiet icon buttons at
  the right, 4 apart, 4 from the end.
- **Title bar**: 32 on `chrome`, a `line` under it; the app's mark and name 12 from the
  left, then the app's own buttons at the right (22 px icon buttons, 4 apart), then the
  window's buttons, 36 wide each.
- **Tab strip**: 32 on `tab_bg`; a tab is 12 inside left and right, an icon 14, its title,
  a 6 px `accent` square when unsaved, a close button (18 with a 14 px icon) on the active
  tab and under the pointer; `line_strong` seams between tabs; the active tab is on `canvas`
  with a 2 px `accent` line along its top; no hover fill.
- **Tabs of a panel** (`tabs`): 28 tall, 8 inside, `text_2` until the pointer is over them or
  they are chosen; the chosen one has a 2 px `accent` line along its bottom; a `line` under
  the row. `.pill()`: no line, the chosen one `accent_tint`.
- **Toolbar**: a row of 32 with 6 above and below on `canvas`, a `line` under it; breadcrumbs
  in mono `text_2` split by `›` in `text_3`.
- **Status bar**: 30 on `status_bg`, a `line` over it, 12 px text in `text_2`, values in
  mono `text`; groups 12 apart with 1 × 14 `line_strong` rules; 8 from its ends; panel
  switches are 22 px icon buttons whose on state is an accent icon.
- **Activity bar**: 40 wide; an item is 40 × 40 with a 16 px icon; the active one has a 2 px
  `accent` bar at its left.
- **Sidebar** (navigation): rows of 28, an icon 16 and a label, 8 inside, a section label
  (12, `text_2`) over each group, 16 between groups; the chosen row `accent_tint` with the 2 px
  bar.
- **Split**: a 1 px `line_strong` seam with a 6 px grab area; no handle is drawn.
- **Scrollbar**: a 6 px thumb (`scroll_thumb`, radius 2 at its ends), 4 from the edge, shown
  while the pointer is over the container or it moves, gone 1 s later over 400 ms.

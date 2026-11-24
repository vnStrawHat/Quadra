# Layout

How to place things so a view looks designed the first time. Read this before building any
view; the recipes in patterns.md are these rules applied.

A view is boxes in rows and columns. Almost every mistake in a first draft is one of five:
things touch, things sit against an edge, things do not line up, a thing is squeezed or
overflows, two things are the wrong distance for what they are to each other. Each rule below
prevents one of them, and the kit's helpers apply the rules so a view writes no distances: no
gap, no padding, no width of a field. The one number a view may give is the width of a
container that has none of its own (a card, a popover's content), from the list in section 6.

## 1. Three distances: within, between, around

```
+-- container ---------------------------------+
|  around (inset)                              |
|   +-- thing ---------------+                 |
|   | label                  |                 |
|   |  within (4)            |                 |
|   | [ field            ]   |                 |
|   +------------------------+                 |
|      between (12)                            |
|   +-- thing ---------------+                 |
|   | label                  |                 |
|   | [ field            ]   |                 |
|   +------------------------+                 |
|  around (inset)                              |
+----------------------------------------------+
```

- **Within** a thing, its parts are 2, 4, 6 or 8 apart (space.md says which).
- **Between** things in one container: 8 for controls in a row, 12 for fields in a column,
  16 for groups, 24 for sections.
- **Around**: the container's inset, the same on all four sides, at least the gap between
  what it holds.
- **Never 0** between two things that are not one shape, and never a thing against its
  container's edge. If a view has `column()` or `row()` with no `.gap(..)` and more than one
  child, it is wrong unless the children are one shape (a table's rows, a group's segments).

## 2. The helpers

`quadra_design` has a helper for each common arrangement. They return a `Div`, so anything a
`Div` takes can follow (`form()` returns a `Form` and `field(..)` a `Field`: they take
`.child(..)`, and `.help(..)`, `.error(..)`, `.required()`, and go into a `Div` as any view). Use them in place of `column()` and `row()` in a view; reach for a
bare `row()` only inside a component.

| Helper | What it is | Distances |
|---|---|---|
| `vstack()` | a column of lines of one group | 8 between |
| `form()` | a column of fields; its fields and their controls fill its width | 12 between |
| `columns(n)` | `n` equal columns side by side (fields two to a row) | 12 between |
| `hstack()` | a row of controls or of a control and words, centred on the cross axis | 8 between |
| `button_row()` | a row of buttons at the right end | 8 between |
| `section(title)` | a 12 px section label over a column of what it heads | 8 under the label, 8 between |
| `sections()` | a column of sections | 24 between |
| `spacer()` | takes the room left in a row or a column | |
| `field(label, control)` | a label over a control, help or an error under it | 4, 4 |
| `card()` | a bordered group on a plane | inset 12, 8 between |
| `description_list(items)` | terms and their values in two columns: the facts of a thing | 8 between |
| `mono(words)` | what the machine said (a path, a time, a size), in the mono face | |

The containers come with their inset and their gap: the body of a `dialog`, a `drawer`, a
`popover`, a `hover_card` and a `card` is already a padded column. Put the content in; do not
wrap it in a padded box again.

## 3. A container has a head, a body and a foot

```
+----------------------------------------------+
| Title                                    [x] |   head: the title (16, semibold), what closes at the right
|----------------------------------------------|   (a seam only where the body scrolls under it)
|                                              |
|  body: one column, its things 12 apart       |   body: inset 12 (card, floating), 16 (large card), 24 (page)
|                                              |
|----------------------------------------------|   a seam
|                         [ Cancel ] [ Save ]  |   foot: actions at the right, 8 apart, the main one last
+----------------------------------------------+
```

- One title a container, at the top left, on the inset line.
- What dismisses is the top right corner (a 22 px cross) and Escape; what answers is the
  bottom right.
- In a foot, the main action is last (rightmost) and is the only primary one; Cancel is
  before it; an action of another kind (Delete, Edit…) is at the far left.
- The seam over a foot is a `line`; a head has a seam only when it is a bar of its own (a
  panel's header) or the body scrolls.

## 4. One left edge

Everything in a container's body starts on one vertical line: the inset. Titles, labels,
fields, help lines, section labels, the first button of a row.

- Do not indent to show hierarchy inside a form; use a section label.
- A check box or a switch with its words starts on the same line as the fields over it.
- Indent only what is truly nested: a tree's levels (20 a level), options under the option
  that turns them on (24: a tick box's width and its gap).
- Text beside an icon lines up with other text beside icons: icons in a column are one size.

## 5. A row is centred across, and its parts have roles

```
[mark] [icon] Name that may be long…        meta  [action]
  ^      ^     ^                              ^       ^
  6 px   6     takes the room left,           mono,   shows on hover or always;
  apart        one line, ellipsis             text_2  8 from the meta
```

- `align_items(Center)`: a row's parts share a centre line. `hstack()` does it.
- The name takes the room left (`grow(1)`, `min_width(0)`, `.ellipsis()`); what is at the end
  keeps its width (`shrink(0)`).
- Rows of one list have the same parts in the same places, present or not: a missing icon
  keeps its room, so names stay in a column.
- The row's own padding is 8 left and right (12 where it lies against a container's edge
  with no inset, as in a menu: 4 inset + 8).

## 6. Widths

- **A field fills its column.** In a `form()` every field is as wide as the form, and a
  segmented group too, its cells sharing the room. Two short fields may share a row
  (`columns(2)`): never more than two, and only fields that belong together (Address and
  Port, From and To). A field that holds a short value by nature may be told its width
  (`number_input(port).width(96.0)`); it then stands at the start of its row. A checkbox, a
  switch and a button keep their own width everywhere.
- **A section in a form** (`section(title)`) is a field's distance from its neighbours (12)
  and holds what is optional or of another kind; put it last, since nothing marks its end
  but the form's.
- **A button is as wide as its label** with its padding, at least as wide as it is tall;
  never stretched to its container but in a narrow column where it is the only action.
- **A dialog is 440 wide**: room for a column of fields, and for two short ones to a row.
  360 for a question (`confirm` is); 560 (`.width(560.0)`) when a row holds two fields whose
  values are long (two paths, a path and a name). A card in a page is 360 unless it is in a
  grid;
  a popover 280, a tooltip at most 288, a menu at least 200 and at most 360.
- **Text measures at most about 72 characters**: a description wraps inside 560 px.
- **A page** keeps its content at most 880 px wide, at the left, with the page's inset.

## 7. What gives way

When there is less room than a row wants:

1. The spacer goes first, then text that may wrap wraps, then names end with an ellipsis.
2. Controls never shrink under their width and never overlap; a row that cannot hold its
   controls wraps them under its text (settings rows do).
3. A column that is too tall scrolls its body; its head and foot stay.
4. Nothing is cut silently: what is cut ends with `…` and says the rest in a tooltip.

## 8. Vertical rhythm of a page

```
Headline (18)                         \
Description (14, text_2)              /  2 apart, then 24

SECTION LABEL (12, semibold, text_2)  \
+-- card ------------------------+     |  8 under a label
| row                            |     |
| row                            |    /
+--------------------------------+
                                         24 between sections
SECTION LABEL
...
```

- A page has one headline; a section has a label; a card has rows. Three levels, no more.
- Sections are 24 apart, cards in a section 12, the lines of a card 8 with no line between
  (a card of settings: rows 16 apart), or 0 with a `line` between them where rows are long
  and many.

## 9. Bars

A bar is one row of the window's edge: items 4 apart inside a group (icon buttons), groups
12 apart with a 1 × 14 `line_strong` rule between them, 8 from the bar's ends. The left end is
where you are (a title, a path), the right end is state and switches. A bar never wraps: what
does not fit is dropped from the middle outward, the ends stay.

## What a first draft gets wrong

| Seen | Cause | Do |
|---|---|---|
| Children stuck together | `column()` with no gap | `vstack()` or `form()` |
| Content against the border | a box with no padding | use the container's body; do not build a bare box |
| Labels and fields not in a line | fields with widths of their own | `form()`, let them fill |
| Buttons of different heights in a row | a size set on one of them | no size: all md |
| A control pushed out or overlapping | text with no `min_width(0)` | name takes the room left, control `shrink(0)` |
| Double padding | a padded column inside a dialog's padded body | put the content straight in |
| A title that is larger than its dialog's | a second 16 px heading | a 12 px `section(..)` |
| Uneven gaps | numbers typed by eye (10, 14, 18) | a step of the ladder, by helper |
| Icons of two sizes in one row | sizes typed by hand | 14, or nothing |
| Everything bold or everything in the accent | stress used as decoration | one title, one primary |

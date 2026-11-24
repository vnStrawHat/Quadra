# The larger parts

The parts of a window that are more than one control: their sizes and what they are made of.
Their distances are steps of the ladders in [space.md](space.md); the numbers here are the
ones a part owns (a width, a bar's height). What each does is specs/13.

## Title bar

32 tall on `chrome`, a 1 px `line` under it. At the left a 22 px ghost icon button that opens
the app's menu, then the name of the document or the project in 12 px; then the app's own
controls; at the right the caption buttons, 36 wide with 10 px glyphs of the system's icon
font; close hovers `#E81120`. `.title_size(16)` makes the title a window caption at body
size, 12 from the left edge (a settings window: 34 tall). An app whose title bar holds its
tabs or a search field makes it 38 tall so that 28 px controls have 5 above and below.

## Toolbar and breadcrumbs

45 tall on `canvas`, 8 at its ends, its buttons 22 px and 4 apart. Breadcrumbs are in the mono
face at 14 in `text_2`, 8 apart, separated by `›` in `text_3`; the last one is `text` and the
first ones give way (they shorten with an ellipsis).

## Tabs

- **Editor tabs** (`work`): a strip 32 tall. A tab has 4 inside, its label 14 in `text_2` on
  `panel`, or `text` on `canvas` when it is the active one, 1 px `line_strong` seams on the
  side away from the active tab, no fill under the pointer; the active tab has a 2 px `accent`
  line along its top. What is unsaved shows a 6 px `accent` square, always; the close button
  (18, a 14 px icon in `text`) shows under the pointer and on the active tab.
- **Panel tabs** (`tabs`): the chosen one has a 2 px `accent` line along its bottom and its
  label in `text`; the others are `text_2`, `text` under the pointer, with no fill.

## Status bar

30 tall on `chrome`, 8 at its ends, its items 12 apart, labels 12. Panel toggles are 22 px icon
buttons with 14 px icons in `text_2`; one that is on has its icon in the accent and no fill.
Groups are split by a 1 × 14 `line_strong` divider. Counts of diagnostics are in their hues
beside their icons. A status item with more to say says it in a tooltip, not in the bar.

## Settings, rules (`settings(..)`)

A column of pages 226 wide (12 in, a `line_strong` hairline at its right): the search field,
page rows 28 tall with a 12 px chevron in a 16 px box and a 14 px label (under the pointer
`panel_hover`; the chosen page `accent_tint` with a 1 px `line_strong` border, radius 6), a
key hint at the bottom over a `line`. A page row's chevron shows the page's groups under it
as 28 px `text_2` rows, 24 in; choosing a group chooses its page and brings the group's
header to the top.

The page: 32 at its sides, 24 above and below; a header row of the app's (scope toggles, then
actions at the right), the title as a headline, then groups: a mono 12 px `text_2` header
over a `line` (32 above it, 8 for the first), and rows of a 14 px title over a 12 px `text_2`
description that wraps, the control at the right, 16 above and below, a `line` under each. A
search shows the matching rows alone, and a page with none leaves the column.

## Settings, cards (`settings(..).cards()`, `settings_group(..)`)

A group stands on its own in any page as `settings_group(group)`: its title (16, semibold)
over the outlined card of its rows, 16 apart; groups are 24 apart (`sections()`).

- **Navigation**: 250 wide on `panel`, a 1 px `line` at its right. The search field as wide as
  the column, 12 from the top and the sides. Under it, 12 in, page rows 8 apart: 28 tall, 8
  inside, their parts 8 apart, radius 6, 14 px text: a 16 px icon, the label, and when the
  page has more than one titled group a 22 px ghost button with a chevron (right; down while
  open). Under the pointer `press` at 80 %; the chosen row `accent_tint` with a medium (500) label.
  The groups list under the row, 4 apart, inside a 1 px `line_strong` rule at the left (16
  in, then 8 to the rows); they are page rows without icons, as wide as the column.
- **Page header**: 16 in, a 1 px `line` under it; the title 16 at the left, the header's
  actions at the right, 4 apart; a description of the page under the title, 14 `text_2`.
- **Groups**: the page scrolls under the header; a group's title is 16 semibold with a 14
  `text_2` description 4 under it, then 16 below them the card: a 1 px `line` border, radius
  6, 16 in, rows 16 apart. A group without a title is the card alone.
- **Rows**: the title (14 `text`) over the description (14 `text_2`, wraps) at the left, at
  most 60 % of the row; the control at the right, centred, 12 from the text. A row with
  neither gives the control the whole row.

## Picker and command palette

544 wide (the palette 608), at most 384 tall, on the overlay plane with a dialog's corners.
The query row is 36 tall, 12 in, over a 1 px `line`; rows are the menu's (28 tall, text 12
from the edge; 24 in the dense picker), the chosen one with a 2 px `accent` bar at its left;
what matched is `accent_text`; shortcuts are muted at the right; a foot, when there is one,
has a `line` above it.

## Colour picker

The opener is a field (120 wide unless told or in a form): a 14 px swatch (radius 4, 1 px
`line_strong`) and the hex, or the placeholder in `text_3`. The popover is 12 in, its parts 12
apart, 208 inside: the area 208 × 140, radius 6, the hue under a white ramp (left to right)
and a black one (top to bottom); its knob a 12 px ring, 2 px white inside 1 px of black at
40 %. The hue bar is 208 × 12; its knob the same ring at 14, filled with the hue. Then the hex
field, as wide as the area, and the swatches 20 × 20, radius 4, 1 px `line_strong`, 6 apart
in rows; the chosen one ringed in `border_focused`.

## File and folder picker

A text field with a secondary button "Browse…" 8 after it, both 28 tall, a 14 px `file` or
`folder` icon before the button's label. In a form the field takes the room the button
leaves.

## Scrollbar

A 6 px thumb 4 from the edge, at least 25 long, `text` at 30 %. Everything that scrolls has
one (scroll areas, lists, the code editor, the terminal); strips the wheel moves sideways
(tab strips) have none. It is hidden until the pointer moves over what scrolls or its content
moves, stays 1 s after the last of these, then fades over 400 ms. A shown thumb can be
dragged; its 14 px track is the grip.

## Key hints

- **Key text**: one run a keystroke, the modifiers then the key joined by `-`, in the order
  Ctrl, Win, Alt, Shift (`Ctrl-Shift-P`), chords split by a space, in `text_2`; `key_label`
  writes this form, `key_text(keys, t)` the theme's.
- **Key hint** (`key_hint`): the keys in a box (radius 6, 4 inside, a 1 px border of `text` at
  16 %) at 10 px, then what they do, all mono italic 12 in `text_disabled`.
- **Kbd** (`kbd`): the keys alone, mono 12, for a menu row's right end or a button's label.

# Space, size and shape

Every number is a pixel at 100 % scale. Each kind of number has a ladder of four steps; a view
takes a step, it does not make a number. `tools/check.sh` fails on a distance or a radius off
its ladder in the kit, the gallery and the apps (checklist.md).

## Distances

Two ladders. The small one is for the parts **inside one thing** (a button, a row, a field
with its label). The large one is for the room **between things and around them**.

| Step | Token (`quadra_design::token`) | Inside a thing |
|---|---|---|
| 2 | `SPACE_XXS` | a title and its description; two lines that are one statement |
| 4 | `SPACE_XS` | a label and its field; a field and its help line; an icon button and the next; a tag and the next |
| 6 | `SPACE_SM` | an icon and its label; a mark and its word |
| 8 | `SPACE_MD` | a control and the next control; the lines of one group; a control's inner padding left and right |

| Step | Token | Between and around |
|---|---|---|
| 12 | `SPACE_LG` | a field and the next field; a card and the next; the inset of a floating surface, a card, a bar's ends |
| 16 | `SPACE_XL` | a group and the next group in a container; the inset of a large card |
| 24 | `SPACE_XXL` | a section and the next; the inset of a page |
| 32 | `SPACE_XXXL` | the inset of a wide page; a headline's block and the page under it |

`0` and `1` are not distances: 0 is for parts that are one shape (segments of a group, cells
of a table), 1 is a hairline. 48 and 80 exist once each (an empty state's room, a dialog's
distance from the window's top).

**The rule of three distances**: within < between ≤ around. The parts of a thing are closer
to each other than the thing is to its neighbour, and a container's inset is at least the gap
between what it holds. If two gaps in a view are equal, the eye reads one group; make them
differ by a step where there are two groups.

## Heights

**Controls** (`Size`; `size::H_XS` .. `H_LG`), each with its border:

| Step | Height | Text | Icon | Padding left and right | Used for |
|---|---|---|---|---|---|
| xs | 18 | 12 | 12 | 4 | a button inside a row or a tag |
| sm | 22 | 12 | 14 | 4 | icon buttons, buttons in bars, tags, the small segmented group |
| **md** | **28** | 14 | 14 | 8 | every control unless told |
| lg | 32 | 14 | 16 | 8 | the one action of an empty state or a sign-in card |

Every control of one line is md unless told: a button, a field of any kind, a segmented
group. Controls side by side are the same step. Checkboxes (16), radios (16), switches
(32 × 20 in a ring of 38 × 26) and sliders (20) are their own shapes; in a row with a 28 px
control they are centred on it.

**Rows** (a thing in a list that is pressed or read as a line):

| Step | Height | Used for |
|---|---|---|
| data | 24 | what there are hundreds of: list rows, tree rows, table rows, log lines, variables |
| **item** | **28** | what is chosen from: menu rows, navigation rows, options of a select, tabs of a panel, a panel's header; the status bar |
| bar | 32 | a strip of the window: the tab strip, a toolbar's row, a title bar, the search head of a menu |
| touch | 40 | a row that is only an icon (an activity bar, a folded rail) |

A bar's height is its row; where it holds controls smaller than its row they are centred
(22 px buttons in a 32 px bar: 5 above and below). The status bar is the exception: it is
lower than the title bar, an item row (28, its 22 px buttons 3 above and below), so that
the window's top edge is the heavier of the two.
A row of two lines (a title and a line about it, 2 apart) is the touch step, 40.

## Radii

| Step | Token | Used for |
|---|---|---|
| 2 | `RADIUS_XS` | marks: a status square, a colour tag, a progress bar, a slider's track, a scroll thumb's ends |
| 4 | `RADIUS_SM` | small things and things inside a rounded thing: a tick box, a badge, a tag, a key hint, an xs or sm button, a row inside a menu |
| **6** | `RADIUS_MD` | controls and what is their size: buttons, fields, segmented groups, cards, menus, popovers, tooltips, a row with an inset in a list |
| 10 | `RADIUS_LG` | what floats and is large: dialogs, notifications, toasts |

A thing inside a rounded thing has the outer radius less the inset between them, and not
less than 2: a row 4 inside a menu of 6 is 4 (the ladder's nearest), a cell inside a
segmented group of 6 with a 1 px border is 5. What touches the window's edge is square
(panels, bars, tabs, drawers, tables). A pill (a radius of half the height) is for a switch
and an avatar only.

## Lines

- A hairline is 1 px: `line` between two parts of one plane (a header and its body, the rows
  of a table), `line_strong` around something that is pressed or typed in (a field, an
  outlined button, a card) and between two planes (a panel and the content).
- A mark of what is chosen is 2 px of `accent` on one edge: left for a row, bottom for a tab
  of a panel, top for a tab of a strip. Wide rows that are tall (the rail's tabs) take 3.
- A focus is the control's own 1 px border in `border_focused`; a control with no border gets
  a 1 px outline that takes no room. Nothing else is outlined.

## Icons

Lucide outlines drawn on a 24 grid with a 2 px stroke, shown at four sizes:

| Size | Used for |
|---|---|
| 10 | a chevron in a stepper or a tight corner |
| 12 | a chevron or a check at the end of a row; a mark inside 12 px text |
| **14** | every icon in a control, a row, a menu, a bar |
| 16 | navigation rows, a card's or dialog's title, lg buttons, a notification's status |

Larger only as the picture of an empty state (32, `text_3`). An icon is the colour of the
text beside it (`text_2` when it only decorates, `accent_text` when toggled); it is 6 from
its label; icons in one column are one size and one behind the other. A button that is only
an icon is 22 square with a 14 icon and always has a name a reader hears and a tooltip.

## Elevation

Only what floats has a shadow, and there are two levels. On a dark plane a light shadow does
not show, so dark themes cast deeper ones.

| Level | Used by | Shadows (x, y, blur, black alpha), light | dark |
|---|---|---|---|
| 1 | menus, tooltips, popovers, combobox lists, the command palette | 0 2 3 .12; 0 1 0 .06 | 0 6 16 .34; 0 1 3 .26 |
| 2 | pickers, dialogs, notifications, toasts | 0 2 3 .12; 0 3 6 .08; 0 6 12 .04; 0 1 0 .12 | 0 18 44 .44; 0 4 10 .30 |

Every floating surface is `overlay_bg` with a 1 px `line` border at its level. There is no
backdrop behind a dialog: a click outside dismisses it.

## Motion

- Hover, press, focus, selection: at once.
- What appears and leaves (menus, popovers, dialogs, notifications): 150 ms, quintic ease
  out, a few pixels of travel and opacity.
- What is busy: a spinner turns once a second; a busy bar's piece crosses again and again.
- A scroll thumb fades out over 400 ms, 1 s after the last movement.
- Reduced motion (the OS setting) makes all of it instant but the spinner.

## Chrome sizes

| Element | Size |
|---|---|
| title bar | 32 (an app may take 38) |
| tab strip | 32 |
| toolbar | 32 row with 6 above and below, a `line` under it |
| panel header | 28 |
| status bar | 28 (lower than the title bar) |
| activity bar | 40 wide |
| splitter | 1 px `line_strong`, 6 px grab area |
| scrollbar | a 6 px thumb, 4 from the edge |

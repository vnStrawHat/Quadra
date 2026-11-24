# Controls

Three families. A control belongs to one and looks like its family; it has no look of its
own.

| Family | What it is | The look |
|---|---|---|
| **Field** | where a value is typed or chosen: input, textarea, select, combobox, number, date, time, key, colour, path | `input_bg` fill, 1 px `line_strong`, radius 6, 28 tall, 8 inside left and right, 14 px text |
| **Button** | what is pressed to do something: button, icon button, toggle button, segmented group | no fill or a quiet one, radius 6 (4 at xs and sm), 28 tall, 8 inside |
| **Toggle** | a yes or no beside its words: checkbox, radio, switch | a 16 px shape (a switch 32 × 20), its words 8 after it |

A field beside a button, a select beside an input, a stepper beside a segmented group: the
same height, the same corners, the same border colour, their text on one baseline.

## States, the same for every control

| State | Field | Button (ghost) | Button (outlined, tinted) |
|---|---|---|---|
| rest | `line_strong` border | no fill | its border and fill |
| hover | no change (a field is not a target, it is a place) | `panel_hover` | `raised_hover` over its fill |
| press | | `press` | `press` |
| focus | border `border_focused` | a 1 px `border_focused` outline, by keys only | border `border_focused`, by keys only |
| invalid | border `error`, focused or not; the message under it | | |
| disabled | `panel` fill, `text_disabled` | `text_disabled`, no hover | the same, its fill kept |
| chosen / on | | label and icon `accent_text` | `accent_tint` fill, `accent_text` label |
| busy | | a spinner in place of its icon, not pressable | |

## Fields

- **Text input** (`input`): the field look, 240 wide alone, the width of its column in a
  `form()`. Text in the editor foreground, placeholder `text_3`, selection `selection`. An
  icon before the text is 14 in `text_3`, 6 from it. `.prefix(words)` puts words in `text_2`
  before the text (`ssh`), `.mono()` the mono face, `.small()` 12 px text in a 22 px field
  (bars), `.fill()` takes the room left in a row; `.reveal()` gives a password a 22 px eye
  inside its end. `.height(px)` only for a bar that has another row height.
- **Textarea**: the field look, three lines tall unless told (`.rows(n)`), 8 inside left and
  right and 6 above and below; it grows with its lines.
- **Select**: the field look with a 12 px up-down chevron in `text_2` at its end, 8 from
  the value; 160 wide alone. Its menu is the menu (surfaces.md) as wide as the field or
  wider, a 14 px check in `accent_text` on the chosen row. Searched (`.searchable(ask)`): a
  search head over the rows (surfaces.md, Menu).
- **Combobox**: a text input whose list of matches is a menu under it, 280 wide alone.
- **Number input**: a text input 140 wide alone with an up and a down chevron (10 px icons
  in 18 × 11 ghost buttons) stacked inside its end; the value in mono. **Stepper**
  (`.stepper()`): one field 128 wide, a 32 px `−` button, the value centred, a 32 px `+`
  button; the buttons are ghost with 16 px icons, only their outer corners rounded.
- **Date picker and time input**: the field look, 180 and 120 wide alone, a 14 px calendar or
  clock before the value; the calendar is a popover (cells 32 × 28, the chosen day
  `accent_tint`, today with a 1 px `border_focused`).
- **Key input**: the field look, 160 wide alone, a pointer cursor; the keys centred in mono;
  "Not bound" in `text_3`; "Press keys…" in `text_2` while it records.
- **Colour input**: a text input 120 wide alone, a 14 px swatch (radius 2, 1 px `line_strong`)
  before the hex in mono. **Colour picker**: the same field opens a popover: the area
  208 × 140 (radius 6), the hue bar 208 × 12, the hex input, swatches 20 × 20 (radius 4), 6
  apart; the chosen swatch ringed in `border_focused`.
- **File and folder picker**: a text input and a secondary button "Browse…" 8 after it, a
  14 px `file` or `folder` icon before its label.
- **Field wrapper** (`field(label, control)`): the label 12 px in `text` over the control, 4
  apart; `.required()` adds `*` in `error`; under the control, 4 apart, one 12 px line: the
  error (`error`, a 12 px alert icon before it) or else the help (`text_2`). The line's room
  is kept only when there is one.

## Buttons

Height by size (space.md); padding 4 at xs and sm, 8 at md and lg; icon and label 6 apart; the
label 14 (12 at xs and sm), weight 400.

| Variant | Look | Use |
|---|---|---|
| `Ghost` | no fill, `text` label | the default in bars, rows and toolbars |
| `Secondary` | 1 px `line_strong`, no fill | the default beside fields and in a foot (Cancel) |
| `Primary` | `accent` fill and border, `text_on_accent` label and icon, a shortcut after it the same at 70 %; under the pointer `accent_hover`, pressed `accent_press`; focused by keys, a `text` border (its own is the focus colour already). Off, it is its tint: `accent_tint` fill, an `accent` border at 42 %, `text_disabled` label | the one main action of a view |
| `Danger` | `error_tint` fill, an `error` border at 42 %, `error` icon | the main action when it destroys |
| `Link` | `accent_text` label, no fill | an action inside a sentence or a quiet "more" |

- One primary a view. In a foot: `Secondary` for Cancel, `Primary` (or `Danger`) for the
  answer.
- An **icon button** is 22 square (sm) with a 14 px icon, ghost, with a name a reader hears
  and a tooltip. `.muted()` puts its icon in `text_2` (chrome).
- **A find bar** (the code editor's, 1Term's, Markdown's) is one row on a popup's plane, 4
  inside it: the query, a field of the md step with the search icon, then 22 px icon
  buttons centred on it (the switches, the one before, the next, the cross), 2 apart, and
  the count in 12 px `text_2`.
- A **toggle button** on: label and icon `accent_text`; with a label also the `accent_tint` fill.
- A key hint (`.shortcut("Enter")`) follows the label in 12 px `text_2`.
- A dropdown trigger (`.caret()`) ends in a 12 px chevron.
- Busy (`.loading(true)`): a spinner in place of the icon, the label kept, not pressable.
- **Segmented group**: an outlined group (1 px `line_strong`, radius 6) of cells split by
  1 px lines, 28 tall with its border (md; sm 22 with 12 px labels; lg 32), cells 12 inside
  left and right (8 at sm); the chosen cell `accent_tint` with an `accent_text` label. Two to
  five options of one or two words; more is a select.

## Toggles

- **Checkbox**: a 16 px box (radius 4, 1 px `line_strong`); checked or mixed: `accent` fill
  and border with a 12 px check or a dash in `text_on_accent`. Its words 8 after it, the
  whole row pressable.
- **Radio**: a 16 px ring; on: the ring `accent` around an `accent` dot. A group is a
  `vstack()` of them, 8 apart; a hint under an option is 12 px `text_3`, indented to the
  words.
- **Switch**: a 32 × 20 track (off: `raised` with a `line_strong` border, knob `text` at
  50 %; on: `accent`, knob `text_on_accent`), for a setting that takes effect at once. In a
  settings row it is at the right; elsewhere its words are 8 after it.
- **Slider**: a 4 px track (radius 2) filled with `accent` up to a 12 px knob, in a box of
  20, 200 wide alone; its value, if shown, in mono 8 after it.

Choose by the question: one of two that acts now, a switch; one of two to five, a segmented
group or radios (radios when the options need a line of explanation); one of many, a select;
many of many, checkboxes.

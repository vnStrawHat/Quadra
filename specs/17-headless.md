# 17 — Headless Components

Components without a look (00, phase R6). A headless component is what a control does: its
state, its keys, its focus, what it tells a reader, how its popup opens and closes. It draws
nothing. A kit is a look over the headless crate: `quadra-design` is one, and a user makes another for the
design system they want without touching the headless crate.

The proof that this holds is that `quadra-design` is built on the headless crate alone, and that a second
kit of another look is built from it with no change to it.

## The crate

`quadra-headless` stands between `quadra-app` and `quadra-design` (02, "Crates"). It depends on
`quadra-prim`, `quadra-reactive`, `quadra-layout`, `quadra-platform` (keys, the clipboard),
`quadra-tree`, `quadra-view` and `quadra-async` (timers). It does not depend on `quadra-app`:
what it needs of the app (the clipboard, the window) comes through the context of views, as
in the kit today.

**No look in the headless crate.** Its sources name no colour, role, paint, radius, font or icon: no
`Token`, `Paint`, `Color`, `.bg(..)`, `.radius(..)`, `.border(..)`, `.elevation(..)`,
`.cursor(..)`. The gate checks it (a search over `crates/quadra-headless/src`). Sizes, gaps and
widths are the look's too, and are passed in.

## Parts

A headless component is a builder of its state (the signals the caller owns, 13,
"Components") and of what it does (disabled, a range, a step). It ends by taking the
**parts** the look has built and giving back a view: the parts with the behaviour on them.

- A part is a `Div` the look built (a checkbox's face, a popup's plane), or a function that
  builds one, for parts there are several of (an option of a radio group, a row of a menu)
  or that are built again (the content of a popup, each time it opens).
- The headless crate puts on a part what it does: the role, the name and the states for a reader, the
  focus and its scope, the key context and the actions, the handlers of the pointer, a
  node ref where it needs one. It adds no node of its own unless the behaviour needs one (a
  popup's region in the flow, its focus scope).
- **A look follows the state two ways.** What is paint follows the states of the node the
  base marks (`.when(State::CHECKED, ..)`, `SELECTED`, `EXPANDED`, `DISABLED`,
  `FOCUS_VISIBLE`, `HOVER`, `ACTIVE`). What is not (a knob that moves to the end, a fill as
  wide as the value, a row a search hides) follows a reader the headless crate gives the function that
  builds the part: a `Memo` (a copy costs nothing, a read is tracked), such as whether a
  toggle is on or a value's share of its range, or a plain value such as the index of an
  option. The caller's own signals are readers too.
- A part that shows inside another (a check inside a checkbox's box) reads the same
  readers: the headless crate does not reach into the look's nodes.

```rust
// A checkbox of a look: the face is a row with a box and a label; the headless crate makes it one.
let face = |on: Memo<bool>| row().gap(6.0).child(div().size(16.0, 16.0).bg(move || fill(on.get())))
    .child(text("Autosave"));
quadra_headless::toggle(checked).build(face)
```

## Numbers: the behaviour's and the look's

- **The behaviour's**, with a default the caller may change: how long a pointer rests
  before a tooltip (500 ms) or a hover card (400 ms) shows (`popup::hang` shows an overlay
  opened by hand against an anchor; `hang_with` against one read each time it shows, for
  what follows a part of a view, a link of a text), how long a card stays after the
  pointer left (150 ms), how far a press moves before it is a drag (4 px) or tears a tab off
  (12 px), how many rows a page key moves.
- **The look's**, always passed in: every size, gap, width and cap. A list is told the
  height of its rows; a menu how tall it may be before it scrolls; a popup where it stands
  (`Place`, its gap to the opener included); a column of a table how narrow it may get; the
  cursor over a control.

## Actions and the keymap

The actions (`Confirm`, `Cancel`, `MoveNext`, `MovePrevious`, `MoveFirst`, `MoveLast`,
`MoveInto`, `MoveOut`, `PageNext`, `PagePrevious`, `ExtendNext`, `ExtendPrevious`, `Copy`,
`Increase`, `Decrease`, `IncreaseMore`, `DecreaseMore`), their key contexts and
`keymap()` are the headless crate's (`quadra_headless::action`). Their names keep the prefix `kit::`:
nothing stores them, and an app's keymap that names them keeps working. The kit re-exports
`action` and `keymap`.

## Open tokens

A theme's names are its kit's: the headless crate names no role, and `quadra-tree` knows roles only as
numbers (13, "Tokens").

- **Shadows are the theme's.** What a level above the window casts is a list of shadows
  (how far down, how blurred, a colour) for each level, given by the theme
  (`Theme::with_shadows`). The table in the tree and `with_deep_shadows` go: the kit's dark
  and light themes give their sets. A level the theme has no set for casts none.
- **Measures are the theme's too.** A `Measure` is a number of the theme, as `Token` is a
  colour: a radius, a space, or a time in milliseconds, as its name says
  (`Theme::with_measures`; one the theme does not have is 0). A number of a style is an
  `Amount`, pixels or a measure: the radius, the padding of every side and the gaps take
  one. A style that holds a measure is kept as a role is, and resolved again when the theme
  changes: what comes out other lays out (spaces) or paints (radii) only the nodes that
  hold it. A transition's time may be a measure, read from the theme of the box when it is
  mounted. The kit names its measures (`token::RADIUS_SM` to `RADIUS_XL`, `SPACE_XS` to
  `SPACE_XL`: 4, 8, 12, 16, 24, `DURATION_FAST` 150 and `DURATION_SLOW` 400) and a theme file
  may give them other numbers under their names in lower case (`"measures": {"radius_md": 8}`).
  Its controls take their radii from them; the constants of `size` stay for arithmetic.
- **Type** stays constants of the kit (`quadra_design::size`, `fonts`): a family is a name the
  text system knows, and no theme file varies one yet.
- **What below the headless crate still has a look**: the scroll thumb (6 px, 4 px in from the edge,
  the text colour at 30 %), the default colours of text and selection (the kit's root sets
  them for a window). The thumb's numbers become values a root sets when a kit needs other
  ones.

## The components

What moves, from where, and the parts its look gives. A kit's module keeps the look: its
builder, its numbers, its roles, and the parts. What the headless crate gives that the kit wraps keeps
its name in the kit (`quadra_design::checkbox` stays).

| Base | What it does | Parts | From the kit |
|---|---|---|---|
| `action` | actions, contexts, `keymap()` | none | `action.rs` |
| `popup` | open beside an opener, at a place or along an edge; focus in, Tab inside, Escape, a press outside, focus back; `open_now`, `hang` | the plane, the content | `popup.rs` |
| `press` | a button: a click and `Confirm` do one thing, not while disabled or busy; pressed when a toggle. `press::toggle`: a checkbox or a switch, turned by a click on the face (its label in it), `Confirm` or a reader | the face | `button.rs`, `toggle.rs` |
| `choice` | one of a few: the group is one stop of Tab, the arrows go round | the group, an option (readers: index, chosen) | `choice.rs`, `tabs.rs` |
| `list` | selection, the arrows, Home, End, activation, rows mounted as they show; `places`, a few places chosen by id (a sidebar), the arrows among those that show | a row (index), the row height, the frame, what an empty list says; a place | `list.rs`, `sidebar.rs` |
| `menu` | rows that can be chosen, a highlight the arrows move, submenus, scrolling to the highlight, a choice closes all; `on_context`, a click with the other button | a row's face and the box around it, a part that is no row (a line, a heading), the box of the rows, the popup of a row's own menu | `menu.rs`, `menu/rows.rs` |
| `pick` | texts a query matches (`fuzzy`, `fuzzy_hits`, `best`); `Found`: the arrows over the matches with the focus in the field, Enter or a click takes one. `combo`: the combobox, typing opens the list, Escape or a press elsewhere closes it, the focus back in the field | the field, a row (the match), the box of the rows, what no match says, what goes before a row (a heading of its group: no row, the arrows pass it by); for a combobox the panel and its place | `combobox.rs`, `palette.rs`, `select/search.rs` |
| `hint` | a tooltip and a hover card: shown after a rest, gone on leaving or a press; a card stays while the pointer is on it | the bubble, the card | `tooltip.rs`, `hover_card.rs` |
| `dialog` | the page (a dialog to a reader), the cross that gives it up, an answer that acts and closes; it stands in a `popup` | the page, the cross, the buttons | `dialog.rs` |
| `notices` | `Queue`: what is shown, for how long (`TOAST`, 10 s), in what order; held while the pointer is on it (`hold`); closing, which returns what went (`close`), and putting another in a place (`replace`) | a card, a toast, the stacks | `notice.rs` |
| `slider` | `Scale` (ends, step, where the track is along the face); a value in a range by steps: a press and a drag on the face, the keys, a reader; `pair` and `end`, two knobs on one track | the face, the track, the fill, the knobs, placed by `Scale::part` | `slider.rs`, `range.rs` |
| `number` | `Number`: a value and the text of its field, which follow each other (`parse`: a comma is a point), steps within its ends, the arrows | the field (of `words`), its box, the buttons that step | `number.rs` |
| `keys` | `key_input`: a field that records a keystroke. Focused, it records (again on a click or `Confirm`); the next press that is no modifier, with its modifiers, is the value as `Keystroke` prints it (`ctrl-shift-t`), and it stops recording. Escape gives up and keeps the value; Backspace or Delete alone unbinds (the value is empty); Tab and Shift+Tab still move the focus; repeats do nothing. It sees keys before the keymap (`Tree::on_key_first`) | the face (a button to a reader, its value the keys) | `keys.rs` |
| `time` | `TimeOfDay`, the parts of a time and their keys (`typed`), a day of a calendar that a click chooses. Dates are the app's (`quadra_app::time::Date`): the month's weeks stay with the kit's calendar | a part's face, a day's face | `date.rs`, `time.rs` |
| `disclose` | what is open and closed: accordion sections | a head (reader: open), a body | `accordion.rs` |
| `split` | two panes and the share of the first: the drag, the least, a double click | the panes, the seam | `split.rs` |
| `table`, `live` | `Sheet`: the order of a sort, column widths, the selection, the keys, the activation of a row, the copy, the columns that fill (`fills`, weights); `live::watch`, the selection kept by key and the cells that changed noted (`FLASH`) | the frame, a title, its grip, a row, the cells, the tints | `table.rs`, `table/*` |
| `tree` | `Branch` (a thing's id, what is in it, what its row shows); the open things, the rows that show, the selection, the keys | a row's face (the thing, its depth, open; whether selected, whether the tree has the focus), the frame | `tree.rs` |
| `rules` | `Rule`, `validate` | none | `field.rs` |
| `settings` | `Model`: pages, groups, rows known by their text; a search that keeps rows whatever the accents, the page that shows, a group scrolled to; `acts`, `opens` for the column's rows; `Reset` (whether a row differs from its default, and what puts it back; `Reset::of(value, default)`), and `reset_all(rows)`: whether one of the rows a search keeps differs, and what resets those | the column, a page, a group, a row | `settings.rs`, `settings/*` |
| `work` | `Pane` (the tree and where its parts stand, the seam's width given), `Dock` (what a drop asks), the view of the tree (a seam dragged, `LEAST`, a double click; a pane that takes tabs), `Strip` (a tab chosen, moved after `SLIDE`, closed by its cross or the middle button, torn off after `TEAR` and taken where it is dropped) | a tab's face, its cross, the lane of the tabs, the chip that follows the pointer; `Parts` of the tree: the seam's width and face, the veil | `work/*` |

What stays in the kit, as it is only a look: `display`, `notes`, `bars`, `titlebar`,
`status`, `progress`, `icon`, `icons`, `fonts`, `theme`, `token`, `size`, `root`, and `diff`
(its line script, `diff_lines`, is plain data and stays with its rows; its folds are buttons
of the headless crate). A click that calls one function of the caller's stays a handler on the look's
face, as there is nothing of a component in it: a link of a breadcrumb, a tool of the
activity bar, the copy of a note, the chevron of a group of settings, the clear cross of an
input, the buttons of a title bar.

A row that is the headless crate's does not wait for all of them: each moves in its own commit, and
the kit uses it from then on.

## Widget names

The modules are named by what a component does (`press`, `choice`, `disclose`, `hint`,
`combo`), and one behaviour is several widgets: a checkbox and a switch, radios and tabs.
The crate also names them as widgets, at its root (`widgets.rs`): each is the behaviour with
what makes it that widget, so a look cannot give a radio group the role of a tab list.

| Widget | Is |
|---|---|
| `button(face)` | `press::press` |
| `checkbox(on, face, disabled)`, `switch(..)` | `press::toggle`, a checkbox or a switch |
| `radio_group(group, count, chosen)`, `radio(face, i, chosen)` | `choice` with `Role::Group` and `Role::Radio` |
| `tabs(strip, count, chosen)`, `tab(face, i, chosen)` | `choice` with `Role::TabList` and `Role::Tab` |
| `accordion_head(face, open)`, `accordion_body(open, body)` | `disclose` |
| `tooltip(cx, of, rest)`, `hover_card(cx, of, rest, grace)` | `hint` |
| `combobox(..)` | `combo::combo` |

`dialog`, `menu`, `list`, `tree`, `table`, `slider`, `number`, `key_input`, `split`, `popup` and
`settings` are named as widgets already. A look uses the widget names; the behaviour's stay
for what is none of them (a disclosure that opens a date picker).

## Tests

- **What the kit does does not change.** No test in `crates/quadra-design/tests` changes while
  the headless crate moves out, but for its imports. The gallery's pictures in both themes
  (`tools/shots.sh`) differ by no pixel from those taken before the first move, kept in
  `.tmp/shots/r6-dark` and `r6-light` and compared with `QUADRA_REFERENCE`.
- **The headless crate** has a test of each component through the semantic tree (20, in six files),
  with parts that are bare boxes: no look and no pixel.
- **The second kit** has its own tests, through the semantic tree too.

## The second kit

An example crate, `examples/plain` (`quadra-plain`): a look of its own, square and flat with
outlines, made from the headless crate and from boxes and texts, with no code of `quadra-design`. It has
a button, a checkbox and a switch, a radio group and tabs, a slider, a select with its menu,
a dialog and a list: the parts every kind of headless component gives. Its window shows them,
and its tests drive them through the semantic tree. It is not a second gallery.

## Order

1. This spec.
2. Shadows in the theme.
3. `action` and `keymap`; `popup`; `choice`. These are what the rest uses.
4. The controls: `press` (with `toggle`), `list`, `disclose`, `split`.
5. The popups: `menu`, `pick`, `hint`, `dialog`, `notices`.
6. The values: `slider`, `number`, `time`, `rules`.
7. The data: `table`, `tree`, `settings`, `work`.
8. The second kit.
9. The renderer (00, R6).

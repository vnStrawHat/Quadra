# 13 — Style, Themes and the Kit

The look and behaviour of components are those of behaviour/09, 15, 16 and 19. This spec says how
they are built on the new architecture.

## Style values

A style is a struct of optional values in three groups:

- **Layout** values (06): sizes, spacing, flex and grid properties, position, overflow.
- **Paint** values: background (colour or gradient), border colour, radii, shadows, text colour
  and decoration, opacity, cursor, transform.
- **Text** values, inherited down the tree: family, size, weight, line height, features.

A node's declared style is a base plus partial styles per state (05). The style pass resolves
them in a fixed order: base, then `hover`, `focus`, `focus_visible`, `active`, `selected`,
`checked`, `expanded`, `drop_target`, `disabled`, each applied if its bit is set. Inherited text
values come from the parent's resolved style.

As built, for text: a node may say text values (family, size, weight, italic, line height,
colour), each one or not; what holds at a node is its own value, else that of the nearest
node around it that says one, else the default. A text and a text field have no style of
their own: `text(..).size(12.0)` is a value of the text's own node. Boxes say them with
`.text_color(..)`, `.text_size(..)`, `.font_weight(..)`, `.italic(..)`, `.line_height(..)`,
`.font_family(..)`, the first five in state variants too; the colour may be a role.

- A widget is given the values that hold at its node (`Widget::inherit`) before it is next
  measured or painted after they may have changed, and keeps them: the tree counts the
  changes to text values and to where nodes are, and a widget is given them again when the
  count has moved on. A frame in which nothing of the kind changed costs each text one
  comparison.
- A node whose text values change marks the widgets in it and under it that have the
  changed value from it: for layout when it shapes text, for paint only when it is the
  colour. A node on the way that says the value itself keeps it, with what is under it.
- A node that leaves its parent has the texts under it measured again where it comes to be.

A partial style is a list of values, each one case of an enum with a case per setter
(`Patch::Background(colour)`, `Patch::Padding(8.0)`): plain data, a few words each. The same
value is what a reactive property writes when its source changes, so a property bound to a
signal allocates nothing per change. A node without variants keeps only its resolved style; a
node with variants also keeps its own style, which is what a property writes, and resolves
again when that or a state changes. A state that changes on a node with no variant for it
does nothing at all.

## Tokens

A colour, radius, spacing, font or duration in a style is a literal or a **token**:

```rust
div().bg(token::SURFACE).border_color(token::BORDER).radius(token::RADIUS_MD)
div().bg(token::ACCENT.alpha(0.12))
```

- A token is a small id. A style holding tokens is plain data; no closure, no binding.
- The style pass resolves tokens against the theme in the node's context. A subtree may
  provide another theme (a dark panel in a light window).
- Changing the theme flags the affected root with "restyle all". Resolved styles are diffed as
  usual: a theme that changes only colours lays out nothing.
- As built, for colours: a colour of a style is a `Paint`, which is a `Color` or a `Token`
  with an opacity (`ACCENT.alpha(0.12)`); every setter that takes a colour takes either,
  constant, in a state variant, or reactive. `Token` is a number and `Theme` the colour of
  each number, both in `quadra-tree`, which knows no role by name: the names are the kit's.
  A role the theme has no colour for is transparent.
- A node keeps the values of its style that are roles beside its own style, and they are
  applied over it with the theme of the moment, before the variants of its states. A colour
  given after a role for the same property takes its place. A node whose style holds no
  role keeps nothing for the theme and is not looked at when the theme changes.
- `Tree::set_theme` styles again the nodes whose style holds a role; an equal theme does
  nothing. The app has one theme for all its windows, as a signal in the context of every
  view (`Signal<Rc<Theme>>`): writing it is how a view changes the theme.
- Not yet: a theme for a subtree; tokens for radii, spacing, fonts and durations.
- Component-level tokens (`button.primary.bg`) resolve to palette tokens in the theme file, so
  a theme can restyle one component without code.

## Themes

Themes stay JSON files (behaviour/15) with the same structure: palette, type scale, spacing,
radii, component styles. They are loaded and validated at start-up; an unknown key or a
malformed value is reported with its path. The system appearance (dark, accent, high contrast,
text scale) arrives from the platform (10) and can select or adjust the theme.

As built: `quadra_kit::token` names the roles of the palette (`token::CANVAS`, `ACCENT`, ..),
each a `Token` numbered in the order of the table, and `token::NAMES` their names in a file.
`theme_from_json(text)` reads a theme file into a `KitTheme` (name, mode, a colour a role),
starting from `dark()` or `light()` by the file's mode; the two built-in themes are files
bundled in the crate (`themes/one-dark-pro.json`, `themes/one-light.json`). Roles a file
leaves unset follow its own colours (the table of behaviour/15 says which follows which).
`KitTheme::theme()` is what a tree styles with. The sections of code, the terminal and
charts are checked to hold colours and are not kept yet: their roles come with their crates.

An app uses the kit with `quadra_kit::install(app, &theme)`, which loads the bundled faces
(`fonts::UI`, `fonts::MONO`; eight files, 1.6 MB of the binary) and gives the app the theme,
and with `quadra_kit::root()` as the box at the root of each window: the canvas, and the UI
text (face, 14 on 23, the theme's text colour) for everything in the window. The numbers of
the design language (type scale, control heights, radii, weights) are constants in
`quadra_kit::size`; a theme does not vary them yet. Not yet: the UI face shaped without
contextual alternates.

## Components

A component is a builder type with a function to create it:

```rust
button("Save").variant(Variant::Primary).icon(Icon::Save).disabled(move || !dirty.get())
    .on_click(move |_| save.emit(()))
```

Conventions:

- **Inputs that can change are reactive** (a signal, memo or closure); the rest are plain
  values. A component never needs to be built again to change.
- **Controlled by default**: a value control takes the signal that holds its value
  (`checkbox(checked: Signal<bool>)`). A caller that wants to intercept passes a read closure
  and an `on_change` handler instead.
- **Composition by children and slots**: `card().header(..).child(..).footer(..)`.
- **Built from built-in widgets only**: a component adds no widget type unless it paints or
  lays out in a way boxes and text cannot (a slider track is boxes; a colour wheel is a canvas).
- **Semantics are part of the component**: role, name, states and actions are set where the
  component is defined, and its tests find it by them (12).
- **Keyboard behaviour is actions**: a component handles `Confirm`, `Cancel`, `MoveUp` and the
  like, bound in the kit's default keymap, never raw keys (09).
- **No state by position**: open, selected and scroll state live in signals the caller can own,
  or on the node.

The gallery shows every component in every variant and state, and is the acceptance test of
the kit: its pages are compared with the reference screenshots (00, phase R3).

As built so far:

- **Actions and keys.** `quadra_kit::action::Confirm` is what Enter and Space mean on a
  control (the key context `action::CONTROL`); `quadra_kit::keymap()` is the kit's bindings
  over those of every window, and `install` gives them to the app.
- **`button(label)`**: `.variant(Primary | Secondary | Outline | Ghost | Danger | Link)`,
  `.size(Xs | Sm | Md | Lg)`, `.disabled(..)`, `.toggled(..)`, `.full()`, `.test_id(..)`,
  `.on_click(|| ..)`. A click and `Confirm` do the same thing. Its colours are roles: the
  fill, the label, the fill under the pointer and while pressed, the border (behaviour/15,
  "States"); a tinted button's hover is a role of its own (`accent_tint_hover`,
  `error_tint_hover`: the tint 8 % towards its colour unless a theme says). The label is a
  text that takes the button's text colour. To a reader it is a button named by its label,
  pressed or not when it is a toggle.
- **Icons.** `icon(icons::CHECK)` shows one of the kit's Lucide outlines (`quadra_kit::icons`,
  87 of them; ISC, `LICENSE-lucide.txt`), 14 px unless `.size(..)` says, in the colour of the
  text around it unless `.color(..)` says. A body is read once (`quadra-vector`); its mask is
  rasterized once for each size in device pixels and kept as a tile of the coverage atlas
  beside the glyphs (`Fonts::mask`, `TileKey::Mask`); showing it is one instance. The widget
  is `Drawing` in the tree, and `drawing(svg)` the view for any drawing. A button takes
  `.icon(body)` before its label (14 px, 16 in the two larger sizes), and
  `icon_button(body, name)` is a ghost of 22 px with the icon alone, named for a reader.
- **`input(value)`**: the field of `quadra-ui` in a frame (the field's fill, a hairline
  border, 8 by 6 px around a line of text, at least 32 high, 240 wide unless told):
  `.placeholder(..)`, `.password()`, `.icon(body)` before the text, `.invalid(..)`,
  `.disabled(..)`, `.width(..)`, `.on_submit(..)`, `.test_id(..)`. The border is where the
  focus shows; a value that is not accepted has the colour of errors with the focus or
  without (the state `INVALID`, applied after every other); a disabled field is a step back
  and takes neither the focus nor text. A reader hears the placeholder as its name.
- **`checkbox(checked)` and `switch(on)`** take the signal that holds their value and write
  it: by a click on the control or its label, by `Confirm`, or when a reader toggles them.
  `.label(..)`, `.disabled(..)`, `.test_id(..)`. The checkbox is a box of 16 px with a
  hairline border and an accent check; the switch a track of 32 by 20 inside a ring one
  pixel off it, with a knob of 12 px that is at the end, over the accent, when on.
- **`radio_group(options, chosen)` and `segmented(options, chosen)`** hold one of a few and
  write its index: a click on an option, or the arrows while the group has the focus
  (`MoveNext`, `MovePrevious` in the key context `action::CHOICE`), round the ends. The
  group is one stop of Tab. Radios are rings of 12 px with a dot; segmented cells belong to
  one outlined control, the chosen one tinted with an accent label (`.size(..)`: 22 px with
  12 px labels, or 28 and 32 with 14).
- **`tabs(labels, chosen)`** is the names of a few pages in a row, with the index of the
  chosen one in a signal; the caller shows its page. A tab is 28 px tall with 8 px beside
  its label, muted until the pointer is over it; the chosen one has the accent's label. A
  line runs under the row, or with `.pill()` the chosen tab is tinted and there is none.
  The row is one stop of Tab and the arrows go round it, as in a radio group (the same
  code: `choice::chooser`). To a reader: a tab list of tabs, one selected.
- **`list(count, selected, row)`** is any number of rows of one height (23 px, or
  `.row_height(32.0)`), mounted as they show (`uniform_list`). `row` builds what a row holds
  from its index, a signal. `selected` holds the selected row if any: a click writes it, and
  with the focus in the list the arrows move it and stop at the ends, Home and End go to
  them (key context `action::LIST`), the list scrolling to keep it in view. A double click
  or Enter is `.on_activate(|row| ..)`. A row is as wide as the list, 6 px beside what it
  holds, filled under the pointer and with `press` when selected. `.empty(words)` is what a
  list with no rows says.
- **What shows and does nothing** (`display.rs`) is functions that give boxes and texts:
  `heading(words, level)` (23, 18, 16 px, then the size of text; regular weight), `label`
  (12 px, muted), `badge(words, tone)` and `tag(words)` (chips of 16 and 18 px in 10 px mono
  text; a badge is a faint tint of its tone), `kbd("ctrl-s")` and `key_label` (shortcuts are
  written "Ctrl-Shift-P", the modifiers in the order Ctrl, Win, Alt, Shift), `avatar(name,
  side)` (the initials on a colour that is the name's own; images later), `card()` with
  `card_head(title, more)`, `divider()` and `vdivider()`.
- **`accordion().item(title, open, || body)`**: sections under one another in one outline.
  A head is 28 px, a chevron then the title, and a button to a reader, expanded or not; a
  click, Enter or Space writes the signal that says whether its section is open. The body
  is built each time it opens.
- **`split(first, second, share)`**: two panes side by side, or `.stacked()`, and a seam of
  one pixel in a grip of 6 with the cursor of a resize. `share` is the first pane's part
  of the whole, a signal: dragging the seam writes it, never leaving a pane under
  `.least(px)` (80), and a double click makes the panes even. What does not fit in a pane
  is cut.
- **`titlebar(title)`** is the bar of a window with its own frame (10): 32 px on the chrome
  plane over a strong line, 12 px text, and the window is dragged by it. `.menu(|| items)`
  puts the app's menu behind a button at its left end, `.icon(body)` an icon before the
  title, `.control(view)` things of the app's after it, which work as anywhere;
  `.title_size(16.0)` makes it the caption of a small window (34 px). At its right are
  minimize, maximize (restore while the window is maximized) and close: 36 px wide, their
  glyphs at 10 px from the system's face of caption glyphs when it has one, drawn icons
  otherwise; close turns `#E81120` under the pointer. They ask the window (`window()`),
  and are buttons to a reader.
- **The bars** (`bars.rs`). `toolbar()` is a box of 45 px on the canvas with a line under
  it, its children 4 px apart. `statusbar().left(view).right(view)` is the strip of 30 px
  at the bottom of a window, in 12 px text; in it stand `status_item(icon, words)`,
  `status_button(icon, name, on)` (an icon toggle: muted, the accent while on, never
  filled; a toggled button that is an icon alone takes no fill anywhere) and
  `status_divider()`. `breadcrumb(parts, pick)` is the way to where the user is: mono,
  muted names with a mark between them, each a link that gives `pick` its index, but the
  last.
- **A status** (`status.rs`: running, pending, failed, succeeded, terminating, unknown) has
  a role of the theme for its colour and a shape of its own in a box of 12 px
  (`status_mark`): a dot, a ring, a triangle, a check, a dash, a square. `status_label` is
  the mark and the name, `Status::of(word)` the status a word names.
- **`sidebar(chosen)`** is the places of an app (`NavItem::new(id, label).icon(..)`) in
  groups under muted titles, on the panel's plane with a strong line at its right, 240 px
  wide unless told. `chosen` holds the id of the chosen place: a click writes it, and with
  the focus in the sidebar the arrows, Home and End move it. A place is a row of 26 px,
  filled when chosen. While `.folded(|| ..)` says so it is 40 px wide and shows the icons
  alone; the places keep their names for a reader. `.head(view)` is above the places, and
  what does not fit scrolls.
- **`slider(value)`** holds a number between two ends (`.range(min, max, step)`; 0 to 100
  by 1 unless told): a track of 4 px, filled with the accent up to a knob of 12 px, in a
  box of 20 px, 200 wide unless told. A press on the track takes the value there, a drag
  goes on from it wherever the pointer is; with the focus the arrows step it, Page Up and
  Page Down go a tenth of the way, Home and End to the ends (key context `action::SLIDER`).
  To a reader it is a slider with its value, which it can step or set.
- **`range_slider(low, high)`** is two knobs on one track for the ends of a range, filled
  between them. A press or a drag moves the knob nearer to the pointer, so a drag past the
  other knob goes on with that one; each knob is a slider of its own to the keys and to a
  reader ("Minimum", "Maximum"), and goes as far as the other lets it. It shares the scale
  of the slider (`slider::Scale`).
- **`combobox(options, value)`** is an input, 280 px, that offers under itself the options
  its text matches (as the command palette matches: in order, whatever the case and the
  accents, the best first, the matched letters in the accent): rows of 23 px, at most 240
  px of them. Typing opens the list; the focus stays in the input, the arrows go round the
  options, Enter or a click takes the highlighted one (`.on_pick(|index| ..)`) and writes
  it to the value, Escape or a press elsewhere closes the list.
- **Dates** (`date.rs`). A `Date` is a year, a month and a day, written day, month, year
  (`05/03/2026`); it knows the days of its month and its day of the week. `calendar(value)`
  is a month on an overlay's plane: arrows to the months around, the names of the days from
  Monday, and the days in weeks of seven, each a cell of 32 by 28 that a click chooses; the
  chosen day is tinted with an accent label. The arrows change the month that shows, not
  the date. `date_picker(value)` is a field of 180 px that shows the date and opens that
  calendar in a popup; a day chosen closes it. Not yet: today (the platform has no local
  time), the arrows of the keyboard over the days, names in the app's language.
- **`time_input(value)`** edits a `TimeOfDay` in a field of 120 px: hours and minutes are
  two parts that take the focus each, sliders to a reader. The arrows step the part with
  the focus, round its ends (`.minute_step(..)` for the minutes), Home and End go to its
  ends, and digits type it: two in a row make a number of two when that is a value of the
  part.
- **`number_input(value)`** is an input that holds a number (`.range(..)`, `.step(..)`,
  `.decimals(..)`), 160 px with a pair of chevrons that step it, or with `.stepper()` one
  frame of 128 by 32: a minus, the value in the middle, a plus. The up and down arrows step
  it too (key context `action::NUMBER`). A number typed, with a point or a comma, is the
  value; what is no number changes nothing and shows as not accepted; a value written from
  elsewhere is what the field says.
- **`field(label, control)`** puts a control under its label, 12 px, with `.required()` a
  mark after it; under the control is `.help(words)` or, while it is not empty, what
  `.error(..)` says, in the colour of errors after a mark. `validate(value, &rules)` gives
  what the first broken `Rule` says (`Required`, `MinLen`, `MaxLen`, `Email`): plain data.
  `textarea(value, rows)` is the text area of `quadra-ui` in the frame of an input, at
  least that many lines tall.
- **What is under way** (`progress.rs`). `progress(part)` is a bar of 6 px on the chrome
  plane, filled with the accent as far as `part` (0 to 1, a constant or reactive), and a
  progress bar with its percentage to a reader. `progress_busy()` is the bar for what has
  no end in sight: a third of it crosses the track every 1.4 s. `spinner()` is an open ring
  that turns once in 0.9 s, in the colour of the text around it. `skeleton(width, height)`
  is a block that stands for what is on its way, and pulses. In the three that move, what
  moves is a layer and each frame writes one constant of it (an offset, a turn, an
  opacity): nothing is laid out, painted or uploaded, at the exact place of every frame.
- **`table(columns, rows, cell)`** (`table.rs`, `table/view.rs`) shows `rows()` rows of
  cells under a header; `cell(row, column)` is what a cell says, and both are asked again
  when what they read changes. Rows are 26 px with a seam under each and are mounted as
  they show (`uniform_list`); the header is 28 px on the plane of table heads. A `Column`
  has a title, a width (140 px; `.width(..)`), and how it shows its cells: text, numbers
  (`.numeric()`: at the right in the mono face, sorted as numbers), `CellKind::Status`
  (the word with its mark and colour) or `CellKind::Age` (seconds as 45s, 12m, 3h, 6d). A
  click on a title sorts by its column from the least, another from the most, with an
  arrow in the accent; the order is worked out again when the data changes. The 5 px at
  the edge of a title are dragged to make the column wider, never under 48 px. A click
  selects a row, with Ctrl one more or one less, with Shift from the last one clicked; the
  arrows, the page keys, Home and End move the row the keys are on and select it, Shift
  with an arrow grows the selection, and Ctrl-C copies the selected rows as lines of cells
  with tabs between (key context `action::TABLE`). The selection is of rows as the data
  numbers them, in a signal the caller may own (`.selected(..)`). Not yet: scrolling
  sideways and pinned columns, hidden columns, a cursor on cells, cells that flash when
  they change.
- **`tree(roots, chosen)`** shows things inside things (`TreeNode::new(id, label)
  .icon(..).children(..)`) as rows of 26 px: 20 px in for each level, with a faint guide for
  each, then a chevron if the thing holds something, its icon and its label. `roots` is
  asked again when what it reads changes; the rows that show are mounted (`uniform_list`),
  and a row is built again only when the thing at its place is another one, or opens.
  `chosen` holds the id of the selected thing: a click selects, and on what holds something
  a second click or a double one opens or closes it. With the focus in the tree the arrows
  move the selection, Home and End go to the ends, the right arrow opens what is closed
  then goes into it, the left one closes what is open then goes out to what holds it, and
  Enter and Space open and close (key context `action::TREE`). The ids of the open things
  are a signal the caller may own (`.open(..)`). To a reader: a tree of items, expanded or
  collapsed. Not yet: children that are asked for when a thing opens, several selected.
- **The keyboard's focus on a control without a border of its own** is a line of one pixel
  around it: `outline`, a paint value that takes no room (it is drawn over what the node
  paints, along its edge).
- **Colours of fields are text values** (`placeholder_color`, `selection_color`,
  `caret_color` on boxes, roles if wanted): the kit's root sets them for the window, an input
  its own selection. A field shows its placeholder while it holds nothing, composing counted.
- A component that builds a view from properties it was given passes them on as they are:
  `Prop<T>` is itself something a setter takes.
- The example `gallery` of the kit (`examples/gallery/`) shows the components in a real
  window in either theme: a window with its own title bar, a sidebar of pages, a toolbar,
  a status bar and a command palette. It
  becomes the gallery app when it is compared with the reference screenshots.

## Overlays in the kit

Popover, menu, context menu, tooltip, select, combobox, dialog, drawer, toast and the command
palette are built on `overlay` (05): placement against an anchor, a focus scope, dismissal on
outside press or Escape, return of focus to the opener. One implementation of each of those
four behaviours serves all of them.

As built: `popup.rs` is that implementation. A popup is shown while a signal is true; when
it opens the focus goes to the first thing in it that can have it (or to its panel), Tab
stays inside, Escape is the action `Cancel` (key context `action::POPUP`), and a press
outside closes it. A press on its opener is not a press outside (`overlay(..).opener(..)`):
the button that opened a menu closes it by its own click and does not open it again. When
the popup closes the focus goes back to what had it.

- `popover(open, &opener, || content)` is the bare popup, below its opener.
- `menu(open, &opener, || items)`: rows of `MenuItem` (`.icon(..)`, `.keys("Ctrl-C")`,
  `.checked(..)`, `.disabled(..)`, `.on_select(..)`, `MenuItem::separator()`), asked for each
  time the menu opens. A click on a row does what it does and closes the menu; the arrows
  move a highlight over the rows that can be chosen, round the ends, and Enter or Space
  chooses (key context `action::MENU`). Rows are 23 px, 4 px in from the edge; an icon
  column is there only when a row has an icon or a check.
- `select(options, chosen)`: a filled button that shows the chosen option, and a menu of the
  options with a check on it.
- More of the menu. `MenuItem::heading(name)` names the group of rows under it: 20 px,
  muted, never chosen. `.submenu(items)` makes a row one that opens a menu of those rows
  beside itself, at its right and over it by 2 px, with a chevron at its end: under the
  pointer, by a click or Enter, or by the right arrow (`MoveInto`); the pointer over
  another row closes it, as does the left arrow in it (`MoveOut`), and a choice in it ends
  the menus together. A menu taller than 450 px shows that much and scrolls; the highlight
  the arrows move is brought into view.
- `context_menu(view, || items)` gives a view a menu that opens a pixel off the pointer on
  a click with the other button, where it fits in the window. It adds no node around the
  view (as the tooltip): the popup is opened by hand, from the handler
  (`popup::open_now`), and goes with the scope of the view if it is still open then.
- The region a popup is shown in is not displayed (`.display(Display::None)`): where the
  popup stands in the flow it takes no room, not even a gap of the box around it.
- The text values of the window's root hold in overlays too (a popup's text has the
  window's face and colour): an overlay has no parent, and is above the root.
- `tooltip("Save the file", view).keys("Ctrl-S")`: the words show below the view, centred,
  once the pointer has rested on it for half a second, and go when it leaves or presses. A
  tooltip never has the focus. It adds no node: the tooltip's node is the view's own, the
  handlers are put on it, and the bubble is an overlay opened by hand.
- `dialog(open, "Delete the file?", || body).action("Cancel", Variant::Ghost, f)..`: a popup
  440 px wide, centred, 80 px under the top of the window (`Place::Top`), with no backdrop:
  a title, the body in the second text colour, a seam, and the buttons at the right of its
  foot. A button runs what it was given and closes the dialog; Escape and a press outside
  give it up.
- Elevation is a paint value of a box (`.elevation(1)`): a level above the window casts the
  shadows of the design language (behaviour/15) under the box, two for the level of menus,
  popovers and tooltips, four for that of dialogs. A change of level is a paint change.
- **Notifications and toasts** (`notice.rs`). The app holds a `Notices` (`notices()`, made
  where the window's view is built; a copy is the same one), mounts `.host()` once in the
  window, where it takes no room, and tells it what to show from anywhere:
  `.notify(Notice::new(title).body(..).tone(..).action(label, f).primary(label, f)
  .lasts(time))` is a card of 448 px on the panel's plane at the level of dialogs, with the
  icon of its tone, a close button, and its buttons at the right, each of which does what it
  says and closes the card. Cards stack in the bottom right corner, 12 px from the edges
  (`.above(30.0)` lifts them over a status bar), the newest the lowest, 8 px apart, and
  stay until closed unless they say how long they last. `.toast(tone, words)` is a line
  with its icon at the bottom centre for ten seconds. The stacks are overlays that take
  neither the focus nor a press outside them (`Place::BottomRight`, `Place::Bottom`).
- **`command_palette(open, || commands)`** is a popup of 608 px under the top of the window
  at the level of dialogs: a query row of 36 px over a line, and under it the commands that
  match, rows of 32 px, at most 347 px of them, the rest scrolled to. A `Command` has a
  title, the keys that run it too (shown muted at the end of its row, as shortcuts are
  written), and what it does. A command matches when the letters of the query are in its
  title in order, whatever their case and accents (`fuzzy_hits`; "hai ph" finds "Hải
  Phòng"); letters at the start and one after another count for more (`fuzzy`), and the
  best match is first and highlighted. The letters that matched are in the accent
  (`marked`: a row of texts). The arrows go round the matches with the focus still in the
  query, Enter or a click runs one after the palette has closed and given the focus back.
- Not yet: drawer; a picker of other things than commands; the menu key for a context menu; a toast that waits
  while the pointer is on it; a tooltip told to assistive technology as the description of
  its view.

## Crates

```
quadra-kit     buttons, inputs, selection controls, lists, tabs, panels, overlays, feedback,
               navigation, titlebar, layout helpers, themes, icons
quadra-data    table, tree, property grid, log view, value viewer         (behaviour/16, 19)
quadra-viz     charts                                                      (behaviour/19)
quadra-term    terminal: parser, screen, scrollback, surface widget, Pty   (behaviour/17; on hold, 00)
quadra-code    code editor: buffer, syntax tokens, surface widget          (behaviour/18)
quadra-md      Markdown: parser, renderer, math, diagrams                  (behaviour/21)
```

- `quadra-kit` holds what almost every app uses. The others are separate so an app pays for
  what it shows, in binary size and build time.
- The V kit kept all of these in one module of 34,000 lines.

## Domain crates and the framework

Terminal, editor and large canvases are **surface widgets** (04, 07): they own a layer, write
instances for what changed, and expose semantics for their content.

- Their logic (the escape-sequence parser, the screen model, reflow, the text buffer, the
  Markdown parser) is plain Rust with no dependency on the framework, tested alone. The V
  implementations of that logic, with their tests, limits and corpora, are ported closely: that
  code is algorithmic and was never shaped by the UI architecture.
- Their widgets are new, written against the new renderer and input.
- State they show (a terminal's screen) is a signal mutated in place with `update`, which
  flags the widget for paint. The widget keeps its own damage tracking to repaint only changed
  rows.

## Porting the kit

Order within phase R3: tokens and themes; text, box and icon; buttons and inputs; selection
controls; overlays; lists and tabs; panels and the titlebar; the gallery as each lands.

What a component brings from its V version: its look, its states, its keyboard behaviour, its
semantics, its tests' intent. What it does not bring: its code structure. The V components
were builders evaluated during layout, with handlers passed as data; none of that shape is
kept.

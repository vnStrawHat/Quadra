# 13 — Style, Themes and the Kit

The look and behaviour of components are those of behaviour/09, 15, 16 and 19. This spec says how
they are built on the new architecture.

## Style values

A style is a struct of optional values in three groups:

- **Layout** values (06): sizes, spacing, flex and grid properties, position, overflow.
- **Paint** values: background (colour or gradient), border colour, radii, shadows, text colour
  and decoration, cursor.
- The **pose** (05, "The pose and transitions"): translation, scale, rotation and opacity.
  They are constants of the node's layer, not paint.
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
- The colours of code are roles too: a theme file's `syntax` gives each of
  `token::SYNTAX_KEYWORD`, `SYNTAX_STRING`, `SYNTAX_NUMBER` and the rest its colour (a name
  that is none of them is an error), for the editor, the markdown of code and the value
  viewer; and so are the sixteen colours text asks for by number (`token::ANSI`, a file's
  `terminal.ansi`), for the log and the terminal, and the tints of lines added and removed
  (`token::DIFF_ADD`, `DIFF_REMOVE`, a file's `editor.diff_add` and `editor.diff_remove`),
  for the diff and the editor.
- A node keeps the values of its style that are roles beside its own style, and they are
  applied over it with the theme of the moment, before the variants of its states. A colour
  given after a role for the same property takes its place. A node whose style holds no
  role keeps nothing for the theme and is not looked at when the theme changes.
- `Tree::set_theme` styles again the nodes whose style holds a role; an equal theme does
  nothing. The app has one theme for all its windows, as a signal in the context of every
  view (`Signal<Rc<Theme>>`): writing it is how a view changes the theme.
- The colours of charts are roles too: `token::SERIES`, eight of them, from the theme file's
  `series` (one it does not list is the accent).
- The shadows of a level above the window are the theme's (17, "Open tokens"): the kit's
  dark theme casts deeper ones, as a light shadow does not show on a dark plane.
- A box gives what is under it a theme of its own (`.theme(rc)`, `Tree::set_theme_of`):
  styles under it resolve their roles with it, and widgets paint with it. What is put
  under it later takes it too; the window's theme leaves it as it is. Where no box has
  one, nothing is looked up (the list of them is empty). A popup opened from inside is
  not under the box and has the window's theme.
- Radii, spaces and times are measures of the theme (17, "Open tokens"); faces are not.
- Component-level tokens (`button.primary.bg`) resolve to palette tokens in the theme file, so
  a theme can restyle one component without code.

## Themes

Themes stay JSON files (behaviour/15) with the same structure: palette, type scale, spacing,
radii, component styles. They are loaded and validated at start-up; an unknown key or a
malformed value is reported with its path. The system appearance (dark, accent, high contrast,
text scale) arrives from the platform (10) and can select or adjust the theme. As built:
`Appearance::colors` are the colours of the system's controls (`SystemColors`: the window
and its text, the highlight and its text, links, disabled text, buttons; `GetSysColor` on
Windows); `high_contrast(colors)` is a theme of them (planes the window's, text and lines
its text, what is chosen, focused or the accent the highlight, translucent where it tints,
the quiet text as plain as the rest; the terminal and the series of charts keep theirs),
and `system_theme(appearance)` is it while high contrast is on, the dark or the light theme
otherwise. `install_system` follows it. The system's accent is not taken by default (the
design language has its own); `KitTheme::with_accent(color)` gives a theme another. The
gallery's pictures of high contrast: `tools/shots.sh contrast` (black windows, white text, a
light purple highlight).

As built: `quadra_design::token` names the roles of the palette (`token::CANVAS`, `ACCENT`, ..),
each a `Token` numbered in the order of the table, and `token::NAMES` their names in a file.
`theme_from_json(text)` reads a theme file into a `KitTheme` (name, mode, a colour a role),
starting from `dark()` or `light()` by the file's mode; the two built-in themes are files
bundled in the crate (`themes/one-dark-pro.json`, `themes/one-light.json`). Roles a file
leaves unset follow its own colours (the table of behaviour/15 says which follows which).
`KitTheme::theme()` is what a tree styles with. The sections of code, the terminal and
charts are checked to hold colours and are not kept yet: their roles come with their crates.

An app uses the kit with `quadra_design::install(app, &theme)`, which loads the bundled faces
(`fonts::UI`, `fonts::MONO`; eight files, 1.6 MB of the binary) and gives the app the theme,
and with `quadra_design::root()` as the box at the root of each window: the canvas, and the UI
text (face, 14 on 23, the theme's text colour) for everything in the window. The numbers of
the design language (type scale, control heights, radii, weights) are constants in
`quadra_design::size`. `install` shapes the UI face without its contextual alternates
(`calt` off).

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

- **Actions and keys.** `quadra_design::action::Confirm` is what Enter and Space mean on a
  control (the key context `action::CONTROL`); `quadra_design::keymap()` is the kit's bindings
  over those of every window, and `install` gives them to the app.
- **`button(label)`**: `.variant(Primary | Secondary | Outline | Ghost | Danger | Link)`,
  `.size(Xs | Sm | Md | Lg)`, `.disabled(..)`, `.toggled(..)`, `.full()`, `.test_id(..)`,
  `.label(words)` (its name to a reader when it is not the visible words: "Reset Copy" for one
  of several "Reset"), `.on_click(|| ..)`. A click and `Confirm` do the same thing. Its colours are roles: the
  fill, the label, the fill under the pointer and while pressed, the border (behaviour/15,
  "States"); a tinted button's hover is a role of its own (`accent_tint_hover`,
  `error_tint_hover`: the tint 8 % towards its colour unless a theme says). The label is a
  text that takes the button's text colour. To a reader it is a button named by its label,
  pressed or not when it is a toggle.
- **Smaller things.** A button shows a spinner before its label while `.loading(..)` says
  so, looking as it did, and its click does nothing then. A checkbox shows a dash while
  `.indeterminate(..)` says some of what it stands for is checked. An input with
  `.clearable()` has a cross at its end while it holds text, and a click on it empties it.
  A tooltip takes `.meta(words)`, a smaller muted line under its text. `key_hint(keys,
  does)` is the keys in a small tinted box, then what they do, in small mono italics.
  `confirm(open, title, message, yes, on_yes)` is a dialog with Cancel and one answer.
  `menu_button(label, || items)` is a ghost button that opens a menu under itself, and
  `menubar([..])` a bar of 32 px of them.
- **Icons.** `icon(icons::CHECK)` shows one of the kit's Lucide outlines (`quadra_design::icons`,
  87 of them; ISC, `LICENSE-lucide.txt`), 14 px unless `.size(..)` says, in the colour of the
  text around it unless `.color(..)` says. A body is read once (`quadra-vector`); its mask is
  rasterized once for each size in device pixels and kept as a tile of the coverage atlas
  beside the glyphs (`Fonts::mask`, `TileKey::Mask`); showing it is one instance. The widget
  is `Drawing` in the tree, and `drawing(svg)` the view for any drawing. A drawing with
  colours of its own (art, a logo) is shown in them: rasterized once a size into the colour
  atlas (`Svg::render_rgba`, `Fonts::image`), the shapes without a colour in the text's. A
  button takes
  `.icon(body)` before its label (14 px, 16 in the two larger sizes), and
  `icon_button(body, name)` is a ghost of 22 px with the icon alone, named for a reader.
- **`input(value)`**: the field of `quadra-view` in a frame (the field's fill, a hairline
  border, 8 by 6 px around a line of text, at least 32 high, 240 wide unless told):
  `.placeholder(..)`, `.password()`, `.icon(body)` before the text, `.invalid(..)`,
  `.disabled(..)`, `.width(..)`, `.on_submit(..)`, `.test_id(..)`. The border is where the
  focus shows; a value that is not accepted has the colour of errors with the focus or
  without (the state `INVALID`, applied after every other); a disabled field is a step back
  and takes neither the focus nor text. A reader hears `.label(words)` as its name, or the
  placeholder without one.
- **Names for a reader.** Each control of the kit can be named to a reader by `.label(words)`,
  a constant or words that change (a signal, a closure), drawn nowhere: `input`, `textarea`,
  `color_input`, `number_input` (both looks), `combobox`, `key_input`, `slider`,
  `range_slider`, `radio_group` and `segmented` (the group), `button` (beside its visible
  label). The label of a `checkbox` or a `switch` is its visible words, and its name. A control that is named by what it shows (`select` by its option,
  `date_picker` by its date, `time_input` by its time) keeps those words as its value when
  a label names it. A read-only `input` has no field: its frame is a group the label names.
- **`color_input(value)`** is an input, 120 px, for a colour written `#rrggbb` (or `#rrggbbaa`,
  `Color::parse_hex`) in `value`, a `String`: a swatch of 14 px before the text, filled with the
  colour typed, or while the field is empty with `.fallback(paint)` (the colour that stands when
  none is set: a role of the theme), else left empty in its hairline border. Text that is no
  colour is not accepted (`INVALID`) and the swatch shows the fallback. `.placeholder(..)`
  (its name to a reader without a label), `.label(..)`, `.width(..)`.
- **`color_picker(value)`** (`color/picker.rs`) chooses a colour in a popover; `value` is a
  `String` as in `color_input`: `#rrggbb`, or empty for "the default" (the theme's colour of a
  setting, the tag of a host's group). The opener is a field of 120 px (`.width(..)`) the
  look of the date picker's: a 14 px swatch of the colour and its text, or while empty the
  swatch of `.fallback(paint)` and that colour's `#rrggbb` in `text_3`, as the window's
  theme (the app's `Signal<Rc<Theme>>`) gives it; with no fallback or no theme, the
  placeholder (`.placeholder(..)`, "Default" unless told). A button to a reader, named by `.label(..)` with the colour as its
  value, or by the colour (the placeholder while empty); `.test_id(..)`. A click opens the
  popover, its focus in the area; opening and closing write nothing. In it, 208 px wide:
  - the area of saturation (left to right) and brightness (top to bottom) at the hue, a
    knob where the colour is; a press or a drag sets both, the arrows step them by 1 %
    (a slider to a reader, "Saturation and brightness", its value the two in per cent);
  - the hue, a slider over 0 to 360 on a bar of the hues (`quadra_headless::slider`: keys,
    drag and a reader's value as the kit's slider);
  - `color_input(value)` for the hex, with the same fallback: typed text that is a colour
    moves the area and the hue, empty is the default;
  - the swatches of `.presets([(name, Color)])` in a row that wraps, each a button named by
    its name; with a fallback they are led by a "Default" swatch of it that empties the
    value. A swatch chosen writes its colour and closes the popover.
  What is written is `#rrggbb` in lower case. The picker keeps the hue, saturation and
  brightness it moves while it is open, so a drag through grey or black keeps its hue; it
  takes them again from `value` only when that changes to a colour it did not write. A
  preset is a colour, not a role: a caller whose presets are roles of the theme (1Term's
  host tags) gives `KitTheme::color(role)` for each. 1Term's settings colours (empty: the
  theme's) and its hosts' colour tags are its two callers.
- **`file_picker(value, browse)` and `folder_picker(value, browse)`** (`path_picker.rs`) are a
  path in an input (240 px, `.width(..)`; `.fill()` the room left in its row) and a secondary
  button "Browse…"
  after it, 8 apart. The path can be typed; the button runs `browse(current)`, which asks
  the system's dialog and answers the path chosen or `None` (given up: the value stays).
  The kit does not ask the OS itself: `quadra-shell`'s dialogs are the app's, and a kit
  that depends on it would carry the shell's own dependencies. An app passes
  `move |_| dialogs.open_files(&FileDialog::new("Identity file")).into_iter().next()`
  (`pick_folder` for a folder); tests pass `FakeShell`'s, the gallery a closure that
  answers a sample path, so no test or picture opens a real dialog. They differ by the
  button's icon (`file`, `folder`) and its name to a reader ("Browse for a file", "for a
  folder"). `.placeholder(..)`, `.label(..)` (the input's), `.width(..)`, `.test_id(..)` (the
  input's).
- **`checkbox(checked)` and `switch(on)`** take the signal that holds their value and write
  it: by a click on the control or its label, by `Confirm`, or when a reader toggles them.
  `.label(..)`, `.disabled(..)`, `.test_id(..)`. The checkbox is a box of 16 px with a
  hairline border and an accent check; the switch a track of 32 by 20 inside a ring one
  pixel off it, with a knob of 12 px that is at the end, over the accent, when on.
- **`radio_group(options, chosen)` and `segmented(options, chosen)`** hold one of a few and
  write its index: a click on an option, or the arrows while the group has the focus
  (`MoveNext`, `MovePrevious` in the key context `action::CHOICE`), round the ends. The
  group is one stop of Tab. Radios are rings of 12 px with a dot; segmented cells belong to
  one outlined control, the chosen one tinted with an accent label, the end cells rounded as
  its inside (`.radii(Corners)` on a box: a box does not cut what it holds to its corners) (`.size(..)`: 22 px with
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
  `heading(words, level)` (23, 18, 16 px, then the size of text; regular weight; lines of
  1.618 of the size), `label`
  (12 px, muted), `badge(words, tone)` and `tag(words)` (chips of 16 and 18 px in 10 px mono
  text; a badge is a faint tint of its tone), `kbd("ctrl-s")` and `key_label` (shortcuts are
  written "Ctrl-Shift-P", the modifiers in the order Ctrl, Win, Alt, Shift), `avatar(name,
  side)` (the initials on a colour that is the name's own; images later), `card()` with
  `card_head(title, more)`, `divider()` and `vdivider()`.
- **What says something in the flow** (`notes.rs`). `alert(tone, title, more)` is a box
  tinted by its tone with the tone's icon, a title and a smaller muted line.
  `empty_state(icon, title, hint, action)` is what an empty place shows, centred: a muted
  icon, why it is empty, a hint, something to do about it. `description_list(items)` is
  terms and their values in a grid of two columns, the terms muted and as wide as the widest.
  A value given as `copyable(text)` has a button of an icon after it that writes the text to
  the app's clipboard (the context's; the harness's in memory in tests) and is a check, named
  "Copied", for 1.5 s.
- **`accordion().item(title, open, || body)`**: sections under one another in one outline.
  A head is 28 px, a chevron then the title, and a button to a reader, expanded or not; a
  click, Enter or Space writes the signal that says whether its section is open. The body
  is built each time it opens.
- **`split(first, second, share)`**: two panes side by side, or `.stacked()`, and a seam of
  one pixel in a grip of 6 with the cursor of a resize. `share` is the first pane's part
  of the whole, a signal: dragging the seam writes it, never leaving a pane under
  `.least(px)` (80), and a double click makes the panes even. What does not fit in a pane
  is cut. `.alone(|| ..)`: while it gives `Some(0)` or `Some(1)` that pane has the whole
  and the other and the seam are not displayed, kept mounted with what they hold (a layout
  of one pane, then two, costs a layout, nothing built again).
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
  the mark and the name, `Status::of(word)` the status a word names: besides the six, the
  reasons a container is in its state as a cluster writes them: `Error`, `OOMKilled`,
  `Evicted`, any `…BackOff`, `Err…` or `…Error` are failed; `ContainerCreating`,
  `PodInitializing` and `Init:<n>/<m>` pending; `Init:` before a reason is that reason's.
- **`sidebar(chosen)`** is the places of an app (`NavItem::new(id, label).icon(..)`) in
  groups under muted titles, on the panel's plane with a strong line at its right, 240 px
  wide unless told. `chosen` holds the id of the chosen place: a click writes it, and with
  the focus in the sidebar the arrows, Home and End move it. A place is a row of 26 px,
  filled when chosen. While `.folded(|| ..)` says so it is 40 px wide and shows the icons
  alone; the places keep their names for a reader. `.head(view)` is above the places, and
  what does not fit scrolls. `.search(query)` puts a search field at its top (full width,
  8 from the top and the sides; small, 28 tall, the search icon, "Search…"; not there while
  folded) that keeps the places whose names hold the query (any case), and the titles of
  the groups that keep one; with none, `No results` in `text_3`. Enter in the field chooses
  the first place kept, and the arrows, Home and End go among the places kept alone.
  `.counts(|id| -> Option<usize>)` puts a count at the right end of each place it gives one
  (12 px mono in `text_3`, live as what it reads changes, not there while folded); a reader
  hears it after the name ("Pods, 420"), and the search reads the name alone. `.foot(view)`
  is under the places, outside their scroll, over a line, in a row (a `.fill()` field takes
  its width): a filter box of the view that
  shows, in reach while the places or the content scroll; not there while folded.
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
  calendar in a popup; a day chosen closes it. Today (`local_now`) is in the accent and
  stronger. The days take the focus (in the picker as soon as it opens): the arrows move
  the day by one and by a week, Page Up and Page Down by a month, the month that shows
  following it, and Enter takes it. The names of months and days, the first day of the week
  and how a date is written are the app's language (`language()`).
- **`time_input(value)`** edits a `TimeOfDay` in a field of 120 px: hours and minutes are
  two parts that take the focus each, sliders to a reader. The arrows step the part with
  the focus, round its ends (`.minute_step(..)` for the minutes), Home and End go to its
  ends, and digits type it: two in a row make a number of two when that is a value of the
  part.
- More of the fields: `input(..).readonly()` shows a value that can be selected and copied
  and not typed over; `select(..).placeholder(words)` says the words, muted, while `chosen`
  is the index of no option (`usize::MAX`: nothing chosen yet); `textarea(value)` is a
  builder (`.rows(..)`, `.width(..)`, `.placeholder(..)`).
- **`number_input(value)`** is an input that holds a number (`.range(..)`, `.step(..)`,
  `.decimals(..)`), 140 px with a pair of chevrons inside it at its end that step it, or
  with `.stepper()` one frame of 128 by 32: a minus, the value in the middle (the field is
  `text_input(..).fit()`: as wide as its text), a plus. The up and down arrows step
  it too (key context `action::NUMBER`). A number typed, with a point or a comma, is the
  value; what is no number changes nothing and shows as not accepted; a value written from
  elsewhere is what the field says. What the field shows of a value is never written back
  (1.5 shown with no digits is "2", and stays 1.5), and a step shows its own digits
  (`.step(0.1)` shows one).
- **`key_input(value)`** records a keystroke into `value` (`quadra_headless::keys`): the
  keys as `key_label` writes them, "Press keys…" while it records, "Not bound" while
  `value` is empty. `.label(..)` names it to a reader, `.width(..)` (160 px). Escape gives up;
  Backspace or Delete alone unbinds; no binding of the app runs while it records.
- **`field(label, control)`** puts a control under its label, 12 px, with `.required()` a
  mark after it; under the control is `.help(words)` or, while it is not empty, what
  `.error(..)` says, in the colour of errors after a mark. `validate(value, &rules)` gives
  what the first broken `Rule` says (`Required`, `MinLen`, `MaxLen`, `Email`): plain data.
  `textarea(value)` is the text area of `quadra-view` in the frame of an input, at least
  three lines tall unless told.
- **What is under way** (`progress.rs`). `progress(part)` is a bar of 6 px on the chrome
  plane, filled with the accent as far as `part` (0 to 1, a constant or reactive), and a
  progress bar with its percentage to a reader. `progress_busy()` is the bar for what has
  no end in sight: a third of it crosses the track every 1.4 s. `spinner()` is an open ring
  that turns once in 0.9 s, in the colour of the text around it, a progress bar named
  `Loading` unless `.label(..)` names it. `skeleton(width, height)`
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
  numbers them, in a signal the caller may own (`.selected(..)`).
- **What a cell shows** (`table.rs`): what a cell says stays what it is, what sorts, what
  is copied and what live data compares; a column may show it otherwise.
  `.view(|row, says| view)` builds a cell's view once for each row that is mounted: `row`
  is the row as the data numbers it and `says` what the cell says, both memos, so a row
  reused by the scroll or a cell that changes binds to them and is not built again (a
  status with a note after it, a bar of a share, the marks of a workload's pods, buttons).
  `.mono()` puts a column in the mono face and `.muted()` in the quiet text, at the left,
  for names and images, and for notes. A button in a cell takes its press: the row is not
  selected by it (a pressed control ends the click's dispatch, as anywhere); something else
  that acts on a click there calls `e.stop()`.
- **Live data** (`table/live.rs`): `.keys(|row| ..)` says what each row is known by. When
  the data changes, the selection, the row the keys are on and the one a range starts from
  are put where their keys are now (a row that has gone is not selected any more), under
  the order of the sort, which is worked out again. A cell that says something else than
  it did under the same key is tinted with the accent at 30 %, which fades in 800 ms; a
  row that is new is not tinted, and neither is an age, which changes all the time. The
  tints are boxes under what the cells say, one a changed cell that shows, each a layer:
  while one fades, a frame writes its opacity and nothing is laid out or painted; when all
  have faded the frames stop and the boxes are gone. A change of the data reads the key
  and the cells of every row, so this is for tables of thousands of rows, not millions;
  unless the source says which rows it changed (`.changes(rows)`, a signal it sets after
  each change): then the cells of those rows are read, and of every row only when rows
  come or go.
- A table whose columns are wider than it scrolls sideways (`scroll_x`), the header with
  the rows.
- `.pinned(n)`: the first `n` columns stay where they are; the others are in a box that
  cuts them, after a line, moved left as the wheel turns sideways (or with Shift) or the
  scrollbar along the bottom is dragged (`sideways_bar`), as far as the last column's end at
  the table's edge, the header with the rows. The box's overflow cuts only its layout: what
  moves in it is a layer cut to it (`Div::cut_to_parent`), so nothing is painted past the
  table or over the pinned columns. Moving them lays out the rows that show and nothing
  else. The diff and the log view cut the words of a line the same way.
- `.hidden(columns)`: a signal of the columns, by their places, that do not show (their
  parts are not displayed) and are not copied.
- `.cells(column)`: a cursor on cells. The left and right arrows move the column of the
  cell the keys are on (a signal), past hidden columns and stopping at the ends; a click on
  a cell puts it there; the cell is outlined in place of the row while the table has the
  focus, is selected to a reader, and Ctrl-C copies what it says.
- The header has the menu of the columns with the other button: a row a column, checked
  while it shows; choosing one hides or shows it (`hidden`), and the last one that shows
  cannot be hidden. Cells that change are tinted by live data (above).
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
  collapsed. A label may have a colour of its own (`.color(..)`), and a thing a detail
  after its label, in a colour (`.detail(text, colour)`): of what holds things it shows
  only while that is closed, and says in short what is not seen. `.row_height(px)` and
  `.indent(px)` are for a denser tree; the key for copying is bound in a tree, for what is
  around it to handle. A thing may hold what is not given yet (`TreeNode::lazy()`): it shows
  as closed, and opening it tells the tree's `.on_open(|id| ..)`, which gives its children
  through the roots. With `.many(selected)` several are selected: a click selects one, Ctrl
  and a click adds or takes away one, Shift and a click selects those from the one the keys
  are on to it, in the order shown; the keys select the one they move to alone, and the
  focus ring follows the keys, not what is selected.
- **`settings(current)`** is the preferences of an app in pages (behaviour/15, "Settings"
  and "Settings, cards", has the numbers): a column of the pages at the left, the chosen one
  at the right. A page is `SettingsPage::new(title).icon(..).about(..).group(..)`, a group
  `SettingsGroup::new(title).about(..).row(title, about, control)`: a row is what a setting
  is called, a line about it, and its control at the right, any view of the app's
  (`.wide(control)` is a row that is its control alone). A reader hears the row's title as
  the name of its control: the first switch, checkbox, input or slider in it that has no
  name of its own takes it (`Tree::meaning` reads what a node says). The app owns the controls and the
  index of the chosen page, `current`; the component keeps which pages are open in the
  column, the chosen group and where the page is scrolled. Every page is mounted once, the
  ones that do not show are not displayed: a control keeps its state while another page
  shows. Two layouts: rules (groups under small mono headers, a hairline under each row),
  and `.cards()` (page icons, a header with the page's title and what `.header(view)` adds
  at its right, each group an outlined card). `settings_group(group)` is one group of the
  cards layout on its own, in any page: its title, a heading, over the card of its rows. In the rules layout the header views are a
  row above the page's title.
  - `.search(query)` puts a search field above the column, writing `query`. While it holds
    text only the rows show whose title or line about them holds it, whatever the case and
    the accents; a group and a page that keep no row leave the page and the column, and
    while the chosen page keeps none the first page that keeps one shows (`current` is not
    written).
  - A click on a page's row chooses it and scrolls its page to the top. In the rules layout
    every page has a chevron that shows its groups under it without choosing it; in the
    cards layout a page has one when more than one of its titled groups keeps a row, and a
    click on the row also opens them. A click on a group chooses its page and scrolls the
    group's header to the top of the page.
  - `.hint(keys, does)` is a key hint under the column (rules).
  - A row can go back to its default: `.resets(value, default)` after `.row(..)`, for a
    `Signal<T>` of a `T: PartialEq + Clone`, or `.resets_with(changed, reset)` for anything
    else (a row of two controls). While the row differs from its default an icon button shows
    left of its control (`icons::UNDO`, ghost, small; to a reader `Reset <title>`),
    and a click puts the default back; at its default the button is not displayed and the row
    looks as it does without it. While any row of the page that shows (of those a search
    keeps) differs, a button "Reset all" shows at the page's header (cards: at the right of
    its title, before the header views; rules: at the right of the page's title), and it
    resets those rows, the others of the page are left as they are. A row with no reset is
    never counted: data that is no setting (a password, a list of hosts) is not given one. The
    rows of `settings_group(group)` alone reset too; it has no "Reset all".
  - The rows of the column take the focus (Tab), and Enter and Space do what a click does
    (key context `action::CONTROL`). To a reader: a tree of items, selected, expanded or
    collapsed (it can expand and collapse them); the page's title is a heading and each row
    a group named by its title.
- **`diff(old, new)`** shows what differs between two texts, line by line. The texts are
  properties: when one changes the lines are compared again.
  - What differs is the shortest script of lines removed and added (`diff_lines`, a plain
    function over two lists of lines, which an app can use for its own counts): what the
    texts start and end with alike is set aside, and the rest is searched by distance, in
    linear space: from both ends at once until the searches meet in the middle of a shortest
    script, then the part before and the part after alike (lines are numbers by then, equal
    for equal lines). A part more than 8 192 lines apart is not searched: all of the one is
    removed, then all of the other added. In a run of changes the lines removed come before
    those added.
  - `.mode(..)` is a property, `DiffMode::Inline` unless told. **Inline** is one column:
    every line of the script on a row of its own, with its number in the old text and in the
    new, a `-` or a `+`, and the row tinted (`token::DIFF_REMOVE`, `token::DIFF_ADD`).
    **Side by side** (`DiffMode::SideBySide`) is the old text at the left and the new at the
    right, each with its numbers, in halves of the same width: a line of both is on one row
    in both halves, the lines of a run of changes are paired in order, and a line with no
    partner has an empty half beside it. Both halves are one list: they scroll together.
  - **A line that was edited** shows where: a removed line and the added one it is paired
    with (in order, within their run) have what is between their common start and end
    tinted more (the colour of its mark at 30 %), when at least half of the shorter is
    common, spaces not counted. Otherwise they are two lines and nothing is marked.
  - `.context(n)` folds the lines alike that are more than `n` lines from a change into one
    row that says how many they are; a click on it, or Enter, shows them. Without it every
    line shows. A fold of one line is not made. Other texts close the folds.
  - Rows are those of a `uniform_list`, in the mono face, 18 px each: a text of a hundred
    thousand lines costs what the rows that show cost, after the comparison. A long line is
    cut at the edge of its half and the wheel turned sideways, or with Shift, moves what
    every line says (the numbers stay); a tab is four spaces.
  - Texts of 20 000 lines or more, both together, are compared on the pool (13 ms for
    100 000 lines of which 700 changed): until the script comes back the last one shows,
    and a newer text cancels it.
  - Rows are selected as a table's are (`quadra_headless::table::Sheet` of one column): a click,
    Shift and Ctrl, the arrows; Ctrl-C copies them as a unified diff writes them (` `, `-`,
    `+` before each line; a row side by side is its two lines). F7 and Shift-F7
    (`NextChange`, `PreviousChange`) select the first row of the next run of changes, or
    of the one before, and scroll to it.
  - To a reader: a table named "Diff", a row for each row, and each line a cell named by
    what it is and says ("Removed: ..", "Added: ..", or the line); a fold is a button.
  - Colours: `.colours(f)` gives `f` each whole text and takes the colours of each of its
    lines (`LineColours`, parts and roles), read in order so a comment goes on into the
    next line; the part that changed keeps its tint under them. `quadra_code::line_colours(path)`
    is such an `f`, by the tokenizer of the path's language. Without it, lines are one colour.
  - In a run of changes a removed line is paired with the added line most like it (the
    same line edited: enough of it is as it was, `changed_parts`), as many such pairs as
    there can be in order; what is left sits beside each other in order. Past 64 lines a
    side they are paired by order alone.
  - `.wrap()`: long lines wrap at the width of their column, a row is as high as its
    lines (the rows of a `virtual_list`, each measured), the halves of a row side by side as
    high as the higher, the numbers and the mark on the first line; nothing scrolls
    sideways.
  - What changed in a pair is found word by word (a word is a run of letters, digits and
    `_`, a run of spaces, or one other character), in as many places of the line as there
    are: runs of words removed and added, those with only spaces between them made one, each
    cut to what is not common to its start and end (`wéb` to `wöb` tints the one letter).
- **The keyboard's focus on a control without a border of its own** is a line of one pixel
  around it: `outline`, a paint value that takes no room (it is drawn over what the node
  paints, along its edge).
- **A widget that paints with roles of its own** (the spans of a rich text) is given the
  theme when its node is added and when the tree gets another (`Widget::theme`): it keeps
  the colours, and is painted again when they changed.
- **Colours of fields are text values** (`placeholder_color`, `selection_color`,
  `caret_color` on boxes, roles if wanted): the kit's root sets them for the window, an input
  its own selection. A field shows its placeholder while it holds nothing, composing counted.
- A component that builds a view from properties it was given passes them on as they are:
  `Prop<T>` is itself something a setter takes.
- The gallery (`examples/gallery`, the crate `quadra-gallery`) shows the components in a
  real window in either theme: a window with its own title bar, the pages in a sidebar by
  group with a search at its top, the page that shows, a status bar and a command palette. A page is the group it is
  in, its title, what the component is for, and framed demos, each with how it is written
  (`common.rs`). The pages began as those of the V gallery; each is compared with pictures
  the gallery drew before (12, "Snapshots").

## The workbench

What the window of a tool is made of around its content (behaviour/16) is `quadra_design::work`:
an app that is not a workbench does not name it.

- `activity_bar(active, [Activity::new(id, icon, title).badge(|| count), ..]).footer([..])`
  is the leftmost column, 40 px wide with its line: a tool is 40 px high, a ghost icon of
  28 px, in the accent while `active` holds its id (a click writes it), its title in a
  tooltip at its right. A count stands in the corner of the icon while it is not zero, "99+"
  over ninety-nine. The footer's tools are kept at the bottom.
- `panel_header(title)` is the title strip of a panel: 28 px on the panel's plane over a
  line, the title muted; what is added to it stands at its end (icon buttons of 18 px).
- `tab_strip(tabs, active)` shows the open views of a pane: 32 px on the plane of tabs. The
  tabs are a signal of `Tab`s (`Tab::new(id, title).icon(..).dirty(..)`) and `active` holds
  the id of the one that shows. A tab has a dot while it is dirty, its icon, its title, and,
  when the strip was told how to close (`.on_close(|id| ..)`), a cross that shows under the
  pointer; a click with the middle button closes too. The active tab stands on the canvas
  with no line under it, so it joins what it shows; the seams between tabs are on the side
  away from it. A press chooses a tab; dragged 4 px, it takes the place of the tab the
  pointer is over (the strip writes the new order to `tabs`). The strip scrolls sideways
  under the wheel (06, "Scrolling"), and a list at its end names every tab, the active one
  checked. `.action(view)` puts something before that list.
- A **pane tree** is plain data, `Pane`: a leaf that names a group of tabs
  (`Pane::leaf("editor")`), or a split of two trees side by side or one over the other with
  the share of the first (`Pane::row(a, b, 0.3)`, `Pane::column(a, b, 0.65)`). An app keeps
  it in a signal and saves it with its settings. `split(group, axis, new, before)` puts a
  new group beside one, in half of its room; `remove(group)` takes a leaf out, and the other
  side of its split takes the place of the split (the last leaf stays); `set_ratio(path,
  share)` gives the first side of a split a share, a twentieth for each side at least (a
  path is the sides taken from the root, 0 or 1); `toggle(group)` folds a leaf to its
  header or opens it again, in a split of one over the other only, and never both sides
  (the second stays open).
- `panes(root, |group| view).header(28.0)` shows the tree. It is not mirrored by nested
  boxes: every leaf and every seam is a child of one box, placed where the tree puts it in
  the box's size, on whole pixels (`Pane::place`). A change of the tree moves them: a leaf
  is built once and kept, with its nodes and their state, while its group is in the tree,
  whatever is split, taken out or folded around it. A seam is a strip of 6 px with a line
  of 1 px in its middle: a press holds it and the share follows the pointer (never under
  80 px for a side, where the split has the room), a double click makes the sides even. A
  folded leaf keeps the height of its header, under or over a line of 1 px that is not
  dragged.
- **Docking.** `dock(root, [("left", tabs), ..])` is a `Dock`: the pane tree of a workbench
  (`.root`, a signal) and the tabs of the group at each of its leaves (`.group(name)`: the
  tabs and the active id, two signals). `tab_strip(tabs, active).dock(work, group)` makes a
  strip that of a group: a tab is torn off it, dragged 12 px up or down (09, "Drag and
  drop"), and follows the pointer as a chip; along the strip it is moved by the strip as
  anywhere, and it is chosen by a click, not by the press. A strip takes the tabs dropped
  on it, before the first tab whose middle is past the pointer. `panes(..).dock(work)`
  makes every pane take them too: on the middle of a pane a tab joins its group, on an edge
  (the outer quarter of each side, `zone_at`) it has a new pane split off there, named
  "group-N". The part a drop would take, the pane or the half at that edge, is veiled while
  a tab is over the pane. A group left with no tab goes, and its pane with it
  (`Dock::apply` does what a `DockEvent` asks; an app with its own drop targets takes the
  dragged `TabDrag`).
- A tab's title is 200 px at most, cut with an ellipsis past that. `.menu(|id| items)` gives
  each tab its own menu, opened by the other button on it. `.on_rename(|id, title| ..)`
  lets tabs be renamed: a double click on a title puts it in a field with the focus; Enter
  or leaving the field gives the new title, Escape gives it up.

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
the popup closes the focus goes back to what had it. Every popup is on one plane,
`popup::surface(level)` (behaviour/15, "Elevation"): the `overlay_bg` fill, a 1 px `line`
border, `radius_sm` (square along an edge of the window) and the shadow of its level.

- `popover(open, &opener, || content)` is the bare popup, below its opener.
- `menu(open, &opener, || items)`: rows of `MenuItem` (`.icon(..)`, `.keys("Ctrl-C")`,
  `.checked(..)`, `.disabled(..)`, `.on_select(..)`, `MenuItem::separator()`), asked for each
  time the menu opens. A click on a row does what it does and closes the menu; the arrows
  move a highlight over the rows that can be chosen, round the ends, and Enter or Space
  chooses (key context `action::MENU`). Rows are 23 px, 4 px in from the edge, with 6 px
  beside and between their parts; the shortcut stands at the right end, 16 px from the label
  at least. An icon column is there only when a row has an icon or a check; icons are in the
  second text colour, and a row that is on shows a check in the accent in place of its icon. `.danger()` puts the label of a row that destroys something in the colour of
  errors.
- `select(options, chosen)`: a filled button that shows the chosen option, and a menu of the
  options, as wide as the button at least, with a check in the second text colour before the
  chosen one (no room is kept for it while none is chosen). `.searchable(ask)`, for many
  options: the popup holds a field that narrows them as it is typed into (the palette's
  fuzzy match, the best first), the arrows from the chosen one, Enter or a click takes one;
  each opening starts with no query.
- `menu_button(label, || items)` takes `.icon(body)`, `.variant(..)`, `.size(..)` and
  `.test_id(..)` for its face. `popover_button(label, || content)` is the same button (in
  the default variant) over a popover, with the content 12 px in from its edge.
- More of the menu. `MenuItem::heading(name)` names the group of rows under it: 20 px,
  muted, never chosen. `.submenu(items)` makes a row one that opens a menu of those rows
  beside itself, against the menu's right edge (its left where only that fits) and never
  over the menu, its first row level with this one, with a chevron at its end: under the
  pointer, by a click or Enter, or by the right arrow (`MoveInto`); the pointer over
  another row closes it, as does the left arrow in it (`MoveOut`), and a choice in it ends
  the menus together. A menu taller than 450 px shows that much and scrolls; the highlight
  the arrows move is brought into view.
- `context_menu(view, || items)` gives a view a menu that opens under the pointer, by the
  gap of a popup (4 px), on a click with the other button, where it fits in the window. It
  adds no node around the view (as the tooltip): the popup is opened by hand, from the
  handler (`popup::open_now`), and goes with the scope of the view if it is still open then.
  The items are asked for as it opens; none, and nothing opens (an app's setting turns the
  menu off so, without mounting the view again).
- The width a popup is said to have is that of its plane, border and all: what it holds
  asks for 2 px less (`popup::EDGE`). A menu is 200 px at least so.
- The region a popup is shown in is not displayed (`.display(Display::None)`): where the
  popup stands in the flow it takes no room, not even a gap of the box around it.
- The text values of the window's root hold in overlays too (a popup's text has the
  window's face and colour): an overlay has no parent, and is above the root.
- `tooltip("Save the file", view).keys("Ctrl-S")`: the words show above the view (below it
  where there is no room), centred and 6 px off it, once the pointer has rested on it for
  half a second, and go when it leaves or presses. The words may change (a closure or a
  signal): the bubble and the description a reader is told follow. A tooltip never has the focus. It adds
  no node: the tooltip's node is the view's own, the handlers are put on it, and the bubble
  is an overlay opened by hand (`popup::hang`: an overlay against an anchor while a signal
  is true, in a scope of its own under that of the view).
- `hover_card(view, || content)`: more about a name or a link, on the plane of a popover
  (12 px in, 8 px between its parts), under the view at its start and 6 px off it, once the
  pointer has rested on the view for 400 ms. It stays while the pointer is on the view or on
  the card, so what the card holds can be pressed, and for 150 ms after it left both: the
  time to cross the gap. It never has the focus, and adds no node, as the tooltip.
- `drawer(open, Edge::Right, "Pod api", || body)`: a dialog that lies along an edge of the
  window (`Edge::Left`, `Right`, `Bottom`) over all of its length, with square corners:
  the title and the cross of a dialog over the body. `.size(px)` is how far it reaches into
  the window (360 at first). Escape, the cross and a press outside give it up. A popup
  that lies along an edge has the length of the window on that axis (`Length::Fill` on the
  root of the overlay, which is laid out in the window's size).
- `.beside()` puts the drawer in the layout where it stands, next to the content (an
  inspector beside a list), not over the window: nothing outside closes it and the focus is
  not kept in it, so what is beside it stays in use (a press on the next row shows that
  row's object in it). It keeps the rest: it slides in, it is a dialog named by its title to
  a reader, the focus goes into it when it opens, Escape gives it up while the focus is in
  it, and when it closes the focus goes back to what had it, unless it went elsewhere. It
  is on the window's plane (`panel`), its line on the content's side, `.size(px)` wide
  (tall at the bottom). `.head(|| view)` takes the place of the title, before the cross:
  a row of the app's (a kind, a name, a state, actions).
- `dialog(open, "Delete the file?", || body).action("Cancel", Variant::Ghost, f)..`: a popup
  440 px wide with its border, centred, 80 px under the top of the window (`Place::Top`),
  with no backdrop:
  a title of 16 px with a quiet cross of 22 px at its end, the body in the second text
  colour, and, when it has actions, a seam and the buttons at the right of its foot. A
  button runs what it was given and closes the dialog; the cross, Escape and a press outside
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
  stay until closed unless they say how long they last. `.on_open(f)` makes the card's words
  a button named by its title: a click does `f` and closes the card. `.on_close(f)` runs once
  when the card goes, however it goes (its close button, its time, a button, `close`), but
  not when `.replace(id, notice)` puts another in its place, in the same place in the stack;
  `.shows(id)` says whether it still shows. `.icon(svg)` puts another of the kit's icons in
  the place of the tone's, `.picture(file)` a picture from a file's bytes (a PNG, JPEG or GIF,
  16 px, decoded at that size, named `Icon`; nothing shows when it is none). `.toast(tone, words)` is a line
  on the panel's plane with its icon at the bottom centre for ten seconds. The icon is 16 px
  on a card and 14 px in a toast, in the colour of the tone; a notice that only informs has
  the accent on a card and the second text colour in a toast. The stacks are overlays that take
  neither the focus nor a press outside them (`Place::BottomRight`, `Place::Bottom`).
- **`command_palette(open, || commands)`** is a popup of 608 px with its border under the top
  of the window at the level of dialogs: a query row of 36 px over a line, and under it the commands that
  match, rows of 32 px, at most 347 px of them, the rest scrolled to. A `Command` has a
  title, the keys that run it too (shown muted at the end of its row, as shortcuts are
  written), and what it does. A command matches when the letters of the query are in its
  title in order, whatever their case and accents (`fuzzy_hits`; "hai ph" finds "Hải
  Phòng"); letters at the start and one after another count for more (`fuzzy`), and the
  best match is first and highlighted. The letters that matched are in the accent
  (`marked`: one paragraph with a span a run, so a space at the end of a run keeps its
  width and the line reads as one text). The arrows go round the matches with the focus still in the
  query, Enter or a click runs one after the palette has closed and given the focus back.
- A drawer slides in from its edge over 200 ms (a timeline on its page: past the edge, then
  its place; composed only). A toast, or a card that lasts, waits while the pointer is on it
  and lasts as long again when it leaves (`Queue::hold`). A tooltip's words are the
  description of its view for a reader, whether it shows or not.
- The menu key and Shift-F10 open a context menu under what has the focus in its view.
- A picker of other things (files, branches, symbols) is the palette of commands whose
  doing is to take each: `.placeholder(..)` and `.empty(..)` give it its own words, and
  `Command::detail(..)` muted words after a title that the query does not search.

## Crates

```
quadra-design     buttons, inputs, selection controls, lists, tabs, panels, overlays, feedback,
               navigation, titlebar, layout helpers, themes, icons
quadra-data    log view, value viewer with its JSON and YAML readers      (behaviour/19; the
               table, with its live data, and the tree are the kit's)
quadra-viz     charts                                                      (behaviour/19)
quadra-term    terminal: parser, screen, scrollback, surface widget, Pty   (behaviour/17; on hold, 00)
quadra-code    code editor: buffer, syntax tokens, surface widget          (behaviour/18)
quadra-md      Markdown: parser, renderer, math, diagrams                  (behaviour/21)
```

- `quadra-design` holds what almost every app uses. The others are separate so an app pays for
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

## Charts (`quadra-viz`)

What the charts are and look like is behaviour/19, "Charts". As built:

- **The plot** is the one widget: lines over a box, each a list of values spread over the
  box's width, the largest value the plot is told at the top. It paints a line a device
  pixel column at a time, on the pixel grid: one line primitive in column mode a column
  (07), which carries how tall the line is there and how far it falls across the column, so
  every slope has an even width and exact edges; and, for a line with an area, one quad a
  column under it. Where the line is in a column stays a fraction of a pixel, at every
  scale of the display: a line must never show steps (the reference's did, when its
  columns were logical pixels that covered all or nothing). Nothing is rasterized and
  nothing enters an atlas: values that change
  every second overwrite the plot's instances in place, as many as before. The geometry
  (`line_spans`, `nice_max`, `value_at`, `short`, `hh_mm`) is plain functions, tested alone.
  A number beside a chart has three digits that say something, or fewer: 166, 12.3, 0.25,
  1.5k.
- Colours are roles: a theme's `series` are the eight roles `token::SERIES`, the accent
  where a theme lists fewer, and a plot is given the theme as any widget that paints with
  roles is.
- **`sparkline(values)`** is a plot of one line 1.5 px thick over an area at 12 %, 120 by 24
  px unless told (`.size(w, h)`), in the first series colour unless told (`.color(..)`), its
  top 1.1 times its largest value unless told (`.max(v)`). `values` is a constant or
  something reactive.
- **`line_chart([Series::new(name, values), ..])`** is built from views around plots, one a
  series: the values of each are a constant or something reactive, and a series that
  changes paints its own plot. All share the top, the largest value of any rounded up to 1,
  2, 2.5 or 5 times a power of ten: four grid lines with their values at the left, in mono
  10 px. `.time(start_ms, step_ms)` puts the times of the first, the middle and the last
  sample under the plot; `start_ms` is a constant or something reactive, so the times of a
  live chart that drops its oldest sample follow its samples, under the plot and in the box
  under the pointer. `.unit(..)` follows a value where one is said; `.height(px)`, 200
  unless told. One series has an area under it; several have none.
- `.limit(name, value)` draws a line at `value` across the plot, dashed, 1 px in `text_2`
  (a container's limit, a node's allocatable): the top of the scale reaches it however far
  under it the series are; the box under the pointer and a reader say it after the series
  (`limit 2000m`). `value` is a constant or something reactive.
- The pointer over the plot shows a hairline at the nearest sample and a box with the time
  and each series' value there, a swatch of its colour before it. The box is to the right
  of the hairline in the left half of the plot and to its left in the right half. It is
  state of the chart (a signal the pointer writes), and views bound to it: a pointer that
  moves and stays nearest to the same sample costs no frame (the handler has the size of
  its box with the event, 09).
- **`bar_chart(bars)`** is views alone: a row a bar (`Bar::new(label, value)`), its label
  in mono 12 px, a track with the bar in it as wide as its share of the rounded maximum (2
  px at least), and its value. `bars` is a constant or something reactive; rows are kept by
  their label.
- To a reader a sparkline is an image, named by `.label(..)`, and so is the plot of a line
  chart, named by its series; the values a chart says in text (its scale, its times, its
  bars) are texts.
- `.legend()` on a line chart puts each series' name beside a swatch of its colour in a row
  under it, which wraps. `series_color(i)` is a paint: the theme's eight, then the eight
  at 60 %, then at 35 %.
- `.upright(height)` on a bar chart stands its bars side by side, each in a column of its
  own, as tall as its share, its value over the column and its label under it: a chart of
  categories.
- A column of a plot covers the line at its two edges and every sample between them: of
  more samples than the plot has pixel columns, a peak between two edges is drawn.
- To a reader the plot of a line chart is an image named by what its lines say: each
  series, its least and its most, and its last value (`cpu: 90 to 190m, last 140m`).

## Data (`quadra-data`)

What these are and look like is behaviour/19. As built:

- **A value** (`Value`) is what JSON and YAML say: nothing, a truth, a number as it was
  written, a text, a list, or a map whose keys keep their order. `get(key)`, `at(path)`
  and `summary()` (a scalar as it is written, `{n}` and `[n]` for what holds n things) read
  it. A path is `.spec.containers[0].image`; a key that is not a plain word is written
  `["app.kubernetes.io/name"]`, so no two things of a value have the same path.
- **`parse_json(text)`** reads JSON whole: an error says its line. **`parse_yaml(text)`**
  reads the YAML that manifests are written in: mappings and sequences by indentation
  (a sequence may sit at the indentation of its key), `- ` items that hold maps, flow
  `{}` and `[]` on one line, plain, single- and double-quoted scalars, `|` and `>` block
  scalars, comments, and the first document of a file. Anchors and tags are skipped;
  aliases are not resolved. Neither reader goes deeper than 128 levels: what does is an
  error, not a crash.
- **`value_view(value)`** shows a value as the kit's tree (rows that open and close, the
  arrows, one selected): a row is a key in the colour of properties in code, or the place
  in a list in the faint text, then the value in the colour of its kind in code (text,
  number, constant); what holds things says how many while it is closed. Rows are in the
  mono face, 24 px, a level 16 px in; the first two levels start open, and a value that
  changes keeps open what is still there. Over the rows a bar says the path of the
  selected row, and `Copy path`, or the key for copying, puts it on the clipboard.
- **A log** (`Log`) is what an app appends lines to, each with the time it was written, in
  milliseconds since 1970, or none: `log.append(lines)`, `log.push(at_ms, text)`,
  `log.clear()`. The lines are packed in one buffer of bytes with an offset and a time
  each (a line of 100 bytes costs about 112), up to 4 GB of text; a million lines are held.
- A line may colour itself with the escape sequences terminals know (SGR): the sixteen
  colours, which are the theme's (`token::ANSI`, a theme file's `terminal.ansi`), the 256
  and the 16 million, bold and faint; other sequences are dropped. Only the lines that
  show are decoded.
- **`log_view(&log)`** shows the lines in the mono face, 12 px, a row of 18 px each, the
  time of each before it (`12:04:05.123`, UTC; `.timestamps(false)` leaves them out): a
  `uniform_list`, so only the rows that show exist. A line is not wrapped.
- It **follows**: the newest line stays in view while lines come. Scrolling up stops
  that; scrolling to the end again, or `Follow` in the bar, takes it up again. Following
  is a place past the end of the list, which the list keeps at its end: lines that come
  cost no scrolling.
- The bar has a **filter**: a text, found whatever its case, or with `.*` a pattern
  (`regex-lite`, whatever the case too). Lines that do not match are not listed, and the
  bar says `n of m`; with `Dim` they are all listed, the others faint. A filter is matched
  against what a line says, without its escape sequences. A new filter reads every line
  once (a byte search for a text); lines that come are tested as they come. A pattern
  that is not one filters nothing, and the bar says so.
- Lines are selected as a table's rows are (a click, Shift and Ctrl, the arrows) and Ctrl-C
  copies them, each its time and what it says without escape sequences; other lines
  listed select none. A long line is cut at the edge, and the wheel turned sideways, or
  with Shift, moves what every line says (the times stay), as far as the longest line yet.
  What the diff and the log share is `quadra_design::rows`: `one_column`, `selectable`,
  `sideways`. To a reader the log is a table named "Log".
- `Log::new().limit(n)` holds at most `n` lines: past that the oldest go, an eighth of the
  limit at a time (the buffer is moved once for all of them).

## Markdown (`quadra-md`)

What it reads and how it looks is behaviour/21. As built:

- **The parser is plain functions over text**, with no dependency on the framework but the
  URL detector of `quadra-text`. `parse(source)` gives the blocks, each with the source line
  it starts on: `Kind::Paragraph`, `Heading { level }`, `Item { number, depth, task }` (a
  bullet has no number; the depth is half its indent, a tab counting four), `Code { lang }`
  (the first word after the fence, in lower case), `Quote { depth }`, `Rule`,
  `Table { align, rows }`, `Math` and `Front`. A kind holds what it has and nothing else.
  `parse_inline(text)` gives the spans of a block's text: bold, italic, struck, code, math,
  a link, an image.
- `outline(blocks)` lists the headings with their anchors (`anchor(text)`: lower case,
  spaces to `-`, punctuation dropped); `plain(text)` is a text without its marks,
  `block_at(blocks, line)` the block a source line is in, `words(source)` the count of
  words.
- **A document that is not ours costs what its length costs**: a search that failed is not
  made again from further on (a closing backtick, bracket, comment end), brackets are
  matched once a text, and links inside the labels of links stop at 16 deep, beyond which a
  label is plain text. No input makes the parser fail, hang or recurse without bound; the
  seeded test (12, "Fuzzing") holds it to that with the corpus of the V tree.
- **`markdown(source)`** shows a document; the source is a property. The blocks are the
  children of one column (a keyed list): a block is known by what it says, and which of
  those that say the same it is, so when the source changes the blocks whose source did
  not change keep their nodes, wherever they now are, and the block being typed in is the
  one built again. Blocks are 16 px apart, the items of one list 4; that gap is the only
  thing of a block that changes under it.
  - The look is behaviour/15: body 16 / 1.5; headings 28, 22.4, 19.2, 16, 14 and 13.6 px,
    semibold, the first two over a line; items with a bullet, their number or a box (a task:
    read, not clicked), 22 px in for each depth; a quote behind a bar; a rule; code in a box
    on the tone of the editor, in the code face at 14 px, which scrolls sideways (06),
    coloured by the tokenizer its language names ("Code" below; the front matter as YAML); a
    table as a grid with lines, the head on the tone of the chrome, every other row on the
    panel's. Code in a line is in the code face, in the colour of strings; what is struck
    is faint.
  - **The columns of a table** share its width by what each asks for (its longest cell on
    one line, measured when the table is mounted), and none is narrower than its longest
    word up to 160 px: a longer word (an address) is broken inside its cell. Code in a
    cell is measured in the code face it is drawn in. The table is as
    wide as the page, so prose in a cell wraps; it is wider, and scrolls sideways, only when
    those least widths do not fit beside each other.
  - **Links** (`[text](url)`, `<url>`, bare URLs, and URLs in code, which keep the colours
    of the code they are in) are links while Ctrl is held (05): the document is not the app's own
    words. `.on_link(|target| ..)` is given the target; whether it is opened is the app's
    to say (`quadra_text::url_openable`).
  - Depths past 12 show as 12 and a table has 64 columns at most: a document that is not
    ours does not decide how wide the page is.
  - To a reader: headings by their text, the boxes of tasks checked or not, a table and
    its cells, and the texts.
- **Math** (`quadra_md::tex`; the subset is behaviour/21, "Math"). `tex::parse(source)` gives
  the nodes of a formula and the first thing that is wrong with it. A fault does not stop the
  parser: what is around it is still there, and a command it does not know is its own name.
  - A `Node` is an atom of a class (what it is to its neighbours, which decides the space
    between them: ordinary, operator, binary, relation, opening, closing, punctuation,
    inner), words, a group, a fraction, a root, a base with what is raised and lowered after
    it, a big operator or a function with where its limits go, a list between delimiters
    that grow, a delimiter of a fixed size, an accent, a line over or under, a space, the
    rows of an environment, a stack, a change of style. A kind holds what it has and nothing
    else.
  - A letter of an alphabet (slanted, as variables are; `\mathbb`, `\mathcal`, `\mathbf` ..)
    is the character Unicode has for it, so a formula is laid out with one font and no
    styles of it. The names of symbols, operators, functions, accents, spaces and delimiters
    are tables, carried from the V tree by a script and not by hand.
  - A formula comes from a document that is not ours: it nests no deeper than
    `tex::DEEPEST` (256; a brace is two), beyond which it is at fault, for the parser and for
    what walks its nodes after it; the text of an argument is read once. The seeded test
    (12, "Fuzzing") holds the parser to both.
  - **The layout** is that of TeX in boxes (`tex::Lay::new(text, size).list(nodes, style)`
    gives a `Laid`: a width, a height above the baseline and a depth below it). The styles
    are display, text, script and script of a script, at 1, 1, 0.7 and 0.5 of the size.
    Atoms are spaced by their classes (a binary operator with nothing to work on is
    ordinary); a fraction has its bar on the axis, which is the middle of a minus; scripts
    are raised and lowered by the size of their base, and are limits above and below a big
    operator in display; a root is three strokes and a rule; delimiters grow as their glyph
    set larger up to 1.6 times, and as strokes beyond; the rows of an environment are about
    the axis, their columns as wide as their widest cell. A glyph is as high and as deep as
    its ink (08, "Glyph tiles"), and the family is Cambria Math, or the default without it.
  - A box holds its parts as a tree (glyphs, rules, strokes, boxes): putting a box in another
    moves nothing, so a formula costs its length however deep it nests. `Laid::each` gives
    every part with its place.
  - **`formula(source)`** is the view (`.display()` sets it as a line of its own). Its widget
    lays out at the size of the text around it, once a size, and paints in its colour:
    glyphs with their baselines on whole pixels, rules on whole pixels and a pixel thick at
    least, a stroke as a stroke of the renderer (07). To a reader a formula is its source. One with a fault shows its source in the colour of what is wrong.
  - In a document a display formula (`$$ .. $$`, a fence of `math`) is a line of its own, in
    the middle, and scrolls sideways when it is wider than the page; one with a fault is its
    source in a box, with what is wrong under it.
  - **In a line** (`$..$`) a formula is a box of the paragraph (05, rich text):
    `formula(source).span()`, on the baseline of its line, at the size of the text, in text
    style (limits at the side, fractions small). One with a fault is its source in the code
    face, in the colour of what is wrong.
- **Diagrams** (`quadra_md::mermaid`; what is drawn is behaviour/21, "Diagrams").
  `diagram(source)` reads the source once and is a node as large as the diagram. Its kind is
  the first word after blank lines, `%%` comments and front matter. Flowcharts and sequence
  diagrams are drawn; another kind, and a source with a fault, which is told with its
  line, is `fault()`, and the view is then a node of no size.
  - **A flowchart** is read from left to right, a statement at a time: nodes in nine
    shapes, links with a stroke (solid, dotted, thick), a head at either end, a label and a
    length, chains, groups with `&`, subgraphs in each other. A label is decoded in one
    pass: what a name gives is not read again.
  - **What a diagram may hold** is checked while it is read, before anything is laid out:
    1000 nodes, 500 links (the pairs of two groups are counted before they are made, and a
    node said twice in a group is one), 100 subgraphs, 64 ranks for one link, and 5000
    nodes for long links to be cut into. A statement is searched once for each thing that
    can close a label, however many links it has. The layout walks with stacks of its own;
    only the sort of a rank by subgraphs calls itself, as deep as subgraphs are in each
    other. A seeded test (12, "Fuzzing") holds reading and layout to no failure, sizes that
    are numbers, and a time.
  - **The layout** is in layers (`flow/rank.rs`, `order.rs`, `place.rs`). The links that
    close a cycle are turned round; the rank of a node is the longest way to it, a link
    two ranks a step so that the rank in its middle holds its label; a link over more
    ranks is cut into a node a rank; a rank is ordered by sweeps to the mean of what its
    nodes are linked to and by swaps that take crossings away, the nodes of a subgraph
    kept side by side; nodes go to the median of what they are linked to, a rank spread
    out again each time. A link is a smooth curve along its nodes, cut where it meets the
    outline of what it joins and short of its heads.
  - **A sequence diagram** (`seq.rs`) is read a line at a time: participants and actors
    with the names they go by, messages (solid or dashed, with an arrow, a cross, an open
    head or none), notes left of, right of or over participants, frames with their parts
    (`loop`, `alt` and `else`, `opt`, `par` and `and`, `critical` and `option`, `break`)
    and `autonumber`; what activates, boxes and titles are read and not drawn, and a frame
    never closed ends with the diagram. It holds 200 participants and 5000 messages, notes
    and frames at most. Its layout (`seq/layout.rs`) puts the participants in columns as
    far apart as the widest thing between them, what happens one under the other, and a
    frame about what it holds, wider by the frames in it; a participant is at both ends of
    its lifeline, a note hides the lifelines under it.
  - **A drawing** (`draw::Drawing`) is what a layout gives: boxes, lines, filled polygons,
    lines of shaped text and heads, in the space of the diagram, their colours roles of
    the theme. It is made once a font and painted by walking it, and another theme costs
    no layout. Boxes are quads; lines and outlines are strokes of the renderer (07), so a
    link of any length is smooth and takes no atlas; a filled shape and the head of an
    arrow are masks in the glyph atlas, known by their points, a shape larger than 512
    device pixels a mask for each piece of that size; text is glyphs with their
    baselines on whole pixels. One painting makes 20 000 dashes at most: past them a
    dashed line is drawn whole.
  - To a reader a diagram is a picture, named by its kind and what its nodes or its
    participants say, and described by its links or its messages in order, a sentence each
    ("Start to Choice.", "Choice to OK: Yes.", "User to App: Save.").
  - In a document a fence of `mermaid` is the diagram in the middle, its text that of the
    kit's controls, and scrolls sideways when it is wider than the page; one that is not
    drawn is its source in a box, with why under it.
- **Images** (behaviour/21): a paragraph that holds only images shows them, one under
  another, each as large as its file says and no wider than the page, named by its words
  for a reader. `markdown(source).base(folder)` says where paths that do not start at a
  root are looked for. A file that is not there or is no image, and an image that is
  elsewhere (nothing is fetched), show a small mark, the words and the path. An image among
  the words of a line is a box of that line (`inline_image`, a box of rich text that paints
  pieces of the colour atlas as a picture does), its bottom on the baseline, as large as its
  file says and no side over 320 px, the line made as high; until its header is read it is
  its words in brackets, and so is one that is elsewhere or cannot be read. To a reader it
  is its words.
- A link says where it goes once the pointer has rested on it half a second, Ctrl held or
  not: its target in the bubble of a tooltip under it (`quadra_design::tip`), gone when the
  pointer leaves it.
- A table wider than the page, its columns at their least (their longest words), scrolls
  sideways; one that fits is as wide as the page.
- Where each block is: `.places(signal)` writes, after a layout that moved the blocks, each
  block's source line, top and height from the top of the document (`Place`): for an
  outline, and for two panes that scroll together (the other pane takes the line of the
  place at its top). `.reveal(|| line)` brings the block a line is in into view, as little
  as shows it. `.on_heading(|line| ..)`: a press on a heading says its source line (an
  editor beside the document goes there).
- `.scrolls(scroll_y)`: the document scrolls itself, its blocks the rows of a `virtual_list`
  (each as high as it is, counted at 48 px until it is mounted): only the blocks that show
  are mounted, so ten thousand sections mount a few dozen; its places are those of the
  blocks mounted.
- One selection over the blocks: a press in a document and a drag select from the text
  under the press to the text under the pointer (the nearest, between them), the texts of
  the blocks between them whole, in the order of the document; each paints what it has
  of it in the selection colour (`RichText::select`). Ctrl-C copies it, a text a line, a
  box said by what it says; Ctrl-A selects all of it; losing the focus lets it go. Ctrl and
  a press is for links. What is drawn and is no text (a diagram, a display formula) is not
  in it; in a document that scrolls itself, what is mounted is.
- Find: `.find(Find { query, current, count })`. While `query` holds a `quadra_code::Query`
  (its case, whole word and pattern switches as in the editor), the document marks where
  it matches in each of its texts, those of code and of tables too, in the order of the
  document, in the editor's find colours (`RichText::mark`), the `current`th (from 0, kept
  under the count) in the stronger; it writes how many to `count`, and brings the current
  one into view when the query or `current` change. It finds again after a layout that
  moved its blocks: an edit, a page of another width. A match is within one text; what is
  drawn is not searched; in a document that scrolls itself, what is mounted is.

## Code (`quadra-code`)

What the editor does is behaviour/18. As built, its buffer and its tokenizers:

- **The buffer** (`Buffer`) is the text of a document: pieces of a few kilobytes in a row,
  and before each the bytes and the line breaks of the pieces before it. Offsets are bytes
  between characters. `insert(at, text)` and `delete(range)` move the bytes of one piece
  (a piece that grows past 4 KB is cut into pieces of half of that, after a line break
  where there is one; what a deletion leaves joins its neighbour when it is small) and
  count again what is before the pieces after it. `line_start`, `line_end`, `line_of`,
  `slice` and `line` find their piece by halving and look through that piece alone; a
  line is lent, not copied, when it is in one piece, which it mostly is. A text has a
  line more than it has line breaks. A file with CR LF is held with LF and written back
  as it was (`crlf`, `file_text`).
  - It is not the piece table of the V tree, whose line starts were one array that every
    change shifted from its line to the end: the bytes and line breaks of the pieces are
    summed in a tree of sums (a Fenwick tree), so a change inside a piece costs the log of
    the count of pieces, and finding the piece of an offset or a line costs as much. A
    piece that is cut in two or joins the next counts the tree again (linear, once in a
    few kilobytes typed or deleted). A piece is plain text.
  - Measured (release): ten megabytes in 210 000 lines open in 6 ms, two hundred in
    4.3 million lines in 120 ms; a letter typed or deleted in 0.8 µs and 1.2 µs; a line
    read in 0.7 µs and 0.9 µs. The budgets are 200 ms and 2 ms (behaviour/18).
  - A test changes a text at random beside a plain string and compares all of both: the
    text, every line, where each starts and ends; another holds the tree of sums to
    counting.

- **The document** (`Doc`) is a buffer with its cursors, its undo and the commands of
  editing; it draws nothing and knows no key, so a test drives it as the editor does.
  - **A cursor** is an anchor and a head, offsets in the buffer, and the column it goes
    back to while it moves up and down. There is one at least; they are kept in the order
    of the text, and those that overlap, or that touch when one selects nothing, are one.
  - **An edit** is a text in place of a range. A command makes its edits for every cursor
    and `apply` makes them as one change, the last first; the edits are in the order of
    the text and not over each other (two cursors in one word delete it once). An edit
    with a caret puts the cursor that asked for it; where none has one, every cursor is
    taken through the edits (an edit a line, not an edit a cursor).
  - **Undo** takes back a group and puts the cursors where they were before it. Typing
    joins the typing before it until a line break, a cursor that moves, another command,
    or a pause of a second (`now`, set by who drives the document). A thousand groups are
    kept.
  - **Motion** is by grapheme in a line, by word (letters, blanks, the rest; what is not
    ASCII is of words), to the first thing of a line that is not blank and then to its
    start, by line and by page in the column the cursor came from. Columns are cells
    (`quadra_text::display_width`: a wide character is two, a mark none) and a tab goes to
    the next stop.
  - **Commands**: text, a new line with the indent of the old and a level more after what
    opens a block, deleting by grapheme and by word, the tab, lines indented, deleted,
    duplicated, moved and made comments, everything and lines selected, the word at an
    offset, a cursor more above or below (a selection in a line is a column), the next
    place the selected text is (`Buffer::find` reads on from piece to piece and stops at
    the first), the bracket that matches the one at the cursor (20 000 bytes away at
    most, strings and comments not skipped), and what copy, cut and paste do (a line to
    each cursor when there are as many).
  - `version` counts the changes, for what a view keeps of the text; `dirty` is the first
    line changed, for who colours it.
  - A test does commands at random with several cursors: after each, the cursors are in
    the text, between characters, in order, none over another; in the end everything is
    undone to the text it started as and done again to what it was. It found three things
    the V tree has too: two cursors in one word asked for edits over each other, a
    selection moved down past the last line was left past the end, and the word selected
    at a cursor could hold another cursor.
- **A tokenizer reads a line at a time.** `Tokenizer::tokenize(line, state, out)` adds the
  tokens of the line to `out` (a `Token` is the bytes `start..end` and a `TokenKind`; what
  no token covers is plain text) and gives the state the next line starts in; the first
  line of a text starts in 0. The state is all a line needs to know of the lines before
  it: what is open at their end (a comment and how deep, where comments nest; a string and
  its quote; a fence). So a change reads again its line, and the next ones only while their
  states differ, and a view reads the lines it shows.
- The kinds are those a theme gives colours to (its `syntax`): keyword, control, string,
  number, constant, type, function, variable, property, operator, punctuation, comment,
  tag, attribute, regexp, escape, invalid. `role(kind)` is the role of a kind
  (`token::SYNTAX_*`).
- **`tokenizer_for(name)`** finds one by the name of a file or by what follows a fence,
  whatever its case: by all of the name (`Dockerfile`), then by its first word
  (`rust title=x`, `js {.cls}`), then by what is after its last dot. A name none knows is
  `Tokenizer::Plain`: no tokens.
- **Three have code of their own**: V; YAML (keys, flows, block scalars: the state is how
  deep the key of the scalar is); Markdown (fences, math, the marks in a line).
- **The others are rows of one table** (`LangSpec`), read by one scanner: the words of the
  language (keywords, control, types, constants, builtins), its comments, its quotes and
  what they hold (escapes, expressions, raw strings with any count of hashes, closed by a
  quote and as many, three quotes, strings over lines), and
  switches for what families of languages differ in (`$name`, `@name`, directives after a
  `#`, attributes, lifetimes, `name!`, keys before a `:` or an `=`, sections, selectors).
  The rows: C, C++, C#, Java, Kotlin, Swift, Go, Rust, JavaScript, TypeScript, Dart,
  Python, Ruby, PHP, Lua, the shells, the shell of Windows and its batch files, SQL,
  makefiles, container files, JSON, TOML, INI, CSS and its kin; markup (HTML, XML) and
  diffs are modes of the scanner. A language more is a row more.
- **Code that is not ours**: whatever a line holds, and whatever state it is given, the
  tokens are in order, none over another, each starting and ending between characters; and
  a line costs what its length costs (Markdown does not search twice for a mark it did not
  find). The seeded test (12, "Fuzzing") holds every tokenizer to both.
- A tokenizer allocates nothing: tokens go into the caller's vector, and words are compared
  where they are.
- **The highlighter** (`Highlighter`) keeps the state each line ends in for the lines
  before `valid`; `sync(doc)` takes the document's `dirty` and lowers `valid` to it, and
  `tokens(buffer, line, out)` reads the stale lines before `line` and then the line. A view
  asks for the lines it shows, so a letter typed deep in a file costs those lines; the
  lines below them stay stale until they show. A test edits at random and holds every
  line's tokens to those of reading the text from its start.
  - The editor asks for colours through the trait `Highlight` (`sync(doc)`, `tokens(buffer,
    line, out)`), which the highlighter is one of: one that parses the whole text into a
    tree (a parser of grammars, kept up by the changes) can be another, mapping what it
    finds to the kinds, given with `editor(..).highlighter(h)`. Such a parser is no
    dependency now; it would be one of specs/01, R15, in a crate of its own.
- **Finding** (`Query`: the text, `case`, `word`, `regex`) gives every place it matches
  (`matches(doc)`, or what is wrong with a pattern). A query that is no pattern is one all
  the same, each letter a class of its two cases, so a letter of Vietnamese matches its
  other case and the offsets are the text's; a whole word has no letter, digit or `_`
  beside it. The pattern is `regex-lite`'s (specs/01, R15). `next_match` goes round the
  ends; `Doc::replace` and `replace_all` (one undo) put a text in place of matches. The
  matches are found again for each version of the document while finding shows (as the V
  tree did), on a copy of the text (ponytail: piece by piece if files of tens of megabytes
  make it slow); a replacement is literal.
- **The editor** (`editor(text).path(name)`) is a scroll view around one widget,
  `EditorWidget`, which holds the document, its highlighter and what finding found. The
  widget is as high as its lines and as wide as the view; the scroll view scrolls it up and
  down, its wheel and its bar, and it scrolls itself sideways (the wheel with Shift, or
  sideways; the caret is kept in view), as far as the widest line it has painted. It paints
  the lines of the band its layer paints, not the document: under the text the line of each
  cursor (with the focus), the selections, the matches and the bracket that matches; the
  text, a glyph at the column of its character (`Doc::col_of`: a tab to its stop, a wide
  character two cells), each in the colour of its token's kind; the cursors, a bar each,
  always painted and clear while the blink hides them; and the gutter, the numbers of the
  lines at the right of at least four digits, two cells before them and three after,
  brighter on the lines of the cursors. The face is the text values at its node (the kit's
  mono face at 15 px, lines of 25).
  - The colours are roles of the theme, a theme file's `editor`: `token::EDITOR_CURSOR`,
    `EDITOR_LINE`, `EDITOR_GUTTER`, `EDITOR_GUTTER_ACTIVE`, `EDITOR_SELECTION`,
    `EDITOR_FIND`, `EDITOR_FIND_CURRENT`, `EDITOR_BRACKET`; the text the kinds' `syntax`,
    and what no token covers a variable's in code (as the reference painted it) and
    `EDITOR_FOREGROUND` in prose (plain text, Markdown). The marks of the gutter are `OK`,
    `WARN` and `ERROR`.
  - **Keys** are bindings of the context `editor` to `EditorCommand`, an action
    (`bind_keys(&mut keymap)` adds them; an app adds them to its keymap): those of
    behaviour/18, "Keys". Tab indents there, being of the nearest context. Escape closes the
    find bar, or with none open gives one cursor. Typed text, a composition (shown at the cursor, underlined, not yet in the
    document), the pointer (a click puts the cursor, Shift extends it, Ctrl adds one, a drag
    selects, two clicks a word, three a line, Alt and a drag a column) and the clipboard of
    the context are those of a text input. The time of a command is the window's clock, so
    typing after a pause is an undo of its own.
  - After a command the caret is scrolled into view, and the widget is laid out again only
    when the count of lines changed.
  - Measured (docs/benchmarks.md, the bench's `editor` and `editor-scroll`): a letter typed
    at the middle of 100,000 lines takes a frame of 0.39 ms (the budget is 2), scrolling 10
    µs a frame.
  - What it says to a reader: a text input of many lines, its text and its selection.
  - **The find bar** stands at the top right of the editor while it is open (Ctrl-F; Ctrl-H
    with the row that replaces; a selection of one line is the query). It has the query,
    the switches of case, whole words and patterns, where the chosen match is of how many
    ("2 of 5", "No results", "Bad pattern"), the match before and the next, and closing.
    The query has the focus when it opens; Enter and F3 go to the next match, Shift-F3 and
    the arrow up to the one before, Escape closes it and gives the editor the focus.
    Replace puts the text in place of the chosen match and goes to the next; All replaces
    every match as one undo. The matches are found again as the query, a switch or the text
    changes, and painted under the text, the chosen one stronger. When the text changes
    and the query is text (not a pattern, which may match across lines), only the lines
    the changes were are searched again and the matches after them move
    (`Query::rematch`, by the bytes each `LineChange` removed and put in): on 400,000 lines
    with a match on each, 230 µs a letter where the whole text took 306 ms. A test of
    random edits, several at once and undone, holds the kept matches to those found
    again, with whole words and with case.
  - **Soft wrap** (`editor(..).wrap(true)`, for prose): a line is broken into rows at the
    text's width in cells, after the last blank that fits; a word longer than a row breaks
    inside it, and blanks hang past the edge. The rows (`editor/rows/wrap.rs`: the first row
    of each line, where each row starts) are built when the width or the tab size changes, a
    pass over the bytes; as the text changes only the lines its changes were are wrapped
    again, and the rows after them move. The document notes the lines of each change
    (`LineChange`: a line, how many lines it was, how many it is, and in bytes where it is,
    what it removed and what it put in; `Doc::line_changes_since`
    gives those after a version, the last 256 of them; a view further behind builds again;
    `Doc::changed_lines` takes what a view keeps a line each through them and says which
    lines of now are to be made again).
    On 100,000 lines a letter typed costs 24 µs where building them all costs 66 ms. A
    test of random edits, several at once and undone, holds the kept rows to rows built
    again. Rows drive every y: paint (the band of a line over all its
    rows, its number on the first), the pointer, the caret kept in view, the height. Up,
    down and the page keys go by rows in the column they came from; Home and End stay the
    line's. Nothing scrolls sideways.
  - **Links** (`.on_link(|url| ..)`): the URLs of the line under the pointer are found as
    it moves (`quadra_text::find_urls`, that line alone); with Ctrl held one is underlined
    and the pointer is a hand, and Ctrl and a press give its URL to the app and move no
    cursor. Once the pointer has rested on one (Ctrl held or not), a hint under the pointer
    says its URL and "(Ctrl-click to open)"; a press or another place hides it. Without the
    hook links are text.
  - **The marks of a diff** (`.base(text)`): each line is marked against the base as added,
    changed or after lines removed (`marks(old, new)`, from the kit's `diff_lines`), found
    again after each change. The editor keeps a hash of each line of the base and of the
    text, hashes again the lines the change was (`Doc::changed_lines`), and finds the
    script between the hashes (`diff_ids`): what both start and end with alike is a
    comparison of numbers a line. On 100,000 lines the whole text compared took 16 ms a
    letter; a letter and its frame take 1.4 ms now. A test of random typing, keys, undo
    and cursors holds the kept marks to those of the whole text; a bar beside the numbers,
    a short one above a line where some were removed. `.changed(signal)` keeps a signal true
    while any line is marked (the text is not the base) and false while none is: an app's
    "modified", at no cost past the marks. `EditorWidget::set_base(text)` puts another base,
    as after a save, and marks again. `.keep(KeptDoc)` lets the document outlive the view:
    the widget gives it to the slot when it goes, and an editor built again over the slot
    takes it back with its text, cursors and undo (an app's view built again when its tab
    shows again).
- In the find bar's query Enter goes to the next match and Shift-Enter to the one before
  (an input submits on Enter alone, and leaves Shift-Enter to what is around it).
- To a reader the editor is a text input of many lines: its text and its selection in
  characters, told again when the text changes and when the primary cursor moves. A text of
  more than 256 KB is told as the 200 lines on each side of the primary cursor's line (the
  selection in them), so a change does not send megabytes to a reader; the lines told
  follow the cursor. `tests/reader.rs`.
- `.top_line(line)`: the line at the top of the view, a signal kept as it scrolls (the
  wheel, the keys); setting it scrolls that line to the top, as far as the view goes, and
  the line it then shows is told back. For a preview that follows the editor.
- `.position(signal)`: where the view stands as a fractional line, the line at its top and
  how far down its rows (wrapped rows counted), kept as it scrolls; setting it scrolls there,
  as far as the view goes. Finer than `.top_line`, for panes that scroll together.
  `.caret_line(signal)`: the line of the primary cursor, kept as it moves (keys, the
  pointer, an edit from outside). `.caret(signal)`: the same cursor as a `Caret { line,
  column, selected }`, the column and the selection counted in characters, for a status bar.
- `.edited(signal)`: the document's version, set after each change of the text (an app's
  preview reads the text when it moves, not on every key). `.handle(EditorHandle)`: what
  reaches the editor from outside it while it is mounted (`is_mounted`). `read(f)` runs `f`
  on its widget in the tree's next pass, and `edit(f)` changes it there as a key would: then
  it is coloured, marked, laid out and painted again, the caret in view and the reader told.
  `focus()` gives it the focus back. A toolbar's button and a save go through it.
- A character whose glyph is wider or narrower than the cells it takes (one of two cells
  in a face without it) is drawn in the middle of them.
- In markup what a `<script>` holds is read as JavaScript and what a `<style>` holds as
  CSS, over lines (the state of markup carries theirs), up to the tag that ends it, in a
  string or not, as a browser does. In JavaScript and TypeScript a `/` where a value starts
  (after an operator, an opening bracket, the start of a line, or a word like `return`)
  opens a pattern with its flags; after a name, a number, a string or a closing bracket it
  divides, and one whose line ends first divides too.

## Porting the kit

Order within phase R3: tokens and themes; text, box and icon; buttons and inputs; selection
controls; overlays; lists and tabs; panels and the titlebar; the gallery as each lands.

A component is designed from what it is for: its look in the design language, its states,
its keyboard behaviour, its semantics, its tests. Where a V version of it existed, that was
history to read, never a shape to keep (the V components were builders evaluated during
layout, with handlers passed as data).

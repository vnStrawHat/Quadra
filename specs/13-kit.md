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
- A component that builds a view from properties it was given passes them on as they are:
  `Prop<T>` is itself something a setter takes.
- The example `gallery` of the kit shows the components in a real window in either theme;
  it becomes the gallery app when there are pages to have.

## Overlays in the kit

Popover, menu, context menu, tooltip, select, combobox, dialog, drawer, toast and the command
palette are built on `overlay` (05): placement against an anchor, a focus scope, dismissal on
outside press or Escape, return of focus to the opener. One implementation of each of those
four behaviours serves all of them.

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

# The Quadra design language

How anything built with Quadra's kit (`quadra-design`) looks and is laid out: the kit itself,
the gallery, the apps. It is written for two readers at once: a person designing a screen and
an agent writing one. Both should be able to build a view that looks right the first time,
without looking at a picture and nudging numbers afterwards.

This folder owns **what things look like**: every size, colour, distance and shape is stated
here once. `specs/13-kit.md` owns **what components are and do** (their API, keys, semantics)
and points here for numbers. A number that differs between the two is a bug in specs/13.

## Read in this order

| File | What it settles |
|---|---|
| [principles.md](principles.md) | The identity: Quadra's four, and the rules of taste that follow |
| [color.md](color.md) | Planes, text, the accent, feedback, hues; themes |
| [type.md](type.md) | The two faces, the four sizes, which text is which |
| [space.md](space.md) | The scale of distances, control and row heights, radii, icons, elevation, motion |
| [layout.md](layout.md) | **How to lay a view out**: within, between, around; alignment; widths |
| [controls.md](controls.md) | One look for everything you type in or press; each control's numbers |
| [surfaces.md](surfaces.md) | Menus, popovers, dialogs, cards, notifications; bars and chrome |
| [data.md](data.md) | Rows, lists, trees, tables, status, charts |
| [components.md](components.md) | The larger parts in detail: title bar, status bar, tabs, settings, the palette, the colour picker, the scrollbar, key hints |
| [patterns.md](patterns.md) | **Recipes with code**: a dialog with a form, a confirm, a settings page, an empty state |
| [checklist.md](checklist.md) | What to check before a view is done; what the gate checks for you |
| [decisions.md](decisions.md) | Why the numbers are what they are; what was found and changed; what is still owed |

An agent that builds one screen reads `layout.md`, `patterns.md` and `checklist.md`; the rest
is reference.

## The language on one page

**Quadra's four** (the identity, [principles.md](principles.md)):

1. **Hairlines, not boxes.** Regions are planes one step of tone apart with a 1 px seam. No
   shadows inside the window, no cards in cards, no gradients.
2. **Squares for marks.** A status, a tag's colour, a bullet, a tick box, the mark of what is
   chosen: small squares and straight bars. Circles are for what turns (a spinner), for one
   of many (a radio) and for people (an avatar).
3. **Mono for what the machine said.** Paths, numbers, addresses, keys, times, ids are in the
   mono face. What a person named or wrote is in the UI face.
4. **One accent, used as a mark.** The accent is a 2 px bar, a border, a label's colour: what
   is chosen, focused or on. Solid, it fills one thing a view: the primary button. Never a
   large fill, never decoration.

**The ladders** (each has four steps; needing a fifth means the design is wrong, not the
ladder):

| Ladder | Steps | Where |
|---|---|---|
| Space inside a thing | 2, 4, 6, 8 | [space.md](space.md) |
| Space between things, and around them | 12, 16, 24, 32 | [space.md](space.md) |
| Control heights | 18, 22, **28**, 32 | [space.md](space.md) |
| Row heights | 24 data, **28** items, 32 bars, 40 touch | [space.md](space.md) |
| Radii | 2 marks, 4 small, **6** controls, 10 floating | [space.md](space.md) |
| Text sizes | 12 meta, **14** body, 16 title, 18 headline | [type.md](type.md) |
| Icon sizes | 10, 12, **14**, 16 | [space.md](space.md) |
| Planes | canvas, panel, chrome, overlay | [color.md](color.md) |
| Text colours | text, text_2, text_3, disabled | [color.md](color.md) |
| Feedback | ok, warn, error, info | [color.md](color.md) |

The bold step is the default: a control is 28 tall, a row of items 28, text 14, an icon 14, a
control's corners 6, unless there is a reason this folder names.

**Layout in five lines** ([layout.md](layout.md)):

- Parts of one thing are 2 to 8 apart; things are 12 or 16 apart; sections 24; never 0.
- A container's inset is at least the gap between what it holds: 12 in a floating surface or a
  card, 16 in a large card, 24 in a page.
- Labels stand over their fields, 4 apart; fields fill their column; two to a row at most.
- Everything in a container starts on one left edge; what closes or confirms is at the right.
- A row's parts are centred on its cross axis; text shares a baseline; numbers align right.

**In code**: `vstack()`, `hstack()`, `form()`, `columns(n)`, `button_row()`, `section(title)`,
`sections()` and `spacer()` lay things out with these distances so that a view has no numbers
of its own ([patterns.md](patterns.md)); the tokens are `token::SPACE_XXS` .. `SPACE_XXXL` and
`token::RADIUS_XS` .. `RADIUS_LG`, and a theme may change them. `tools/check.sh` rejects a
distance, a radius or a text size that is not on a ladder ([checklist.md](checklist.md)).

# Decisions

Why the language is what it is: what was found when the first version (one file, written for
the kit as it grew) was reviewed against what had been built with it, what was decided, and
what is still owed. Dated 2026-11-24.

## What was found

The first version named tokens and listed each component's numbers. It was right about
colour and type and it built a kit that looks of a piece control by control. Three things went
wrong when whole views were built from it.

1. **Nothing said how to lay a view out.** There was no rule for the distance between two
   fields, a container's inset, which edge things start on or how wide a field is. Every view
   chose for itself: dialogs with hand-computed widths (a "half" of 202 and a "full" of 416
   so that two fields and a gap came to the body's width), children with no gap, labels and
   fields that did not line up. An agent could not get a first draft right, because nothing
   it could read said what right was; it drew, looked at the picture and nudged.
2. **The numbers had drifted.** 63 distances, radii and text sizes in the kit, the gallery
   and the apps were off any scale (padding of 5, 7, 10, 14; gaps of 10, 18, 20; radii of 3,
   3.5, 7, 8; text of 11, 13, 22). The same thing was stated three ways (an overlay's
   corners), two tokens had one value (`radius_md` and `radius_lg` were both 6), rows were 23,
   26 and 28 without a reason, and the fields did not share a look (a select was a filled
   button beside outlined inputs).
3. **It read as borrowed.** The palette and the density came from the editor the kit first
   followed, and nothing in the text said what was Quadra's own; so each new view borrowed
   again from wherever its author had last looked.

## What was decided

- **A folder, each number stated once.** `specs/design` owns what things look like;
  specs/13 owns what components are and do and points here. A number in both is a bug.
- **An identity in four statements** ([principles.md](principles.md)): hairlines not boxes,
  squares for marks, mono for what the machine said, one accent used as a mark. They come
  from what the user's own app designs already did well (square status marks, outlined
  fields, mono paths and addresses, an accent line for what is chosen) and are stated so that
  a new view can be judged against them. The palette stays: it was never the problem.
- **Ladders of four.** Distances inside a thing 2, 4, 6, 8; between and around 12, 16, 24, 32.
  Control heights 18, 22, 28, 32. Rows 24 (data), 28 (items), 32 (bars), 40 (touch). Radii 2,
  4, 6, 10. Text 12, 14, 16, 18. Four steps are few enough to choose between without
  thinking and enough that two levels of grouping always differ by a step. A value off a
  ladder needs a reason on its line (`// design: why`); the gate counts them.
- **The layout rules are in the kit, not only on paper.** A dialog's body, a drawer's, a
  popover's and a card's come with their inset and gap. `form()` and a dialog's body make
  their fields as wide as their column, so a view gives no widths. `vstack`, `hstack`,
  `columns`, `button_row`, `section`, `sections` carry the distances. A first draft that
  uses them cannot have children that touch, sit on an edge or miss a line.
- **One look for a family.** Every field (input, select, number, date, time, key, colour,
  path) has the field look: `input_bg`, a 1 px `line_strong` border, corners of 6, 28 tall, 8
  inside. A select is a field one presses, not a button.
- **Corners by level.** 6 for controls, cards and what floats a little (menus, popovers,
  tooltips); 10 for what floats over the whole window (dialogs, the palette, notifications).
  `radius_xl` is gone; `radius_lg` is 10.
- **A check in the gate** (`tools/check_design.py`): a literal distance, radius or text size
  in the kit, the gallery and the apps is a step of its ladder or carries its reason.
- **Proof by drawing.** The gallery has "Patterns" pages built with the helpers alone; the
  recipes of [patterns.md](patterns.md) are their code.

## What changed in the kit (2026-11-24)

| Was | Is |
|---|---|
| `radius_md` 6, `radius_lg` 6, `radius_xl` 10 | `radius_md` 6, `radius_lg` 10 |
| measures `space_xs` .. `space_xl` (4, 8, 12, 16, 24) | `space_xxs` .. `space_xxxl` (2, 4, 6, 8, 12, 16, 24, 32) |
| every popup's corners 4 | 6 at level 1, 10 at level 2 |
| a select filled as a button | a select with the field look; its menu as wide as it |
| fields with a width each (240, 160, 180, 140, 128, 120) | the same widths alone; their column's in a form |
| a dialog's body a padded box | a column 12 in, 12 between, its fields filling it; `.width(px)`; a confirm 360 |
| a dialog's foot 4 between, 8 in | 8 between, 12 at the ends |
| `popover` bare, `popover_button` padded | both 12 in, 8 between |
| list rows 23, tree and table rows 26, sidebar rows 26, menu headings 20 | data rows 24, item rows 28, menu headings 24 |
| round dots for a menu's mark, a tab's unsaved mark, a step's | squares |
| 63 values off the ladders | 0; 6 with a reason |

## Still owed

- **The apps' views still write their own distances.** 1Term's dialogs are laid out by hand
  to the app's design canvas (fixed widths of 202 and 416); they fit the 440 dialog and were
  left. They move to `form()` and `columns(2)` when next touched.
- **Tokens for literals.** The kit's own components write on-ladder numbers (`.gap(8.0)`)
  where a token (`token::SPACE_MD`) would let a theme change the density of everything at
  once. The check accepts both; a theme's `measures` reach the helpers, the containers and
  what already names a token.
- **The check reads literals in calls.** A distance computed (`if small { 3.0 } else { 2.0 }`)
  or kept in a constant is not seen.
- **The title bar is 32 in the kit and 38 in an app whose title bar holds its tabs.** Both are
  stated ([components.md](components.md)); one helper for the taller bar would end the
  hand-built ones.

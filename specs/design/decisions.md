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

## The trial (2026-11-24)

The language was tried the way it will be used: an agent that had never seen the kit was given
this folder and nothing else about looks, and asked for a dialog with a form of eight fields
and a card of facts, without drawing or looking at anything. Its first draft compiled, wrote
no gap, padding, radius, text size or colour, and when drawn had every field on one left
edge, the rows of two equal, the foot in order. It also reported 25 places where the text
was silent, said two things, or did not match the kit. What they changed:

- **In the kit**: a segmented group in a form fills its column, its cells sharing the room
  (it had an outline as wide as the column with the cells at its left); `field(..)` is a
  group a reader hears by its label, its label `text_2`, and it no longer stretches a control
  that has its own width; a dialog's body is `text` (it was `text_2` for everything in it,
  form or sentence); `number_input` and `time_input` take `.width(px)`; a path and a time are
  in the mono face; `mono(words)` for a value the machine said.
- **In the text**: a dialog is 440 with two short fields to a row and 560 only for long
  values (the recipe and the rule disagreed); "no numbers" became "no distances", with the
  width of a container as the one number a view gives; `card_head`, the lines of a card, the
  textarea and the foot with an aside are described as they are; how the facts and the status
  of a card are laid out; where a section goes in a form; what `form()` and `field(..)` return.

## What was owed, and done (2026-11-24)

- **1Term's dialogs take the kit's layout.** The host editor, Connect, Quick connect, the
  sign-in question and the vault's unlock gave up their widths (202, 416, 392) and their own
  grid of two: fields fill their columns and `columns(2)` puts two to a row.
- **The kit names its tokens.** Every gap, padding and radius inside the kit that is a step
  of a ladder is written as its token (`token::SPACE_MD`, `token::RADIUS_MD`), so a theme's
  `measures` change the density and the corners of all of it at once.
- **The check reads what a call is given**, not only a lone number: a distance chosen between
  two (`if small { 4.0 } else { 2.0 }`) and one computed are seen. Four more were found and
  put on the ladders or given their reason.
- **From the trial**: a combobox is as wide as its column in a form, its list with it;
  `number_input(..).unit("backups")` says what a number counts, muted, inside the field;
  `card_head_with(title, more, end)` puts a status, a tag or a quiet button at the end of a
  card's head; a control in a `field(..)` in a form fills its column whatever wraps it.
- **The title bar**: `titlebar(..).height(38.0)` is the taller bar (1Term uses it); nothing
  was missing but the sentence in components.md.

## Decided, not owed

- **A section in a form stands a field's distance (12) from its neighbours**, not a step
  more. Its label (12 px, semibold, `text_2`) is the mark of the group; a wider gap in the
  middle of a dialog read as a hole. It goes last, so that its end is the form's.

## Still owed

- **The check does not follow a constant.** A distance kept in a `const` and passed by name
  is not read; a constant is a decision someone named, and so far each is on a ladder.
- **The gallery and the apps still write on-ladder literals** (`.gap(8.0)`) where the kit now
  writes tokens. A theme's measures do not reach those.

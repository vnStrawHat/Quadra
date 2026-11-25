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

- **OneTerm's dialogs take the kit's layout.** The host editor, Connect, Quick connect, the
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
- **The title bar**: `titlebar(..).height(38.0)` is the taller bar (OneTerm used it until
  2026-11-24, and is a bar of 32 since); nothing
  was missing but the sentence in components.md.

## Decided, not owed

- **A section in a form stands a field's distance (12) from its neighbours**, not a step
  more. Its label (12 px, semibold, `text_2`) is the mark of the group; a wider gap in the
  middle of a dialog read as a hole. It goes last, so that its end is the form's.

## Against paleness (2026-11-24, the user's word for the look, and their decision)

The pictures read pale. The palette was not the cause; four things were, and each is changed:

- **The primary button was a tint**, a step from a plain button, so a view had nowhere for
  the eye to land. It is the accent, solid, by the theme's own roles (`accent_hover`,
  `accent_press`, `text_on_accent` for its words); off, it is the tint it was. One a view,
  as before: the accent is still a mark, and this is its one fill. Its border is the focus
  colour already, so its focus by keys is a border in the text's colour. Seven of OneTerm's
  themes had words on the accent that could not be read (under 3 to 1); they are corrected,
  and a test holds every theme to 3 to 1.
- **Titles had the weight of body text.** A page's headline, a card's title and a group's
  title are semibold (600); size alone did not make a hierarchy.
- **Too much was `text_2`.** A group's title in settings was muted at 16 px; it is `text`.
  `text_2` stays for what explains (a label over a field, a description, help).
- **The accent was near absent**: a 2 px bar and the focus border. What is chosen in a
  list, a tree, a navigation and the settings' pages, a toggle button that is on, and in
  OneTerm the active tab of the rail and the picked row of the dock, is filled with the
  accent's tint in place of the neutral `press`, which stays for the pressed and the
  keyboard's row.

## Done since (2026-11-24)

- **The check follows a constant.** A distance kept in a `const NAME: f32` of any file read
  and given by its name (or by its path) is read as the number it is.
- **The gallery and the apps write tokens** where they wrote on-ladder literals, so a
  theme's measures reach them as they reach the kit; so do the distances the kit chose
  between two (`if small { .. } else { .. }`). A number still written in a distance is a
  factor of arithmetic (`side / 2.0`), not a distance.
- **A measure carries its own number** (`Measure(index, own)`): a tree with no theme, or a
  theme with fewer measures, keeps a view's distances instead of collapsing them to 0.

## The apps, read against the language (2026-11-24)

OneTerm and Markdown took the look from the kit; what they still did their own way is changed:

- **Small dialogs** (SFTP's New folder, Rename, Delete and A large file; Rename tab; Open
  link; Properties) set a width and a padding inside the dialog's own, so their content
  stood 24 from the edge where every other dialog's stands 12. They give the body alone, in
  the dialog of 360 for a question or one field. Delete is `.danger()`.
- **Rows on the ladder**: hosts, SFTP entries, transfers and completions were 26 and 23 and
  are data rows (24); the expanded SFTP pane's header is an item row (28); OneTerm's
  horizontal strip is the bar's 32, as its spec said.
- **No size on a control**: the find bars (the editor's, OneTerm's, Markdown's) gave their
  query 26, between two steps; it is md. Heights that only repeated the md step and widths
  that only repeated a field's own are gone; the host filter fills the rail.
- **Markdown's toolbar** is the kit's `toolbar()`, not a row of 36 of its own.
- **Settings**: a select in a row is 240 in both apps (Markdown's were 200).
- **A failure in a dialog** is the kit's alert, not a paragraph in the error colour; what
  is wrong with a field (a wrong password, a try refused) is said under it by
  `field(..).error(..)`, not by a red line above; the one weight of 700 is 600.
- **Helpers where the apps wrote them out**: a column 8 apart is `vstack()`, a centred row 8
  apart `hstack()`, a body 12 apart `form()` (31 places in the apps and the gallery). The
  lock card's body is a form, so its field is the card's width by the kit.
- **A theme's own tint.** What is chosen is the accent's tint, and 37 of OneTerm's 38 themes
  gave none: they showed the kit's blue under their own accent. A file that gives no tint
  gets its accent at 16 %. A test holds every theme to that, and to words that read (3 to
  1) on the accent at rest, under the pointer and pressed.

One exception with its reason in the code: a host key's fingerprint is mono at 12 px, a step
under the text around it, so that its 50 characters stay one line to compare.

**OneTerm's own shapes, on the ladders too** (the user's decision, 2026-11-24). The canvas the
user drew for OneTerm gave it a title bar of 38, a status bar of 30, tab rows of 36 (52 for
the one that shows) and a folded rail 48 wide with 44 a tab. Asked whether they stay, the
user chose the ladders: the title bar and the status bar are bars (32), a tab of the rail is
an item row (28) and the touch step (40) while it shows its second line, the folded rail is
40 wide with 40 a tab, and New terminal is a control of 28 in a row of 40. With them go the
two exceptions the kit itself kept: its status bar was 30 and the command palette's query
row 36; both are bars. No height of a row or a bar in the kit or the apps is off the
ladders now, but a toolbar (a bar with 6 above and below and its line: 45) and a menu's
separator (a line with 4 above and below: 9), which are sums of steps.

**The status bar is lower than the title bar** (the user's decision, 2026-11-25). With both
at 32 the user found OneTerm's window out of balance: the title bar has to be the taller
of the two, and 38, which it had before, was a little too tall. Left to choose, the pairs
were drawn and compared (title / status): 38 / 32 and 38 / 28 bring back the height the
user took away; 32 / 24 leaves the bar's 22 px buttons 1 px above and below and makes it
lower than a panel's header; 32 / 28 keeps the title bar and makes the status bar an item
row, as tall as a panel's header, its buttons 3 above and below. 32 / 28 it is, for the
kit's `statusbar()` and OneTerm's own.

Nothing is owed.

# Type

Two faces and four sizes. Text is small and has room above and below it: 14 px on a 23 px
line, so a dense screen still reads as lines, not as a block.

## The faces

- **UI face: IBM Plex Sans**, bundled with the kit (regular, italic, semibold, semibold
  italic), shaped without contextual alternates (`calt` off) so labels keep plain forms.
- **Mono face: Lilex**, bundled (regular, italic, bold, bold italic), with programming
  ligatures: code, the terminal, and every value the machine said (principle 3).
- **Two weights**: 400 for everything, 600 for a title and for the one thing in a group that
  is chosen or stressed. No third weight, no italics for emphasis in the interface.

## The scale

| Step | Token | Size / line | What it is for |
|---|---|---|---|
| meta | `TEXT_SM` | 12 / 19 | a field's label, help and error lines, a section label, badges, tags, the status bar, a tooltip's second line, a table's header, timestamps |
| **body** | `TEXT_MD` | 14 / 23 | everything else: controls, rows, menus, tabs, dialog bodies, descriptions |
| title | `TEXT_LG` | 16 / 26 | the title of a dialog, a card, a panel's page, a settings group |
| headline | `TEXT_XL` | 18 / 26 | the one headline of a page |

Outside the four, for one use each: `TEXT_XS` 10 / 16 only inside a mark that has no room
(a key hint's box, a count in a corner, the word under a folded tab); `TEXT_2XL` 23 / 29 and
28 only as the headline of a document or of the gallery's pages. Code and the terminal are
mono 15 / 24. Markdown content has its own scale (body 16 / 1.5; specs/21).

The line height of UI text is 1.618 of its size, rounded to whole pixels. A box that holds
one line of text is sized by the kit's `by_text(px)`, so it grows with the user's text
size.

## Which text is which

Size and colour go together; a view picks a row of this table, not a size and a colour apart.

| Kind of text | Size | Weight | Colour | Face |
|---|---|---|---|---|
| Page headline | 18 | 600 | `text` | UI |
| Title (dialog, card, group) | 16 | 600 | `text` | UI |
| Section label (a heading over rows or fields) | 12 | 600 | `text_2` | UI |
| Body, a control's value, a row's name | 14 | 400 | `text` | UI |
| The chosen or active one of siblings | 14 | 600 | `text` | UI |
| Description under a title or a row | 14 | 400 | `text_2` | UI |
| A field's label | 12 | 400 | `text` | UI |
| Help under a field | 12 | 400 | `text_2` | UI |
| An error under a field | 12 | 400 | `error` | UI |
| Placeholder | 14 | 400 | `text_3` | UI |
| A value (path, number, address, time, id) | as its row | 400 | `text` | mono |
| A shortcut after a label | 12 | 400 | `text_2` | mono |
| A count, a separator, a unit | as its row | 400 | `text_3` | mono |
| Disabled | as it was | as it was | `text_disabled` | as it was |

Rules:

1. **One headline a page, one title a container.** A dialog has one 16 px title; what
   divides its body is a 12 px section label, not a second title.
2. **Hierarchy by colour before size.** Title and description are both in their step; the
   description is `text_2`. Do not shrink text to make it quieter, and never below 12.
3. **A title is 2 over its description** (they are one thing); a section label is 8 over what
   it heads (space.md).
4. **Text wraps or ends with an ellipsis; it never clips and never pushes a control out.** A
   name in a row is one line with `.ellipsis()`; a description wraps. Give the text
   `min_width(0)` and the room that is left, the control its own width.
5. **Numbers that are compared are mono and right-aligned** (a table's column, sizes in a file
   list), with their unit in `text_3` or in the header.
6. **Sentence case**, no full stop after a label, no colon after a field's label, an ellipsis
   (`…`, one character) when more will be asked.

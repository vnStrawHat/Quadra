# Building a view

How a screen is put together with the kit so that it looks right the first time. The short version: **use the helpers and the containers, and write no distances of your own.** The rules behind this are the [design language](../design/README.md); its [layout page](../design/layout.md) is the one to read first.

## Arrangements

A view is boxes in rows and columns. The kit names the arrangements that come up, each with its distance:

| Helper | What it is | Between its children |
|---|---|---|
| `vstack()` | a column of lines of one group | 8 |
| `hstack()` | a row of controls, centred across | 8 |
| `form()` | a column of fields; a field in it is as wide as its column | 12 |
| `columns(n)` | `n` equal columns (two fields to a row) | 12 |
| `button_row()` | a row of buttons at the right end | 8 |
| `section(title)` | a small heading over a column | 8 |
| `sections()` | a column of sections | 24 |
| `spacer()` | takes the room left in a row | |

{{code examples/gallery/examples/guide.rs#arrangements}}

Each returns a `Div`, so anything a `Div` takes can follow (`.width(..)`, `.wrap(true)`, `.child(..)`). Reach for a bare `row()` or `column()` only when building a component of your own.

## Containers come with their inset

The body of a dialog, a drawer, a popover, a hover card and a card is already a padded column. Put the content in; do not wrap it in a box with padding.

{{code examples/gallery/examples/guide.rs#dialog}}

- A dialog's body is a form: fields fill it, 12 apart. No field has a width.
- `.action(label, variant, f)` adds a button to the foot, in order; the main one is last. `.aside(label, f)` is a quiet one at the far left.
- `open` is a signal. Setting it shows the dialog; a button of the foot, the cross, Escape and a press outside close it.

More recipes, each compiled and drawn in the gallery: [patterns](../design/patterns.md).

## Fields and their labels

`field(label, control)` puts a label over a control, with `.help(..)`, `.error(..)` and `.required()`. The field is a group a screen reader announces by its label. Give the control a `.label(..)` as well when a test has to find it (`get_by_role(Role::TextInput, "Name")`), and always when it stands outside a `field`.

Every field control has the same look and height: `input` (and its password form), `textarea`, `select`, `combobox`, `number_input`, `date_picker`, `time_input`, `key_input`, `color_input`, `color_picker`, `file_picker` and `folder_picker`. Alone, each has a width of its own; in a `form()` or a dialog it is as wide as its column (a combobox keeps its own); `.width(px)` says otherwise. A value the machine said (a path, a time, a size) is `mono(..)`.

## What changes

A property that should follow a signal takes a closure. A part that comes and goes is `show`; a list is `each`, or the kit's `list`, `table` and `tree` when it is long.

{{code examples/gallery/examples/guide.rs#changes}}

## Test it

A view is tested with no window and no GPU: mount it in a `Harness`, find controls by role and name, act, and read the tree.

{{code crates/quadra-design/tests/layout.rs#test}}

Find things as a person would (`get_by_role(Role::Button, "Save")`), not by a coordinate or an id. Never touch the real clipboard in a test.

## See it

`tools/shots.sh dark` draws every gallery page to a picture without opening a window; put a view on a gallery page and look at it. Before calling a view done, go through the [checklist](../design/checklist.md); `tools/check_design.py` (in the gate) rejects a distance, a radius or a text size that is not a step of the design language's ladders.

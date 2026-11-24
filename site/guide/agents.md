# For agents

This page is for an AI agent that writes code with Quadra, and for the person who briefs one. Everything the site says exists as markdown: each page has a twin at the same path ending in `.md`, [`llms.txt`](../llms.txt) lists them with one line each, and [`llms-full.txt`](../llms-full.txt) is all of them in one file.

## Read these, in this order

1. [Concepts](concepts.md): a view is built once; signals carry change. Most wrong code comes from expecting a view function to run again.
2. [Building a view](views.md): the helpers and the containers.
3. [Layout](../design/layout.md), [patterns](../design/patterns.md) and the [checklist](../design/checklist.md) of the design language.
4. The page of each [component](../components/index.md) you use: its pictures and the code of its demos.

In the repository, `CLAUDE.md` has the rules of the project (commands, commit messages, where scratch files go) and `specs/13-kit.md` the API of every component.

## The rules that prevent a bad first draft

- **Start from the nearest pattern** in [patterns](../design/patterns.md). Each is compiled and drawn.
- **Write no distance.** `vstack()`, `hstack()`, `form()`, `columns(n)`, `button_row()`, `section(..)`, `sections()` carry the gaps; a dialog, a popover and a card carry their inset. A view with `.gap(..)` or `.padding(..)` in it is not following its pattern.
- **Give a field no width in a form.** It is as wide as its column. Two fields to a row at most (`columns(2)`).
- **A number you must write is a token**: `token::SPACE_LG`, `token::RADIUS_MD`, `size::TEXT_SM`. The ladders are 2, 4, 6, 8 inside a thing and 12, 16, 24, 32 between things; radii 2, 4, 6, 10; text 12, 14, 16, 18.
- **Every colour is a token** (`token::TEXT_2`, `token::LINE`). Never a literal colour.
- **Every control has a name**: `field(label, ..)`, `.label(..)`, or its own words. An icon button has a tooltip.
- **One primary button in a container**, last in its row. The button says what it does ("Delete host"), never "OK".
- **Text that can be long** gets `.ellipsis()` and `min_width(0.0)` or wraps; what stands beside it gets `shrink(0.0)`.

## Verify without a screen

| What | Command |
|---|---|
| It compiles, is formatted, passes lints and tests | `tools/check.sh` |
| No number is off the design language's ladders | `python tools/check_design.py --list` |
| What a gallery page looks like | `tools/shots.sh dark <page-id>` writes `.tmp/shots/dark/<page-id>.png` (and `<page-id>-open.png` when the page opens something) |
| One crate's tests | `tools/cargo.sh test -p quadra-design` |

Look at the picture once to confirm, not to find the layout: if the first picture is wrong, the view broke a rule above; fix the rule, not the pixel.

## Mistakes seen often

| Mistake | Why it is wrong | Instead |
|---|---|---|
| Reading a signal outside a closure (`text(format!("{}", n.get()))`) | read once, never updates | `text(move || format!("{}", n.get()))` |
| Rebuilding a list with `dynamic` on every change | every row is mounted again | `each(items, key, row)`, or the kit's `list` |
| `column()` with no gap and several children | children touch | `vstack()` or `form()` |
| A padded `div` inside a dialog's body | double inset | put the content in the body |
| Fields with widths chosen to add up | they stop lining up when anything changes | `form()` and `columns(2)` |
| A second 16 px heading in a dialog | two titles | `section(title)` |
| Finding a node in a test by `test_id` on a plain `div` | it is not in the semantic tree | `get_by_role(role, name)` |
| A test that reads or writes the system clipboard | it changes the user's machine | the harness's own clipboard |

## Where things are

| You need | Look in |
|---|---|
| A component's API | `specs/13-kit.md`, and its source in `crates/quadra-design/src/` |
| What a component looks like, its numbers | [the design language](../design/README.md) |
| A working example of a component | its page here, and `examples/gallery/src/pages/` |
| What is built and tested | `docs/progress.md` |
| Names to search for, by crate | `docs/tour.md` |

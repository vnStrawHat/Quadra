# Concepts

Five ideas carry the whole framework. Each is small; together they are why a frame costs what changed.

## A view is built once

A view is a function that returns a description: `div()`, `row()`, `column()`, `text(..)` and the kit's components, with builder methods for what they look like and do. Quadra mounts it into a retained tree of nodes. The function does not run again.

{{code examples/gallery/examples/guide.rs#once}}

There is no render function that returns the whole UI on every change, and so nothing to compare against the last one.

## Signals carry change

`signal(value)` makes a value that can change. Reading it inside a closure given to a property binds that property to it.

{{code examples/gallery/examples/guide.rs#signals}}

- `memo(|| ..)` is a value computed from signals, computed again only when what it read changed, and telling its readers only when its result differs.
- `effect(|| ..)` runs something outside the tree when what it read changes.
- `batch(|| ..)` writes several signals and tells their readers once.
- A signal is `Copy`: move it into as many closures as need it.

Almost every property takes either a value or a closure: `.bg(token::PANEL)` or `.bg(move || if on.get() { token::PRESS } else { token::PANEL })`.

## Structure that changes

What is in the tree changes through three views, each doing work proportional to the change:

| View | For | Cost of a change |
|---|---|---|
| `show(condition, view)` | a part that appears and disappears | mounting or removing that part |
| `dynamic(view)` | a part that is built again when what its closure reads changes | that part |
| `each(items, key, row)` | a list keyed by identity; a row keeps its node while its key stays and gets its item as a signal | the rows that came, went or moved |
| `virtual_list(..)`, the kit's `list`, `table`, `tree` | thousands of rows: only those in view exist | the rows that scrolled in |

## Layout is flex and grid, and incremental

Every `div` is a flex container (`row()`, `column()`) or a grid (`.grid(tracks)`), with the properties you expect: `gap`, `padding`, `grow`, `shrink`, `width`, `min_width`, `align_items`, `justify`, `wrap`, absolute placement, overflow and scrolling. When a node changes size, layout runs for the part of the tree whose result can differ, not for the window.

In an app you rarely write these numbers. The kit's helpers (`vstack`, `hstack`, `form`, `columns`, `button_row`, `section`) and containers (dialog, popover, card) carry the distances of the [design language](../design/README.md).

## Style comes from tokens, and a theme fills them

A colour is a token (`token::PANEL`, `token::TEXT_2`, `token::ACCENT`), a distance or a radius a measure (`token::SPACE_LG`, `token::RADIUS_MD`). A theme is a file that gives each a value; `dark()` and `light()` are built in, and `theme_from_json(text)` loads another. Changing the theme repaints what uses the tokens that differ; nothing is rebuilt.

States are part of a style, not code that swaps styles:

{{code examples/gallery/examples/guide.rs#states}}

## Everything has a name a reader hears

Each node can carry a role, a name and a state (`Role::Button`, "Save", disabled). The kit's components set them. That semantic tree is what a screen reader reads through UI Automation, and it is how tests find things: `get_by_role(Role::Button, "Save")`, not a coordinate.

## What a frame does

1. Events arrive and are dispatched along the tree; handlers write signals.
2. The properties bound to those signals update their nodes and mark what they dirtied: style, layout or paint.
3. Layout runs for the dirty part; paint runs for the nodes whose pixels changed; each layer's display list is patched in place.
4. The renderer uploads the instances that changed and draws.

With nothing dirty, no frame is drawn at all.

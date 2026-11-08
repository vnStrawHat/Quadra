# 05 — Layout

Module `quadra.layout`. Pure V, no dependency on `gpu`, `text` or `platform`. Input is a node tree with styles; output is a rect per node in logical px.

## Style

| Group | Properties |
|---|---|
| Display | `flex` (default), `grid`, `none` |
| Position | `relative`, `absolute` with `inset` |
| Box | `width` `height` `min_*` `max_*` as `Length = px \| pct \| auto \| fill`, `aspect_ratio`, `padding`, `margin` (px or auto), `border` widths |
| Flex container | `direction` (row, column, and reverses), `wrap`, `justify`, `align_items`, `align_content`, `gap` |
| Flex item | `grow`, `shrink`, `basis`, `align_self` |
| Grid container | `columns`, `rows` as track lists, `auto_rows`, `auto_cols`, `auto_flow` (row, column, dense), `justify_items`, `align_items`, `justify_content`, `align_content`, `gap` |
| Grid item | `col(start, span)`, `row(start, span)`, `area(name)`, `justify_self`, `align_self` |
| Grid areas | `areas(['header header', 'nav main'])` defines named areas |
| Overflow | `visible`, `hidden`, `scroll` (content size is reported; clipping happens in core) |

Track sizes: `px(n)`, `pct(n)`, `fr(n)`, `auto`, `min_content`, `max_content`, `minmax(a, b)`, `repeat(n, track)`, `repeat_fill(min_width)` for responsive card grids.

Not supported: subgrid, masonry, baseline alignment, `order`.

## Tree and API

```v
pub struct LayoutTree { /* struct-of-arrays node arena with a free list; ids stay stable */ }
pub fn (mut t LayoutTree) node(style Style, children []NodeId) NodeId
pub fn (mut t LayoutTree) leaf(style Style, measure MeasureFn, ctx voidptr) NodeId
pub fn (mut t LayoutTree) set_style(id NodeId, style Style)  // marks dirty only if a layout field changed
pub fn (mut t LayoutTree) set_children(id NodeId, children []NodeId)
pub fn (mut t LayoutTree) mark_dirty(id NodeId)              // e.g. text content changed
pub fn (mut t LayoutTree) remove(id NodeId)                  // recursive, returns ids to the free list
pub fn (mut t LayoutTree) compute(root NodeId, available Size)
pub fn (t &LayoutTree) rect(id NodeId) Rect                  // relative to the parent
pub fn (t &LayoutTree) content_size(id NodeId) Size

pub type MeasureFn = fn (ctx voidptr, known Size, available AvailableSize) Size
```
The tree is **persistent**. Cached views keep their nodes between frames, and only changed subtrees are rebuilt (07-state).

## Incremental layout
- **Dirty propagation**: a dirty node marks its ancestors dirty, up to the first **relayout boundary**: a node whose size does not depend on its content (definite width and height, or `fill` inside a definite parent). Layout restarts from the boundary, not the root.
- **Measure cache**: each node keeps the last two `(constraints -> size)` results, because flex and grid usually probe once for min/max-content and once for the final size. A clean node answers from the cache.
- **Position-only changes** never re-measure children: if a node's size is unchanged, its subtree keeps its layout and only the parent offset moves.

## Algorithms
- **Flex**: the CSS flexbox algorithm restricted to the properties above. Steps: base sizes, line collection, flexible-length resolution with min/max clamping until stable, cross sizes, alignment, absolute children last.
- **Grid**: CSS grid track sizing restricted to the properties above:
  1. Place items: explicit lines and areas first, then auto-placement with a cursor (sparse or dense).
  2. Size tracks per axis: fixed tracks, then intrinsic (`auto`, `min/max_content`, `minmax`) from items spanning one track, then items spanning several tracks distributing extra space, then `fr` tracks sharing the remaining space.
  3. Columns are resolved before rows so text can wrap at the final column width.
  4. Align items within their cells and the grid within its container.

## Tests and benchmarks
- `flex_test.v` and `grid_test.v` are table-driven, with expected rects checked against browser output for the same CSS.
- `bench/layout_bench.v` covers 10,000-node flex and grid trees, a full layout versus an incremental relayout after changing one leaf. Targets are in 12-performance.

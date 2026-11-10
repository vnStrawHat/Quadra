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
pub struct Tree { /* flat node arena: ids are indices, valid until clear() */ }
pub fn (mut t Tree) node(style Style, children []NodeId) NodeId
pub fn (mut t Tree) leaf(style Style, measure MeasureFn, ctx prim.Holder) NodeId
pub fn (mut t Tree) leaf_same(style Style, measure MeasureFn, ctx prim.Holder, same SameFn) NodeId
pub fn (mut t Tree) compute(root NodeId, available Size)     // forgets every result first
pub fn (mut t Tree) compute_new(root NodeId, available Size) // a tree built since clear()
pub fn (mut t Tree) recycle_below(root NodeId, owner u32)    // before rebuilding a view's subtree
pub fn (mut t Tree) replace(old NodeId, fresh NodeId) int    // incremental layout, below
pub fn (t &Tree) rect(id NodeId) Rect                        // relative to the parent
pub fn (t &Tree) content_size(id NodeId) Size
pub fn (t &Tree) verify(root NodeId) ?string                 // against a layout from scratch

pub type MeasureFn = fn (ctx prim.Holder, known_w f32, known_h f32, avail_w f32, avail_h f32) Size // ctx: prim.hold(el), read with prim.held[T] (specs/14, checked holders)
pub type SameFn = fn (old prim.Holder, fresh prim.Holder) bool      // two leaves that measure alike
```
The tree is **persistent**. A full layout (window resize or scale change, theme switch,
`Window.invalidate()`) clears and rebuilds it. A build for notified views keeps it and
replaces only their subtrees (07-state, Invalidation levels). Layouts done during prepaint
(list rows, overlays) add scratch nodes after the build's, dropped at the next prepaint.

## Incremental layout
- **Answers.** A node keeps its last four `(request -> size)` results and which one was its
  last *perform* (the request at which it placed its children). A request is what a parent
  asks: a fixed size or an available size per axis. A node's layout depends only on its own
  style and its children's answers, so a node whose style and answers did not change leaves
  its parent's layout as it was.
- **Replacing a subtree** (`Tree.replace(old, fresh)`). The fresh subtree takes over the
  layout of every part equal to the old one: same style, same children, and leaves that
  measure alike (their `SameFn`) or answer the old requests alike. Then the fresh root is
  asked every request the old root answered, the perform one last. With the same answers
  and the same style nothing above it runs and it keeps the old rect. Otherwise its parent
  forgets its own answers and is asked its old requests again, and so on upwards.
- **Relayout boundary**: the first ancestor that answers as before. No static rule is
  needed: a node with a definite width and height, or one its parent stretches to an
  unchanged size, answers the same whatever its content. Each climbed node costs its
  children, which answer from their caches. When the climbs of one build reach more
  children than the tree has live nodes (many rows of a long list changed height), the
  build switches to one full layout.
- **Measure cache**: a clean node answers from its cache; the perform entry is never the one
  evicted. A node that lost an answer its parent was given counts as changed.
- **Position-only changes** never re-measure children: a perform at the same request on a
  node with nothing changed below returns at once (the rects under it still hold; its
  parent sets its own rect). A node measured at another request since (`probed`) performs
  again, because a leaf may keep the probe's state (a text wrap).
- **Recycling.** Before a view's new tree is built, the nodes of its old tree (not those of
  its nested views: `Node.owner`) are handed to the next `node` and `leaf` calls in the order
  a build makes them. A node filled again with what it held keeps its layout, so a rebuild
  that changed no layout input compares as it builds and leaves no garbage but the old root.
  Garbage over twice the live nodes plus 4096 makes the next build a full layout.
- **Checks.** `Tree.verify(root)` lays a copy out from scratch and returns the first rect or
  content size that differs. `QUADRA_VERIFY_LAYOUT=1` runs it after every incremental
  layout; the whole test suite passes with it.

## Algorithms
- **Flex**: the CSS flexbox algorithm restricted to the properties above. Steps: base sizes, line collection, flexible-length resolution with min/max clamping until stable, cross sizes, alignment, absolute children last.
- **Grid**: CSS grid track sizing restricted to the properties above:
  1. Place items: explicit lines and areas first, then auto-placement with a cursor (sparse or dense).
  2. Size tracks per axis: fixed tracks, then intrinsic (`auto`, `min/max_content`, `minmax`) from items spanning one track, then items spanning several tracks distributing extra space, then `fr` tracks sharing the remaining space.
  3. Columns are resolved before rows so text can wrap at the final column width.
  4. Align items within their cells and the grid within its container.

## Tests and benchmarks
- `flex_test.v` and `grid_test.v` are table-driven, with expected rects checked against browser output for the same CSS.
- `bench` (main.v) covers 10,000-node flex and grid trees; `bench/statemodel` changes one row's colour among 1,000 to 10,000 live rows (one view, a view per row, an equal write). Targets are in 12-performance.
- `tests/relayout_test.v`: a paint-only change runs no layout, a taller row moves the rows below, a fixed box stops the climb, hover, scroll, virtual-list rows, resize, equal writes, a parent and a child notified together, kit components, the fall-back to a full layout.

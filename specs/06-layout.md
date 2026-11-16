# 06 — Layout

Layout is Quadra's own engine (01, R8). Its results stay on the nodes; a change lays out the
changed node and climbs only as far as the change is felt.

## Model

- Sizes are in logical pixels (`f32`). Device pixels appear only at paint (07).
- A node's rect is relative to its parent's border box. A scroll offset is not part of any
  rect: it is the transform of the scroll content's layer (07).
- The style values are those of behaviour/05 ("Style"), unchanged: flex with wrap and gaps; grid
  with tracks in pixels, percentages, fractions, `auto`, `min_content`, `max_content`,
  `minmax`, `repeat`, `repeat_fill`, placement by line, span or named area; absolute children
  with insets; lengths as pixels, percentages, `auto` or `fill`. Not supported: subgrid,
  masonry, baseline alignment, `order`.
- Leaves with content (text, image, a custom widget) report an intrinsic size through
  `Widget::measure` (04).

## The engine

`quadra-layout` depends on `quadra-prim` only. It holds the layout part of every node in its
own dense arena (style, children, answers, rect, content size), indexed by the tree's node
slot. The tree passes a measurer for leaves when it runs a layout:

```rust
pub trait Measure {
    fn measure(&mut self, node: LayoutId, known: Size<Option<f32>>, avail: Size<Avail>) -> Size;
}
```

### Requests and answers

A parent asks a child a **request**: per axis, a definite size or a limit (a length,
min-content, max-content, unbounded); and whether to **measure** (report a size) or **perform**
(also place the children). The child gives an **answer**: its size and content size.

- A node keeps its last four answers with their requests, and which one was its last perform.
  A flex container asks a child for its base size, then its cross size at the final main size,
  then performs it; nested containers repeat that, hence four. Four covers a leaf only when
  requests that differ in available space it ignores are one request ("Measuring leaves").
- A node's layout depends only on its own style and its children's answers. So a node whose
  style and whose children's answers did not change leaves its parent's layout as it was.

This is the idea the V engine was built on, and the part that carries over.

### What a change costs

A node flagged `LAYOUT` (a layout style value, its children, or a leaf's content changed)
forgets its answers. The layout pass then, for each flagged node, deepest first:

1. Asks the node again the requests it had answered, in the order they were asked and the
   perform last, and stops at the first answer that differs. An answer that held for more
   than its request ("Measuring leaves") is the same only if the new one holds for all of
   that too: the parent may have been given it for any of those requests.
2. **The answers are the same**: stop. Only the node's subtree was laid out, and the node keeps
   its rect. Nothing above it runs.
3. **An answer differs**: the parent forgets its own answers and is asked its old requests
   again. Its other children answer from their caches. Repeat from step 2 with the parent.

Stopping at the first difference matters when a size changes every frame (a panel being
dragged, a width that animates). A parent asks for a size, then asks again with the size it
was told: once the first answer has changed, the later requests are ones nobody will repeat,
and answering them would lay the subtree out at its old size. For the same reason a node is
not asked at all when a value its parent reads itself changed (its own width): the parent
runs whatever it answers.

The **layout boundary** is the first ancestor that answers as before. No static rule declares
it: a node with a definite size, a scroll container, or one its parent stretches to an
unchanged size answers the same whatever its content, so the climb ends there by itself.

Details that keep it exact:

- A node already laid out by another node's climb in this pass is skipped.
- **Position only**: a perform at the same request on a node with nothing changed below
  returns at once. The rects under it still hold; its parent sets its rect.
- A node that was measured at another request since its last perform performs again, because
  a leaf may keep the state of the last measure (a text wrap at that width).
- The perform answer is never the one evicted from the cache. A node that lost an answer its
  parent had been given counts as changed.
- A perform that no measure had asked for is forgotten when the node is performed at another
  request: the parent that placed it there has moved on, and nothing else knows the answer.
- A node with no old answers (just mounted) makes its parent the starting point.
- When the climbs of one pass reach more children than the tree has nodes (many rows of a long
  list changed height at once), the pass switches to one full layout from the root.

What does **not** carry over from the V engine: recycling nodes of a rebuilt subtree, replacing
an old subtree by a fresh one, owners, epochs, placements that become void. They existed
because every render built new nodes. Here nodes persist, and a change is a flag on a node.

### Measuring leaves

- A text node measures from its retained paragraph (08): shaping is reused, wrapping at a new
  width uses cached advances.
- A leaf whose content did not change and whose request is in its cache is never measured.
- **What an answer holds for**: with its size a leaf may say for which available widths that
  size stays its answer, and that the available height played no part. A text says both: the
  same lines come out from its widest line up to the width it was broken at (any width up,
  when no line broke for width), and it never reads the height. A request that differs from a
  cached one only there is answered from it. Without that, each auto-sized ancestor adds its
  own available sizes (a bar's height, a group's padding, the window's width) to the requests
  a leaf deep in a status bar is asked, the four answers are overwritten in one layout, and a
  clock whose text changes every second climbs to the window although its size never
  changes. Saying nothing (the default) keeps the exact request: a leaf that fills its space,
  or one whose minimum or maximum is a percentage of it, does.

## Scrolling

- A scroll container has a viewport (its own rect) and content (its children's extent). The
  content is a layer; the scroll offset is that layer's translation.
- Scrolling sets `COMPOSE` on the layer. It does not flag layout or paint, unless a virtual
  list must realize rows.
- Offsets are clamped in the compose pass and rounded to whole device pixels, so glyphs keep
  their subpixel positions while scrolling.
- `scroll_into_view`, anchoring (keep the first visible item in place when content above it
  changes size) and momentum are behaviours of the scroll widget.
- **The scrollbar** (behaviour/15, "Scrollbar"). Every scroll view and list has one, with
  nothing to ask for: a pill 6 px wide, 4 px in from the right edge and from the ends, as
  long as the viewport is of the content and at least 25 px, in the text colour that holds
  at the container, at 30 %. It is a node of its own in the viewport, beside the content
  and above it, and a layer: scrolling moves it by its offset and paints nothing. It shows
  when the pointer moves over the container or the content is scrolled by any means, stays
  one second after the last of those, then fades over 400 ms (a thumb held stays, however
  still, until a second after it is let go); content that fits has none.
  The 14 px at the edge where the thumb is are its grip: a press there takes the pointer,
  and the content follows the pointer in proportion until the release.
- **Scrolling into view** (`Tree::scroll_into_view`, `NodeRef::scroll_into_view`): every node
  around the asked one that scrolls what it holds is asked, the nearest first, to show the
  node's rect, given in its own box as it is scrolled now. It scrolls as little as shows the
  rect (to its start when it is taller than the viewport) and says how far, sideways and
  down, which moves the rect for the next one out: nested scrolling settles in one go. A
  scroll view, the lists and the strips (sideways) answer; a widget that scrolls its own
  content registers with `Tree::on_reveal`. A focus that comes by the keyboard and the
  semantic action `ScrollIntoView` ask for it; `NodeRef::scroll_into_view` asks after the
  next layout, so a node just mounted is where it will be. The kit's tab strip scrolls to
  the tab that becomes the active one (chosen from its menu, opened past the end).
- **A strip** (`scroll_strip(scroll_x, content)`) is a row that scrolls sideways: tabs. The
  wheel scrolls it, turned either way (a page never loses its wheel to it: what the strip
  cannot scroll goes on outwards), and it has no scrollbar. Its content is as wide as the
  strip at least, so what grows in it fills the room that is left. In a row that scrolls
  sideways items keep their size, and are measured with all the width they ask for: the
  width of the row is no limit to them.
- **A box that scrolls sideways** in a page that scrolls (`scroll_x(scroll_x, content)`: a
  block of code, a wide table) is a strip that leaves the plain wheel to the page: it takes
  the wheel turned sideways, and the wheel with Shift. Its scrollbar runs along its bottom,
  as the bar of a scroll view runs down its right edge, and shows and is dragged the same
  way; a strip has none.
- `position: sticky` children are placed in the compose pass from the offset.

## Virtual lists

A virtual list places its own rows and answers its parent from its style alone, so it is
always a boundary. Its rows are absolutely placed children of a content node; what places
them is state (the scroll offset, the viewport's size, the heights), so the list is bindings
on that state and not a layout algorithm of its own:

- From its size, the scroll offset and the known or estimated heights it computes the row range
  to show, plus an overscan margin.
- It mounts or recycles rows for indexes entering the range (05), lays each out at the list's
  width, and positions them.
- It reports its content extent from measured and estimated heights.
- Rows mounted or given another index are measured after the layout that follows; a height
  that differs moves the rows after it, and the frame goes round again (02): at most four
  rounds a frame, the rest in the next one.

## Overlays

Overlay roots are laid out after the main tree, in stack order:

- Available size is the window.
- An anchored overlay is placed against its anchor's rect in window coordinates: preferred
  side, alignment, offset, then flipped or shifted to stay inside the window.
- An overlay whose anchor moved is placed again in the compose pass if only its position
  changes, in the layout pass if its available size does.
- An overlay opens on the side asked for unless only the other side has room, and is then
  moved as far as needed to stay inside the window. Its place is its layer's offset: an
  overlay that follows its anchor is laid out and painted by nobody.

## Rounding

Layout results are not rounded. Paint snaps edges to device pixels (07), so a fractional layout
never produces a blurred edge or a seam, and layout results do not depend on the display scale.

## Resize and scale

- A window resize asks the root a new request. Every node whose request is unchanged answers
  from its cache.
- A scale change keeps layout (logical pixels are unchanged) and flags paint for the window,
  because snapping and glyph sizes depend on scale.

## Tests

- **Conformance**: the flex and grid cases of the V suite, ported as data (style tree in,
  rects out).
- **The check**: `verify` lays a copy of the tree out from scratch and compares every rect with
  the incremental result. The whole test suite runs with it on.
- **Counters**: for every structural budget of 14 there is a test asserting how many nodes were
  asked and how many were measured.
- **Randomized**: seeded sequences of style changes, insertions and removals, each step
  checked with `verify`.

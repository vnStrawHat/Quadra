# 04 — The Node Tree

Each window owns one tree. It persists across frames; passes change it where it is flagged.

## Storage

Nodes live in a generational arena. `NodeId` is an index and a generation: a stale id is
detected, never dangling. Data is split by how often a pass touches it:

| Part | Contents | Read by |
|---|---|---|
| Links | parent, ordered children, scope, layer | every walk |
| Flags | dirty bits for itself and for its subtree, state bits | every pass |
| Style | declared style with its state variants; resolved style | style, layout, paint |
| Layout | rect relative to the parent, content size, layout cache | layout, compose, paint, hit test |
| Paint | the node's range in its layer's display list | paint, draw |
| Widget | `Box<dyn Widget>` and its own data | the pass that calls it |
| Input | handlers, focus, key context, cursor | dispatch only |
| Semantics | role, name, value, states | accessibility pass, tests |

Hot parts (links, flags, layout) are stored in parallel arrays indexed by the node's slot, so a
pass over flags does not pull styles and handlers into the cache. Input and semantics are
stored only for nodes that have them.

## Widgets

A widget is a node's behaviour. Most nodes use a built-in widget; a domain crate writes its own.

```rust
pub trait Widget: Any {
    /// Intrinsic size of a leaf (text, image). Containers are laid out from their style.
    fn measure(&mut self, cx: &mut MeasureCx, known: Size<Option<f32>>, avail: Size<Avail>) -> Size { Size::ZERO }
    /// For widgets that place their own children (virtual list, splitter).
    fn layout(&mut self, cx: &mut LayoutCx, size: Size) {}
    /// Emit this node's own instances, in the node's coordinates.
    fn paint(&mut self, cx: &mut PaintCx) {}
    fn event(&mut self, cx: &mut EventCx, event: &Event) {}
    fn semantics(&mut self, cx: &mut SemanticsCx, out: &mut SemanticNode) {}
}
```

- Every method has a default. A widget implements what it does.
- While a method runs, the widget is taken out of its node, so the context can borrow the tree
  mutably. A widget reaches other nodes only through the context.
- A context exposes what its pass may do: `MeasureCx` reads text metrics; `LayoutCx` places
  children and may mount or remove them; `PaintCx` emits instances; `EventCx` requests passes,
  moves focus, captures the pointer, writes signals.
- Built-in widgets: box (the container; no behaviour beyond its style), text, image, scroll,
  virtual list, text input, canvas. Everything in the kit is made of these (13).

A **surface widget** (terminal, editor, large canvas) asks for its own layer and owns that
layer's instances. It updates ranges of them directly and reports which changed (07).

## Properties and flags

Changing a property sets the flag of the pass that property needs. That table is the whole
invalidation model:

| Property changed | Flag | Example |
|---|---|---|
| A layout style value, text content, children | `LAYOUT` | width, padding, a label's text |
| A paint style value | `PAINT` | background, border colour, text colour |
| Transform, opacity, scroll offset | `COMPOSE` | a slide animation, scrolling |
| A state bit with a style variant | `STYLE` | hovered, on a node with a hover style |
| Role, name, value, a semantic state | `A11Y` | a checkbox toggled |
| A theme token | `STYLE` on the root, "all" | switching theme |

- Setting a flag also sets the matching "a descendant needs this" bit on each ancestor, and
  stops at the first ancestor that already has it. A pass walks down only through those bits.
- `LAYOUT` implies `PAINT` for the nodes whose rect changes; `PAINT` implies nothing above it.
- The style pass turns `STYLE` into `LAYOUT` or `PAINT` by comparing the newly resolved style
  with the old one. Equal means no flag.
- Setting a property to the value it has sets nothing.

## Passes

Order and looping are in 02. What each does:

- **Update**: runs the window's queued bindings. A binding calls its closure and writes one
  property. A closure cannot hold the tree, so it writes through a `Handle`, which queues
  the write; the pass applies the queue, and repeats while applying one queued more (a
  mount).
- **Style**: for each flagged node, resolves base style, state variants and tokens into the
  resolved style (13).
- **Layout**: 06.
- **Paint**: for each flagged node, calls `Widget::paint` and replaces the node's range in its
  layer (07). Clean subtrees are not visited. A layer met on the way is one marker; its own
  content is painted in a pass of its own if it has flagged nodes.
- **Compose**: for each layer whose place, offset or opacity changed, recomputes its
  transform, cut and opacity. No widget code runs. It comes after paint, because the paint
  walk is what finds the layers that moved with something above them.
- **Accessibility**: for each flagged node, calls `Widget::semantics` and adds the node to an
  incremental update. Skipped entirely while nothing listens (12).

Every pass counts what it visited into `FrameStats` (14). Tests assert on those counts.

## Mounting and removing

- Nodes are created by mounting a view (05), inside a scope. Mounting is allowed during the
  update pass (a `dynamic` region replacing its content, a keyed list changing) and during the
  layout pass (a virtual list realizing rows). New nodes start with every flag set.
- Removing a subtree disposes its scope, which removes its bindings, handlers and tasks,
  releases its glyph tiles and images through `Drop`, and frees the nodes. Its ranges leave the
  layer's display list in the same frame's paint pass.
- If a removed node had focus, focus moves to the nearest focusable ancestor. If it had the
  pointer captured, the capture ends.

## Node handles

App code reaches a mounted node through a `NodeRef`, filled when the view is mounted:

```rust
let field = NodeRef::new();
text_input(value).node_ref(field);
// later, in a handler:
field.focus();  field.focus_first();  field.scroll_into_view();  let r = field.rect();
```

`NodeRef` methods are no-ops on a node that is gone.

`layout_signal(node_ref)` gives a `Memo<Rect>` of the node's rect in window coordinates. It is
written after the layout pass, in its own batch, so state derived from a size costs one more
frame round instead of a loop inside the pass.

`Tree::after_layout(f)` runs `f` once, on the tree, after the next layout pass: for what must
see where nodes are before it can say where others go, and cannot wait for a size to change
(a list's box that was given another row may be exactly as tall as before). The overlays are
placed against their anchors before it runs, so it sees where a popup stands. State it writes
is seen by another round of the same frame.

## State bits

`hovered`, `active` (pressed), `focused`, `focus_visible`, `focus_within`, `disabled`,
`selected`, `checked`, `expanded`, `drop_target`. Dispatch sets the first five (09); the others
are properties. A bit that changes on a node with no style variant for it sets no flag.

## What this replaces

| V design | Here |
|---|---|
| `Element` with `request_layout`, `prepaint`, `paint`, rebuilt per render | Persistent nodes; widgets with one method per concern |
| Prepaint rebuilding hitboxes, handlers, key nodes, semantics, test ids | Node data, written once at mount, read when needed |
| Element state keyed by a hash of position | State on the node (01, R7) |
| The view as the unit of render, cache and incremental layout | No unit: flags per node, per pass |
| Render arenas, closure lifetimes, pinning | Scopes and `Drop` |
| Paint patches for hover and press | The general case: a `PAINT` flag on one node |

# 07 — State, Invalidation and Caching

Module `quadra.core` (`model.v`, `effects.v`, `subscription.v`, `invalidation.v`, `view_cache.v`).

## Models

```v
pub struct Model[T] { id u32  gen u32 }          // a plain value, cheap to copy
pub struct View[T]  { Model[T] }                 // a model whose T is Renderable

pub interface Renderable[T] { mut: render(mut cx RenderCx[T]) Element }
pub fn (v View[T]) any() AnyView                 // type-erased handle for roots and child slots

pub fn (mut cx Cx) new_model[T](value T) Model[T]   // owned by the current model, or the app
pub fn (mut cx Cx) new_view[T](value T) View[T]
pub fn (m Model[T]) read(cx &Cx) &T                  // tracked when called during render
pub fn (m Model[T]) peek(cx &Cx) &T                  // never tracked
pub fn (m Model[T]) update(mut cx Cx, f fn (mut v T, mut mcx ModelCx[T]))
pub fn (m Model[T]) update_silent(mut cx Cx, f fn (mut v T, mut mcx ModelCx[T]))
pub fn (mut cx Cx) release(m AnyModel)
```
- Values live in a store keyed by id. A stale handle (released id or older generation) returns an error from `try_read` and panics in debug from `read`.
- **`update` means changed.** It invalidates every dependent. Use `update_silent` for bookkeeping that nothing renders. This removes the "forgot to notify" bug class.
- Globals are app-owned models with a type key: `cx.set_global[Theme](t)`, `cx.global[Theme]()`. Reading a global during render tracks it like any model.

## Ownership
- Every model has an owner. The window owns its root view, a view owns the models and child views it creates, and the app owns models created outside any model.
- Releasing a model releases, depth-first, everything it owns, its subscriptions in both directions, its timers and its background tasks.
- Ownership can be moved with `cx.adopt(m)` when a model must outlive its creator.

## Automatic dependency tracking
- While a view renders, every `read` (models and globals) is recorded in that view's dependency set. The set is replaced on each render, so dependencies that are no longer read disappear.
- When a model changes, the effect queue marks its dependent views **render-dirty**.
- Reads inside event handlers, tasks and `peek` are not tracked.

## Observe and subscribe (side effects)

```v
// cx is ModelCx[Editor] (or RenderCx/EventCx of Editor); the subscriber type is inferred from it
core.observe(mut cx, doc, fn (mut v Editor, d Model[Doc], mut cx ViewCx[Editor]) { v.dirty_title = true })

pub struct Saved { path string }
mcx.emit[Saved](Saved{ path: p })                 // inside Doc's update
cx.subscribe[Doc, Saved](doc, fn (mut v Sidebar, d Model[Doc], e &Saved, mut cx ModelCx[Sidebar]) {
	v.recent.prepend(e.path)
})
```
- `observe` runs after any change of the target model. `subscribe` runs for one typed event. Both receive the subscriber's own state, so no closure capture is involved.
- Subscriptions belong to the subscriber and are released with it. Holding a token is optional: `cx.unsubscribe(sub)` cancels early.
- Rendering never needs `observe`, because tracking covers it. `observe` is for derived state, persistence, logging and similar side effects.

## Effect queue
Updates are applied immediately to the value, but notifications are queued. The queue is flushed when the outermost update, event or task callback returns:
1. Deliver emitted events to subscribers, in emit order.
2. Run observers of changed models, once per model per flush even after multiple updates.
3. Mark dependent views dirty and request a frame.
4. Process releases.

Callbacks during the flush can update further models; their effects join the same flush. A per-flush iteration limit (10,000) reports cycles instead of hanging.

## Invalidation levels

| Level | Triggered by | Work done |
|---|---|---|
| **Render** | a read model changed, a discrete event on the view, `notify()` | re-run `render()` for that view only, diff its layout nodes by position and id |
| **Layout** | a layout style change, text content change, child set change, window resize | re-layout from the nearest relayout boundary |
| **Paint** | hover/active/focus-visible patches of paint-only properties, caret blink, paint animations, image arrival | re-run prepaint and paint for that chunk only |
| **Compose** | scroll offset, overlay position, chunk move | recompose the scene, no element code runs |

Style properties are classified as layout or paint once, at compile time, in `layout.Style` and `ui.PaintStyle`. A hover patch that only touches paint fields can never cause a re-render.

## View cache
Every view is a cache boundary with this cached state:
- the last element tree (needed to repaint without re-rendering),
- its layout subtree in the persistent layout tree,
- its chunk: primitives, hitboxes, handlers, focus targets and semantics in local coordinates,
- its inputs key: available size from the parent, inherited text style hash, theme generation.

When the parent re-renders, `cx.child_view(v.sidebar)` reuses the child entirely if the child is clean and its inputs key is unchanged. Only its offset may change, which is a compose-level update. A dirty child re-renders without dirtying its parent.

Scroll containers inside a view get their own chunk, so scrolling one panel never repaints its neighbors.

## Element memory (render arenas)
A view that renders every frame (a live table, a dashboard) would otherwise build a new
element tree each time and leave the old one to the collector. `ui.div()` and `ui.text()`
(and `ui.svg()`, `ui.icon()`) called during a view's render take their element from the view's
**render arena** instead: a pool of elements the view used before, reset and handed out
again (`core/arena.v`).

- **Two arenas per view.** One holds the tree of the view's latest render (its cache); the
  other is free. A render fills the free one. At the end of the build in which the view
  rendered, the arena of the tree it replaced becomes the free one. That is the moment the
  previous render's closures are freed too (specs/14, closure lifetimes): the old tree is
  then no longer reachable from the window (root element, layout, hitboxes, handlers, paint
  patches, deferred draws all come from the new build).
- **Clean views are untouched.** A cached tree stays in its arena until the view renders
  again, whatever other views do.
- **Pins abandon arenas.** When a render is pinned (something long-lived was made in it, or
  a handler stored something that may hold elements: a model, a global, a popup, a drag
  preview), both of the view's arenas are dropped at the end of the next build in which it
  renders and left to the collector, so whatever kept elements from them stays valid.
- **Nested renders use the view's arena too.** A view reached while another render or a
  kit build is running (`ViewElement.request_layout` inside a `Lazy.build_tree`) renders
  into its own arena under the same rules, without a closure frame of its own; its closures
  belong to the enclosing frame. A view rendered twice in one build renders on the heap the
  second time: the first tree may still be placed.
- **Layout pass arenas.** What kit components build during layout (`Lazy.build_tree`, not
  inside a view's render) comes from the window's pass arena: the arena of the pass before
  last is reset at the end of a pass, when the pass that replaced it has rebuilt every
  holder. `keep_build` (an overlay opened from a build) or a pin during the pass drops the
  current and the previous pass arenas instead. The arena pin is separate from the closure
  pin of a frame: an escape during a render pins the arena being filled (the view's or the
  pass's) and those of the views rendered so far in the build (a nested view's tree is part
  of the tree that escaped); an escape from a handler, or `keep_build`, pins every arena
  that holds a tree.
- **Outside these the heap is used**: in handlers, constructors, tests, list rows and
  overlays built in prepaint. A view rendered by a prepaint-only frame (an overlay's view)
  can take an arena, then renders on the heap until the next build settles it. Only the main
  thread builds elements.
- **A recycled element is reset to its defaults** (`Div{}`, `TextEl{}`, `SvgEl{}`). A div
  keeps the capacity of its children, handlers and keys arrays (cleared) and its interaction
  states block reset to `States{}` (2.6 KB, reused by the next `hover`/`active`/`focus_ring`).
  A text keeps its owned buffer. An svg keeps its source, parsed document, hash and texture
  key only while the next use has the same source. Nothing reads stale fields.
- **Long menus cost nothing until opened**: `kit.menu_button_with(id, text, data, items_fn)`
  builds its items when the menu opens, so a render showing the button allocates no items.
- What may hold an element across renders, and why it is safe:

| Holder | Lifetime |
|---|---|
| a view's cached tree, `ViewElement.inner`, the window's root element | replaced by the build that renders the view again |
| layout nodes (`ctx` of a text leaf), hitboxes, handlers (`handler_with` data, a selectable text's listener), key nodes, deferred draws, drag sources, paint patches | rebuilt by every prepaint or paint, which follows every build |
| scroll thumbs (`ThumbTarget`) | point at element state or kit components, not at divs |
| a drag's preview | starting a drag pins every view's render |
| popups, modals, models, globals, observers, timers, tasks | creating or touching them pins (specs/14) |
| element state (`cx.state`), text caches, a11y nodes, test ids | hold copies or their own objects, never an element |

**Lifetime rule for app code:** an element returned from `render` belongs to that render.
Do not keep it in the view or anywhere else to return it again later; build it again (it is
cheap) or keep the data it shows. The framework's own registries pin as listed above.

## Tests
`model_test.v` (tracking, stale handles, ownership release), `effects_test.v` (ordering, coalescing, cycle limit), `view_cache_test.v`, which counts render, layout and paint calls per view in scripted scenarios on the headless platform.

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

## Tests
`model_test.v` (tracking, stale handles, ownership release), `effects_test.v` (ordering, coalescing, cycle limit), `view_cache_test.v`, which counts render, layout and paint calls per view in scripted scenarios on the headless platform.

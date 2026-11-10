# 06 — Core

Module `quadra.core`. App lifecycle, windows, the frame pipeline, dispatch, focus, overlays, semantics and animation. State, invalidation and caching are specified in 07-state.

## App and windows

```v
fn main() {
	mut app := core.new_app(core.AppOptions{ id: 'com.example.notes' })!
	app.open_window(core.WindowOptions{ title: 'Notes', size: core.size(1100, 720) },
		fn (mut cx core.WindowCx) core.AnyView {
			return cx.new_view(NotesApp{}).any()
		})!
	app.run()
}
```
- `App` implements `platform.EventSink`. It owns the platform, the model store, the text system, atlases, the keymap, globals and the task queue.
- `app.run()` blocks on the platform loop. `app.tick(dt_ms)` advances one iteration without blocking, for the headless test harness (11-devtools-testing).
- `Window` owns its `PlatformWindow`, `Renderer`, root view, layout tree, chunk cache, element state, focus, dispatch tables and semantic tree.
- `AppOptions.quit_on_last_window` defaults to true. Tray apps set it to false.

## Contexts
Every context embeds a base `Cx` with access to the app, the current window, globals, the model store and timers. Contexts that run on behalf of a model are generic in that model: `RenderCx[T]`, `EventCx[T]` and `ModelCx[T]`. APIs that need the current model's type (`listener`, `observe`, `subscribe`, `spawn`, `notify`) live on these and infer `T` from the receiver, so callers pass only the target and event types. Phase contexts without a model are `WindowCx`, `LayoutCx`, `PrepaintCx` and `PaintCx`. APIs valid anywhere take `&Cx`.

## Elements

```v
pub interface Element {
mut:
	request_layout(mut cx LayoutCx) layout.NodeId
	prepaint(bounds Rect, mut cx PrepaintCx)
	paint(bounds Rect, mut cx PaintCx)
}
```
- **Layout** creates or reuses nodes in the persistent layout tree.
- **Prepaint** receives final bounds and registers hitboxes, handlers, focus targets, scroll regions, tooltips, IME rect and **semantics**. All registrations are stored in the current chunk in local coordinates, so a cached chunk replays them without running element code.
- **Paint** emits primitives into the current chunk. `PaintCx` is the only place logical px become device px.
- Elements needing persistent state declare an `ElementId` and use `cx.state[T](id)`. State untouched for a frame is dropped.

## Frame pipeline
A frame runs only when something is invalid (07-state):
1. Flush effects. Dirty views re-render, clean views are reused.
2. Layout from the relayout boundaries of changed nodes.
3. Prepaint and paint for chunks whose render, layout or paint is invalid.
4. Compose chunks into the scene, compute overlap layers, draw, present with dirty rects.
5. Update the IME rect and the accessibility tree diff, and record profiler timings.

## Event dispatch
- **Mouse**: hit-test the composed hitbox list topmost-first. Handlers on the path run in capture order (root to target) then bubble order (target to root). `cx.stop_propagation()` ends the walk.
- Core derives hover, press, click, double click, drag threshold (4 px) and mouse capture during drags (`cx.capture_mouse()`). Hover changes are **paint invalidations** unless the hover style changes a layout property.
- **Modifier-gated cursor**: `HitboxOptions.cursor_mods` makes a hitbox's cursor apply only
  while those modifiers are held (`Window.mods`: the modifiers of the last mouse or key
  event). A key event that changes them updates the cursor at once and repaints when the
  hovered path holds such a hitbox, so a ctrl-hover affordance follows the key with no mouse
  move.
- **Hover hints** (`hint.v`): `HitboxOptions.hint` is a tooltip for a part of an element (a
  link in text, a URL in the editor or the terminal), where the kit's element tooltip cannot
  attach. Hover tracking keeps `Window.hint`: the deepest hovered hitbox with a hint, its
  bounds and when the pointer reached it; the same id, text and bounds are the same hint, so a
  slot reused for another link starts over. A new hint arms one prepaint at `hint_delay_ms`
  (500, also the kit tooltip's delay); a mouse press or a wheel closes it until the pointer
  reaches another hitbox or leaves the window. Core only tracks it: the kit's Host draws it
  (`kit.HintLayer`, specs/08). Nothing is allocated while no hint is hovered; the texts are
  constants (`hint_ctrl_open` "Ctrl+click to open", `hint_ctrl_follow` "Ctrl+click to
  follow", `link_hint(url)` picks one with a prefix test).
- **Keyboard**: along the focus path. The keymap resolves keystrokes into actions using the key contexts on that path, then action handlers run in bubble order. Unbound keys reach `on_key_down`. Text and IME events go to the focused element only.
- **Discrete events** (click, key, action, change) auto-notify the listening view. **Continuous events** (mouse move, wheel, drag) do not, so handlers call `ecx.notify()` only when state really changed.

## Listeners

```v
// inside fn (mut c Counter) render(mut cx core.RenderCx[Counter]) core.Element
kit.button('inc').label('+1').on_click(cx.listener(fn (mut v Counter, _ &core.ClickEvent, mut ecx core.EventCx[Counter]) {
	v.count++
}))
```
`cx.listener` returns a `Listener[E]` that receives the live view at dispatch time. `core.handler(fn (e &E, mut ecx &Cx))` is the variant without a view. Closures are never used to hold a mutable view, because V closures capture by value.

## Keymap
- Bindings: `app.bind('ctrl-k ctrl-s', OpenShortcuts{}, context: 'Editor')`. Multi-key chords with a 1 s timeout are supported.
- Context expressions: `'TextInput && !ReadOnly'`.
- User overrides load from a JSON file through `quadra.app` settings and are hot-applied.
- `app.bindings_for(action)` feeds shortcut labels in menus and tooltips.

## Focus
- `cx.focus_handle()` creates a handle owned by the view. Elements attach it with `.track_focus(h)`.
- Tab order follows paint order, overridable with `.tab_index(n)`. Modals and popovers trap focus. Focus is restored to the previous element on close.
- Focus-visible is tracked separately: the ring shows after keyboard navigation only.

## Overlays
- `cx.overlay(anchor, element, placement)` paints above the main tree. Core flips and shifts overlays to stay inside the window. A popup may instead use a separate `.popup` platform window when it must extend past the window edge (menus near the screen border).
- The window root provides modal, toast and tooltip services, so apps never have to install a root wrapper.

## Semantics
`.role(.button)`, `.label('Save')`, `.value(...)`, `.checked(...)`, `.selected(...)`, `.expanded(...)`. Kit components set these themselves. Prepaint collects them into a tree: each node carries its element id (stable across frames) and the index of the node enclosing it (`register_semantics` returns the index, `push_semantics` scopes the children). The test harness queries it (`find_role`).

Accessibility providers (`core/a11y.v`): while `PlatformWindow.a11y_active()` (assistive technology has asked for the window), each prepaint sends the tree with `update_a11y`; the first ask triggers one prepaint. Actions come back as `A11yEvent`: focus moves focus; invoke, toggle, select, expand and collapse click the node's center, so controls need no second code path. On Windows the provider is UI Automation (`platform/win/uia_*.v`): a fragment root answering `WM_GETOBJECT`, one object per node with a face per interface (Invoke, Toggle, SelectionItem, ExpandCollapse, read-only Value, by role), `UseComThreading` so COM brings every call to the UI thread, and focus-changed events. Not yet: property and structure change events, editable values, the text pattern.

## Animation
- `cx.animate(id, target, duration, easing)` returns the current interpolated value and keeps requesting frames until done.
- An animation of a paint property (color, opacity, transform of a chunk) is a paint invalidation. An animation of a layout property is a layout invalidation of that node only.
- All animations read the frame clock, which the headless harness controls.

## Tests
`dispatch_test.v`, `keymap_test.v`, `focus_test.v` and `semantics_test.v`, all on the headless platform.

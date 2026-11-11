# 05 — Building UI

App code describes UI with views. A view is consumed once, when it is mounted; it creates nodes
and binds their properties to signals. After that the signals drive the nodes directly.

## A component is a function that runs once

```rust
fn counter() -> impl View {
    let count = signal(0);
    row().gap(8.0)
        .child(button("−").on_click(move |_| count.update(|n| *n -= 1)))
        .child(text(move || count.get().to_string()))
        .child(button("+").on_click(move |_| count.update(|n| *n += 1)))
}
```

`counter` runs when it is mounted and never again. Clicking a button writes the signal; the one
binding that read it sets the text of one node; that node is laid out and painted. Nothing else
in the window is visited.

Consequences to keep in mind:

- Local variables of the function are not state. State is a signal.
- A value read outside a closure is read once. Only what a property closure reads is live.
- There is no `self`, no context parameter, no lifetime to carry: handles are `Copy`.

## Views

```rust
pub trait View {
    fn mount(self, cx: &mut MountCx) -> NodeId;
}
```

- Built-in views: `div()` (box), `row()`, `column()`, `grid()`, `text(..)`, `image(..)`,
  `scroll(..)`, `text_input(..)`, `canvas(..)`, and the control-flow views below.
- A component built from other views implements `Component { fn build(self) -> AnyView }`, or
  is simply a function returning `impl View`.
- `AnyView` is a boxed view. Containers box their children when they are added, so a tree of
  views is not one large generic type and adding a child does not instantiate new code
  (01, R17). `mount` runs once per node, so the dynamic call costs nothing that matters.

## Properties

A property takes a constant, a signal, a memo, or a closure:

```rust
div().width(240.0)                         // constant: written at mount, no binding
div().width(panel_width)                   // Signal<f32>
div().bg(move || if on.get() { ACCENT } else { SURFACE })
text(title)                                // Signal<String>
```

- A constant costs nothing after mount. Only reactive properties create bindings.
- A binding belongs to the scope the node was mounted in and is removed with it.
- Several properties set from one closure use `.style(move || Style { .. })`, one binding that
  writes a whole style; the tree diffs it and flags only what differs.

## Style and states

Style is data on the node, with variants per state:

```rust
div().bg(token::SURFACE).radius(6.0).padding(8.0)
    .hover(|s| s.bg(token::SURFACE_HOVER))
    .active(|s| s.bg(token::SURFACE_ACTIVE))
    .focus_visible(|s| s.ring(token::FOCUS))
    .disabled(|s| s.opacity(0.5))
```

The closures passed to `hover` and the others run once, at mount, and produce partial styles
stored on the node. Hovering sets a state bit; the style pass applies the variant. No app code
runs on hover. Values such as `token::SURFACE` are theme tokens, resolved in the style pass
(13).

Transitions are part of style: `.transition(Prop::Background, 120, Ease::Out)`. The tree runs
them; the property animated decides the pass, so a transform or opacity transition repaints
nothing.

## Frame callbacks

Animation that app code drives itself (a value that follows a spring, a scroll that glides, a
bench that changes its scene) is advanced by the frame, not by a timer:

```rust
each_frame(move || {
    offset.update(|y| *y += 8.0);
    offset.get_untracked() < end          // false: done
});
```

- The callback runs at the start of each frame of its window, before the update pass, inside
  the frame's batch. It writes signals; the frame it runs in shows the result.
- While a window has one, it draws a frame every time the display can take one. When the last
  returns `false`, the window goes back to sleep.
- It belongs to the scope it was registered in, runs in it, and stops with it. A scope that
  ends in a frame's update pass has had its callbacks run once more in that frame.
- A registered callback is kept, not queued again each frame: an animation allocates nothing
  per frame.

A component reaches its window's tree through context: the window's root scope provides the
tree's handle, which is how `each_frame` finds where to register.

## Handlers

```rust
.on_click(move |e| { selected.set(id); e.stop(); })
.on_key_down(move |e| if e.key == Key::Enter { submit.emit(()) })
.on_action::<Save>(move |_| save())
```

A handler is a boxed closure stored on the node and dropped with it. Its argument gives the
event data and the dispatch controls: `stop()`, `capture_pointer()`, `prevent_default()`,
`window()`, `node()` (09).

## Control flow

Because a component runs once, a plain `if` or `for` in it chooses once. Structure that changes
with state uses these views:

| View | Use | Cost of a change |
|---|---|---|
| `show(cond, \|\| view)` | A part that appears and disappears | Mount or remove that part |
| `switch(key, \|k\| view)` | One of several parts by a key | Remove the old part, mount the new |
| `each(items, key, \|item\| view)` | A keyed list of up to hundreds of items | Diff of the key list; mounts and removes only what changed |
| `virtual_list(count, \|index\| view)` | Any number of rows | Realizes rows entering the viewport |
| `dynamic(\|\| view)` | Anything; the coarse tool | Removes the region and mounts it again |

- **`each`** keeps the node of every key that stays; moved keys move their nodes. A row gets its
  item as a memo, so a changed item updates its row's bindings in place.
- **`dynamic`** tracks what its closure reads and rebuilds its whole region when that changes.
  It is how code written as "the UI is a function of this state" stays possible, for parts that
  change rarely or are small. Node state inside the region is lost on each rebuild; state that
  must survive lives in signals created outside it.
- Each of these is a mount boundary and owns one scope per mounted part.

## Virtual lists

```rust
virtual_list(move || rows.with(Vec::len), move |index: Memo<usize>| {
    row().child(text(move || rows.with(|r| r[index.get()].name.clone())))
})
.estimate(28.0)
```

- The list is a layer and a layout boundary. It keeps a pool of mounted rows.
- When a row leaves the viewport its nodes are kept and given to the next row entering: the
  list writes the row's `index`, and the row's bindings update. Steady scrolling mounts
  nothing and allocates nothing.
- Rows may have different heights. A row is measured when realized; unmeasured rows count with
  the estimate; the row at the top keeps its position when a measured height differs.
- Scrolling within the realized range changes only the layer's offset (07). The list asks for
  layout only when a row must enter or leave.
- `uniform_list` is the same with one fixed row height and no measuring.
- A row recycled for another index must not keep state of the old one in node state. The list
  resets focus, selection and scroll of recycled rows; anything else belongs in signals keyed by
  the item.

## Overlays

`overlay(anchor, || view)` mounts its content as a root of the window's overlay stack: a layer
above the main tree, positioned against the anchor's rect after the main layout (06). Menus,
popovers, tooltips, dialogs and drag previews are overlays. An overlay is owned by the scope
that opened it and closes when that scope ends or its `open` signal turns false.

## Context in views

`provide` and `expect` (03) work at any mount boundary. The kit provides the theme, density and
locale at the window root; a subtree can provide its own.

## Rules

- No procedural macros and no DSL. UI is Rust expressions.
- A component takes its inputs as arguments or a small builder struct. Inputs that can change
  are passed as signals or closures, not as values.
- Two-way controls take a signal: `text_input(value: Signal<String>)`. The control writes it;
  whoever else reads it updates.

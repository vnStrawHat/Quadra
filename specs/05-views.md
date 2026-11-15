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
- `image(source)` (11, "Images") is a leaf with a few methods of its own: `.width(..)`,
  `.height(..)`, `.grow()`, `.fit(..)`, `.radius(..)`, `.label(..)`, `.on_load(..)`. For
  padding, a background or a border it is put in a box.
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

The closures passed to `hover` and the others (`when(state, ..)` is the general form) run
once, when the view is built, and produce partial styles stored on the node. Hovering sets a state bit; the style pass applies the variant. No app code
runs on hover. Values such as `token::SURFACE` are theme tokens, resolved in the style pass
(13).

### The pose and transitions

Four values of a style say how what a box paints is placed over the box, and change nothing
of the layout: `.translate(Point)`, `.scale(k)` and `.rotate(degrees)`, both about the middle
of the box, and `.opacity(a)`. Together they are the box's pose. They are values like the
others: constants, reactive, or what a state changes (`.hover(|s| s.scale(1.06))`).

- A box whose style says one of them, in its own values or in a variant, is a layer (07):
  the pose is the layer's constants, so a pose that changes paints nothing.
- The pointer, the focus ring's place and the semantic tree go by the box as it is laid
  out: a box that lifts under the pointer does not leave it.
- A translation is in whole device pixels. What is scaled is drawn from what was painted
  at the box's own size: text is not shaped or rasterized again.

`.transition(ms)` makes the changes of what can ease go there over that time, along
`Ease::Out`; `.transition_with(ms, ease)` says the curve. What can ease: the background, the
border's colour, and the pose. The tree runs it; no app code does.

- The style the box is mounted with shows at once; so does what a new theme changes.
- A change starts from what shows, also in the middle of another change.
- A frame of a change of colours styles and paints that node; a frame of a change of the
  pose alone composes its layer and paints nothing (14).
- The window draws frames while a change is under way, and none once it has arrived.

`.animate(timeline)` plays a timeline from when the box is mounted: steps one after another,
each going over its time to what it sets of what can ease, from where the step before left
it.

```rust
let bounce = timeline()
    .to(380, Ease::Out, |s| s.translate(Point::new(0.0, -18.0)))
    .to(380, Ease::In, |s| s.translate(Point::ZERO))
    .forever();
div().animate(bounce)
```

- `.pause(ms)` is a step that changes nothing. `.repeat(n)` plays the steps n times,
  `.forever()` until the box goes; `.alternate()` plays every second time backwards.
- A timeline is a function of the time since it started and of what the box's style says:
  every play starts from the style's values, and a style that changes while it plays (a
  state, a signal) is what the next frame starts from. The transition of the box waits
  while a timeline plays.
- When the last play ends its values stay, until the style changes.
- `.with(ms, ease, ..)` is a step that starts together with the step before, over its own
  time: the two are one step, as long as the longer; of two that set the same thing, the
  later shows. `.then(timeline)` plays another timeline as a step, as many times as it
  says, each play from where the steps before it left (one that plays for ever plays once).
- `.animate_on(|| read, timeline)` plays the timeline from when the box is mounted and again
  from the start each time what `read` reads changes (a count the app moves on). A play
  again starts from what the style says, not from where the last one left the box.

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
- A tree has the clock of its window (`Tree::set_clock`, read with `Handle::now`): the
  platform's, which a test moves. What animates goes by it and not by counting frames.
- `tween(|| target, duration, ease)` is a signal that follows a target: it starts at the
  target, and each time the target changes it goes there over the duration, along the curve
  (`Ease::Linear`, `In`, `Out`, `InOut`), from wherever it is. The clock is read when the
  target changes: a window that was asleep has no recent frame to count from. While it is on
  its way it is a frame callback; once it has arrived it asks for nothing.

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

`on_click` is a click with the main button. `on_pointer(kind, ..)` is the general form: a
press, a release, a move, the pointer entering or leaving, a click with any button. A handler
is never given the tree: it writes signals, and what it wants of the tree it asks through the
tree's handle, which the next frame applies.

`.on_mount(|tree, node| ..)` runs once, with the tree, when the box is mounted: what a
component wants of its own node (a handler the box has no setter for, the node's id for
later, the tree's handle) it sets up there.

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
virtual_list(move || rows.with(Vec::len), 28.0, scroll_y, move |index: Signal<usize>| {
    row().child(text(move || rows.with(|r| r[index.get()].name.clone())))
})
```

The arguments: how many rows, the height a row counts with until it is measured, the signal
that says how far the list is scrolled, and what builds a row from its index.

- The list is a layer and a layout boundary. It keeps a pool of mounted rows.
- The box of a row lies across the list, and what the row builds fills it, whatever width it
  asked for (the list makes it grow): a line under a row goes from edge to edge, and text
  in it wraps at the list's width.
- When a row leaves the viewport its nodes are kept and given to the next row entering: the
  list writes the row's `index`, and the row's bindings update. Steady scrolling mounts
  nothing and allocates nothing.
- Rows may have different heights. A row is measured when realized; unmeasured rows count with
  the estimate. When a row that starts above the top of the viewport turns out taller or
  shorter than it counted, the scroll offset moves by the difference: what shows below it
  stays where it is. A row that changes height while mounted (what it holds, the list's
  width) is measured again.
- Heights are kept a row with running sums in a binary indexed tree: where a row is, which
  row is at an offset, and a new height are each a logarithmic number of steps, four bytes
  and a sum a row. A row added at the end is as many steps (its sum is its height and the
  sums its entry covers besides), and fewer rows cut the sums: a list that grows a row at
  a time does not count them all again.
- A box is measured after the layout that follows its getting a row, never before: the size
  it had belongs to the row it held (`Tree::after_layout`, 04).
- Scrolling within the realized range changes only the layer's offset (07). The list asks for
  layout only when a row must enter or leave.
- `.reveal(|| row)` keeps a row in view: when the closure gives another row the list scrolls
  as little as shows all of it. A row far away is only estimated to be where the list first
  scrolls to, so the list asks again as heights are found out, until the row shows whole and
  measured; then it lets go, and scrolling away is not undone.
- `uniform_list(count, row_height, scroll_y, row)` is the same with one fixed row height, no
  measuring and no memory a row.
- A row recycled for another index must not keep state of the old one in node state. The list
  resets focus, selection and scroll of recycled rows; anything else belongs in signals keyed by
  the item.

## Overlays

`overlay(place, view)` mounts its content as a root of the window's overlay stack: a layer
above the main tree, positioned against the anchor's rect after the main layout (06). Menus,
popovers, tooltips, dialogs and drag previews are overlays. An overlay is owned by the scope
that opened it and closes when that scope ends or its `open` signal turns false.

An overlay is open while its view is mounted, so it is opened and closed like any other part:
`show(open, || overlay(Place::Beside(anchor, Side::Below, Along::Start, 4.0), menu))`. Where
it stands in the flow it takes no room. It is placed beside the node of a `NodeRef`, at a
point of the window, in its middle, centred a distance under its top (`Place::Top`) or over
its bottom (`Place::Bottom`), or in its bottom right corner (`Place::BottomRight`).

A box lays its children out as a grid with `.grid(columns)` (tracks of `quadra_layout`) or
`.grid_fill(least)` (as many equal columns of at least that width as fit); a child spans
columns with `.col_span(..)`. `.gap_x(..)` and `.gap_y(..)` are the gaps by axis.

A box is taken out of the flow of the box around it with `.absolute()`, and placed against
that box by `.left(..)`, `.top(..)`, `.right(..)` and `.bottom(..)`, each a constant or
reactive: the knob of a slider. It is laid out alone when one of them changes. Pointer events reach the top overlay first; an
overlay is told of a press that lands outside it (`on_outside_press`), which is how a menu
closes; a focus scope in it keeps Tab inside.

## Context in views

`provide` and `expect` (03) work at any mount boundary. The kit provides the theme, density and
locale at the window root; a subtree can provide its own.

## Rules

- No procedural macros and no DSL. UI is Rust expressions.
- A component takes its inputs as arguments or a small builder struct. Inputs that can change
  are passed as signals or closures, not as values.
- Two-way controls take a signal: `text_input(value: Signal<String>)`. The control writes it;
  whoever else reads it updates.
- `text_input(value).password()` shows a dot for each character (a grapheme cluster): the
  caret, the selection and a click are placed on the dots as they would be on the
  characters; copy and cut do nothing; assistive technology is told the dots.
  `.on_submit(f)` runs `f` with the text when Enter is pressed, after whatever was typed
  before it in the same turn; Shift-Enter is left to what is around the input.
  `.label(words)` (a constant, a signal or a closure) is what a reader calls it; without one,
  its placeholder.
- `text_area(value)` is the same control on several lines: the text is broken at the width
  the area is given and the area is as tall as its lines; Enter starts a line; Up and Down
  move the caret by the lines as they are shown, keeping its x through lines too short for
  it; Home and End go to the ends of the shown line, with Ctrl to the ends of the text; a
  selection is painted line by line, with a little of each line break in it. Line breaks
  that are typed, pasted or set are line feeds. In a scroll view the caret is kept in view
  (06, "Scrolling into view", for the caret's rect). Its key context is "text-area", which
  binds what "text" binds and the keys of lines: on one line Up, Down and Enter stay keys,
  for a list around the input and for submitting.
- `text(..).selectable()` lets the user select static text: a drag selects (Shift and a
  press take the selection to the press), a double click a word, a third click or the key
  for it everything, and the key for copying puts the selection on the clipboard of the
  view's context. The text can have the focus; the selection goes when the focus does, and
  when the text changes. Its key context is "text-select", which binds those two keys only.
- `text(..).ellipsis()` is one line, as wide as its box lets it be: when the box is
  narrower than the text, what fits is painted and an ellipsis after it (shaped at measure;
  cutting is paint alone).
- `rich_text([span("was "), span("restarted").bold(), span("the guide").link(url)])` is one
  paragraph of spans, each saying what it has otherwise than the text around it:
  `.bold()`, `.weight(..)`, `.italic()`, `.size(px)`, `.family(name)`, `.color(..)` (a
  colour, or a role of the theme), `.background(..)` (a fill behind the span, as high as
  its line, on every line it is on), `.strike()` (a line through it, in its colour),
  `.link(target)`. It wraps as one text does (08). A link is underlined, in its own colour;
  the pointer is a hand over it, and a click on it gives its target to
  `.on_link(|target| ..)`. With `.ctrl_links()` on the text, for content its user did not
  write, a link is one only while Ctrl is held: the hand shows then, and the click counts
  then. To a reader it is a text, named by all of it. `.on_link_hover(|link| ..)` is told
  the link the pointer comes over, Ctrl held or not (the point of the window under its left
  end and its target), and `None` when it leaves them: what a hint of where it goes follows.
  - **What is no text** is in the flow as a span too: `inline(what)`, where `what` is an
    `InlineBox` (`quadra-tree`): it says how wide, how high and how deep it is in the style
    of its span, paints itself with the left end of its baseline where the line puts it, in
    the colour of its span, and says what it is to a reader, which is said in its place in
    the name of the text. It is in the word around it (a comma after it stays with it), and
    is measured before the text is shaped: the first time, and when the style changes. A
    formula is one (13, "Markdown"). A line with a box that reaches higher over the
    baseline, or lower under it, than its text is made that much higher, on whole pixels;
    each line has its height and its baseline (`Line::height`, `Line::baseline`), and a
    line with no such box is the style's.
  - Its text can be selected by whoever selects across texts (`select`, `selected`, `index_at`):
    a document does.
  - **Links for the keyboard and a reader**: a text with links is a box that holds it and a
    node for each link, in the order of the text, without a size of its own. That node
    takes the focus (Tab reaches the links in order, as it reaches any control), is a link
    to a reader, named by its words, and gives its target to `.on_link` on Enter or a
    reader's click (with `.ctrl_links()` too: a key pressed on purpose). While one has the
    focus the text outlines that link, a pixel out of its words, in its colour.
- A field lays its text out at layout; what reads the lines before the next layout (a key
  that moves the caret by lines, a click, in the same turn as an edit) lays it out first.

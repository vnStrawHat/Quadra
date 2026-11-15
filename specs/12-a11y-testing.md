# 12 — Semantics, Accessibility, Testing, Devtools

One description of the UI serves assistive technology, tests and the inspector: the semantic
tree. What a screen reader can reach, a test can reach, in the same way.

## The semantic tree

A node that means something to a user has semantics: a role, a name, and what applies of
value, description, states (disabled, checked, selected, expanded, focused), range, the actions
it supports, and relations (labelled by, controls). Nodes without semantics are skipped; their
semantic children attach to the nearest semantic ancestor.

- Built-in widgets and kit components set their semantics. A custom widget fills them in
  `Widget::semantics` (04).
- A change to anything semantic sets the `A11Y` flag on the node.
- Bounds come from layout and layers at the time they are asked for.
- A stable `test_id` may be set on any node. It is exposed to assistive technology as the
  automation id and to tests as a selector.
- What a node says of itself is what was set on it, completed by its widget: a text is a
  `Text` named by its text, an input a `TextInput` with its value. A view sets the rest:
  `.role(Role::Button)`, `.label(..)`, `.test_id(..)`, and the states `.checked(..)`,
  `.selected(..)`, `.disabled(..)`, `.expanded(..)`, each of which is at once what the node
  says and the state its style follows (13).
- A button, a link, an item, a checkbox and the like with no name are named by the text they
  hold, and that text is then part of them, not a node of its own.
- A click asked for semantically runs the click handlers on the node's path with no pointer
  and no hit test. Focus is the tree's own. The other actions go to what the node registered
  for them (an input sets its value). Expand, collapse and toggle that nothing is registered
  for are a click, when the node says it is in the other state: a view that says
  `.expanded(open)` and flips `open` on a click needs nothing more to be opened by a reader.
- A change is sent as the subtree of the semantic node it happened in: a text, a state, a
  list of children, a rect, a scroll offset. Focus goes with every update.
- **An update is well formed**, because the adapter of an OS refuses one that is not, and
  refusing there is a panic in the app. A node that is sent is the window or in the children
  of exactly one node the listener holds; a child was sent, now or before; the node said to
  have the focus is in the tree. So: a node that starts or stops meaning something, or being
  displayed, is a change of its parent; a node under one that is sent is not sent again;
  nothing is sent of a node that is not displayed, is under one that is not, or is in no
  window; overlays are children of the window; a focus on a node that means nothing is said
  to be on the nearest node around it that does, or on the window.
- `Mirror` in `quadra-a11y` is a listener that applies updates under those rules and holds
  what they add up to. Tests follow a tree with one: what it holds after every frame must be
  the tree. The tree's random test does so through insertions, removals, moves between
  parents, nodes in no window, hiding, layers, roles that change and a focus that moves.

## Accessibility

- `quadra-a11y` defines the semantic tree, its updates and the mirror. It has no OS code and
  no dependency, so everything in this spec runs headless, without a desktop session.
- **The bridge is the window** (10): the platform reports `SemanticsWanted` when assistive
  technology first asks for a window's tree; from then on the app gives the window what
  `take_semantic_update` returns, with `PlatformWindow::update_semantics`; what a reader asks
  of a node arrives as the event `Semantic { node, action }`, and the app has the tree `act`.
  The headless window keeps the updates it is given, for tests.
- On Windows the window answers `WM_GETOBJECT` with the adapter of `accesskit`, which speaks
  UI Automation (01, R11), and turns each update into that crate's. The first answer is the
  adapter's empty tree: the question can come while the app is in the middle of something, so
  the tree follows as an update, at once when no frame is due. Bounds are sent in logical
  pixels; the root carries the scale, and is sent again when the scale changes (a window
  moved to a display of another scale keeps its logical size). A static text is read by its value, a name is a label,
  `test_id` is the automation id; what can be clicked is told by its role. Requests reach the
  adapter on a thread of its choosing: they are queued and the window posted a message, so
  the app sees them on the UI thread like any event.
- A test in the native run reads a window that is never shown through UI Automation itself,
  from a second thread, as a screen reader does: it finds controls by automation id, reads
  name, control type, bounds and toggle state, invokes and toggles, and sees the new state.
- **Lazy**: until assistive technology asks for the tree, the accessibility pass does not run
  and no semantic node is built. On the first request the whole tree is built once. On a
  machine with no assistive technology a window is not asked (checked on the bench's
  windows), so the pass costs nothing there.
- **Incremental**: after that, each frame sends only what changed, plus focus.
- **Actions** arrive as semantic actions and are dispatched to the node's widget (09): click,
  focus, set value, toggle, expand and collapse, scroll into view, increment and decrement.
  They do not go through pointer events.
- Virtual lists expose the total count and each realized row's position in the set, so a
  reader announces "row 4,012 of 1,000,000" for a list that holds forty nodes. An item says
  where it is from 0 (`Semantics::position`, `.place_in_set(i)`), its container how many the
  set holds (`set_size`, `.set_size(n)`), as UI Automation asks: the kit's lists, tabs and
  options. A row of a tree says its level from 1 (`Semantics::level`, `.level(n)`) and its
  place among its siblings, with how many they are (`set_size` on the item), so a reader
  says "level 2, 1 of 2".
- Sliders and progress bars give their number in its range besides their text
  (`Semantics::range`, `.range(..)`); a slider is stepped by increment and decrement.
- Text inputs expose their value, selection and caret (`Semantics::selection`, in characters
  of the value: the anchor and the caret), and accept selection changes
  (`SemanticAction::Select`). A laid out input says its lines (`Semantics::lines`,
  `TextLine`): where each starts in characters of the value (every character is in one, a
  line break in the line it ends), its box from the node's corner, and where each of its
  characters starts and how wide it is. The bridge gives an input a text run (`TextRun`) a
  line, with those places, so a range of the text has bounds; the caret and the selection
  point into the runs, and a place in a run that a reader asks for is a character of the
  value. An input that says no lines (or more than 2 048) is one run with no places.
- A scroll view (`scroll`) is a `ScrollView` unless the view gives it another role.
- Relations (`Semantics::relations`, `Relation`): a node is named by another
  (`.labelled_by(&label)`), described by one (`.described_by(&hint)`: the hint or the error
  under a field), or changes what one shows (`.controls(&panel)`: a tab its panel, a combobox
  its list). The view gives a `NodeRef`, set once both are mounted (`Tree::relate`); a
  listener is told the semantic node the other is, the first one it holds, or the one it is
  part of, and a relation to a node gone or not shown is left out. The bridge maps them to
  the adapter's relations. ponytail: resolved when the relating node is described, so a
  target that becomes another semantic node is told only at the relating node's next change.

Requirements carried from the design language (behaviour/15): visible focus for keyboard users,
contrast of the themes, respect for the OS text scale and high-contrast setting.

## The harness

`quadra-testing` builds an app on the headless platform, the null renderer and a fake text
backend (10, 07, 08). A test:

```rust
#[test]
fn saving_clears_the_dirty_mark() {
    let mut h = Harness::new(notes_app);
    h.get_by_role(Role::TextInput, "Title").type_text("Plan");
    assert!(h.get_by_text("Plan •").exists());
    h.press("ctrl-s");
    assert!(h.get_by_text("Plan").exists());
    assert_eq!(h.stats().nodes_painted, 1);
}
```

- **Find** by role and name, by text, by `test_id`. A query that matches nothing or more than
  one node fails with the semantic tree printed.
- **Act** through semantic actions (`click`, `focus`, `set_value`, `toggle`) or through input
  (`pointer_move`, `pointer_down`, `press`, `type_text`, `wheel`, `drop_files`). Semantic
  actions are the default; input events are for testing input itself.
- **Time** is virtual: `advance(ms)` fires timers, runs tasks and animations; `frame()` runs
  one frame; `settle()` runs frames until nothing is flagged and no task is ready.
- **Background work** runs on the pool as in production; `settle()` waits for it.
- **The clipboard** is the headless one. No test touches the real clipboard.

## Counters

Every frame fills `FrameStats`:

```
bindings_run  nodes_styled  nodes_laid_out  nodes_measured  layers_composed
nodes_painted  instances_written  instances_uploaded  layers_uploaded
draw_calls  nodes_mounted  nodes_removed  words_shaped  glyphs_rasterized
```

A test asserts on them. This is how "work proportional to the change" is kept true: the
structural budgets of 14 are tests on counters, deterministic on any machine. A change that
makes a hover lay out a node fails a test, whatever its timing.

## Snapshots

- **Semantic snapshots**: the semantic tree as text. Cheap, stable, the default.
- **Display snapshots**: the layers and instances the null renderer received, as text. For
  paint logic.
- **Pixel snapshots**: an offscreen surface on the software device with real text, compared
  with a tolerance. For what only pixels show: the look of components, text rendering. These
  run in the native test run. The V screenshots of the gallery are the first references (00,
  phase R3).

  As built: `App::read_pixels(window)` reads back what a window drawn offscreen shows. The
  tool `bench/shots` runs the gallery on the headless platform with the software device and
  the real text backend, turns its pages one after another and writes each as a PNG (written
  uncompressed, by a few lines of its own); nothing shows on the desktop. A page that opens
  something (a menu, a select, a date picker, a dialog, a question, toasts, the palette, a
  tooltip, a context menu) is drawn a second time with it open, as `<page>-open.png`: the
  tool finds the opener by its test id in the semantic tree and sends the headless platform
  the pointer events of a click, a rest or a click with the other button, then Escape, a
  pointer that leaves, and the time the toasts last. With `QUADRA_SCALE=1.5` the pages are
  drawn as a display at 150 % shows them: what is drawn a device pixel at a time (the lines
  of charts) is looked at there too. `tools/shots.sh`
  runs it, and `tools/compare_shots.py` says for each picture what share of the pixels of
  the page differ from the reference by more than a tolerance; a band of rows may sit a
  pixel or two higher or lower than in the reference (the V pictures have lines of text of
  fractional heights, 08).

## Other test kinds

- **Fuzzing** of every parser that reads outside data (terminal sequences, Markdown with its formulas
  and diagrams, code, themes,
  keymaps, settings, images): deterministic, seeded, in `cargo test`, with the corpora carried
  from the V tree. `quadra_testing::fuzz` has what such a test is made of: `Rng` (the same
  numbers for a seed on every machine), `mutate(rng, input, corpus)` (bits flipped, bytes
  overwritten, a cut, a chunk doubled up to hundreds of times, a splice with another input,
  numbers at the edges of their widths, bytes that are not UTF-8, a chunk removed), and
  `run(name, rounds, limit, make, check)`, which fails with the number of the input that
  took longer than the limit. `QUADRA_FUZZ_ITERS` sets the rounds for a long run.
- **The debug layout check** (06): the suite runs with incremental layout compared against a
  full one.
- **Leaks**: after a window closes, the arenas of the reactive runtime and the tree, the layer
  set and the atlas handle counts are back to their values before it opened. The harness
  asserts this at the end of every test.
- **The comparison bench** and micro-benches (14) run outside the gate.

## Devtools

Built into debug builds, opened from a key binding or an environment variable:

- **Inspector**: pick a node; shows its place in the tree, declared and resolved style, state
  bits, layout rect and boundary, layer, semantics, and its bindings with the signals each
  read.
- **"Why did this update"**: for the last frame, each binding that ran with the signal that
  woke it and where that signal was written.
- **Paint flashing**: tints the instance ranges written this frame.
- **Layer view**: outlines layers with their draw order and reason for promotion.
- **Frame HUD**: the counters and pass times of recent frames.
- **Reactive graph dump**: signals, memos and effects of a scope with their edges, as text.

The inspector reads the retained tree and the reactive graph. It needs no instrumentation of
app code.

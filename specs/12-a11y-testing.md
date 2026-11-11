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

## Accessibility

- `quadra-a11y` keeps the semantic tree and its pending updates and defines the bridge trait.
  It has no OS code and no dependency, so everything in this spec runs headless, without a
  desktop session.
- The Windows bridge is in the native crate: it turns pending updates into the tree updates of
  `accesskit`, whose Windows adapter speaks UI Automation (01, R11). The harness uses a bridge
  that records updates instead.
- A test in the native run feeds recorded updates to that crate family's tree consumer, the
  library its adapters are built on, to prove the updates Quadra emits are well formed. It
  needs no window.
- **Lazy**: until assistive technology asks for the tree, the accessibility pass does not run
  and no semantic node is built. On the first request the whole tree is built once.
- **Incremental**: after that, each frame sends only the nodes flagged `A11Y`, plus focus.
- **Actions** arrive as semantic actions and are dispatched to the node's widget (09): click,
  focus, set value, toggle, expand and collapse, scroll into view, increment and decrement.
  They do not go through pointer events.
- Virtual lists expose the total count and each realized row's position in the set, so a
  reader announces "row 4,012 of 1,000,000" for a list that holds forty nodes.
- Text inputs expose their value, selection and caret, and accept selection changes.

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

## Other test kinds

- **Fuzzing** of every parser that reads outside data (terminal sequences, Markdown, themes,
  keymaps, settings, images): deterministic, seeded, in `cargo test`, with the corpora carried
  from the V tree.
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

# 09 — Input

Events are dispatched over the retained tree. Nothing is prepared per frame for them.

## Hit testing

A pointer position finds its target by walking down:

1. Root layers of the window from the top of the overlay stack to the main tree.
2. In a layer, the point is mapped through the layer's inverse transform and tested against its
   scissor.
3. In a node, children are tested in reverse paint order, then the node itself: inside its
   rect, inside its rounded clip if it clips, and `pointer_events` not `none`.

The first node that accepts is the target. The cost is the depth of the tree times the children
scanned at each level. A container with thousands of direct children that are not virtualized
scans them all; that is a known ceiling, and such a container should be a virtual list.

Hit testing uses layout rects and layer transforms, so it always agrees with what was drawn in
the last frame, including scrolled and transformed content.

## Dispatch

The path is the target and its ancestors.

- **Capture, then bubble**: handlers registered for capture run from the root down, then the
  others from the target up. `e.stop()` ends it.
- A handler is told where the pointer is in the window (`e.at`) and from the corner of the
  node the handler is on (`e.local`), and how large that node is (`e.size`): the share of a
  box the pointer is at needs no question to the tree.
- **Pointer capture**: a handler of pointer-down may call `e.capture_pointer()`. Moves and the
  release then go to that node wherever the pointer is, until release or the node is removed.
- **Click** is synthesized on release, for nodes on both the press path and the release path.
  Double and triple clicks are counted by time and distance.
- **Wheel** bubbles until a scroll container consumes it; what it cannot consume continues up.
  A line of the wheel is 40 logical pixels; a touchpad reports pixels. The event carries how
  far to scroll, and a container takes off it what it scrolled, so a list at its end hands
  the rest to the page around it within the same event.
- Every dispatch runs in a batch (03). Handlers write signals; nothing is redrawn until the
  frame.

Handlers are closures on nodes (05). A node with no handler for an event kind is skipped.

## Hover and press

- After each pointer move, and in each frame while the pointer is over the window, the hover
  path is computed again. Nodes entering and leaving get `hovered` set or cleared and enter or
  leave events. In a frame this is done once the layout is settled and overlays are placed,
  before painting: what scrolled under the pointer shows its hover in that frame. What the
  handlers of those events write is applied then too (bindings, styles); a layout they ask
  for waits for the next frame.
- Pointer-down sets `active` on the path; release clears it.
- While a node holds the pointer (capture), hover stays as it was; it catches up at the
  release.
- A state bit sets the `STYLE` flag only on a node that has a style variant for it (04). A
  window of plain content costs nothing to hover over.
- The cursor is that of the deepest node on the hover path that sets one (`.cursor(..)`), the
  arrow when none does; a text input sets the text cursor. It stays what it was while a node
  holds the pointer, as hover does. The app tells the window when it changed (10).
- Tooltips are overlays opened by the kit after a rest delay; the tree only reports hover.

## Focus

- A node is focusable when it sets `focusable` or a tab index. Focus is a `NodeId` per window.
- Clicking focuses the deepest focusable node on the path. Tab and Shift+Tab move in tree order,
  adjusted by tab index, inside the nearest **focus scope** (a dialog or menu traps focus).
  Nodes with a tab index above zero come first, lowest first, then the others in tree order;
  one with an index below zero is never reached by Tab, only by a press or by the program.
  From a node that Tab does not reach, Tab goes to the first of the walk. The node Tab goes
  to is scrolled into view (06).
- `focused`, `focus_visible` (focus arrived by keyboard) and `focus_within` (on ancestors) are
  state bits.
- Focus moves emit focus and blur events and flag accessibility.
- When the focused node is removed, focus goes to its nearest focusable ancestor, or to the
  node its focus scope names as fallback.
- **Disabled** is a state with a meaning for input: a node that has it, and everything in
  it, is not clicked (by the pointer or by a reader), does not take the focus from a press
  (the press goes on to what is around the outermost disabled node), and is passed by when
  Tab walks.
- A window that does not have the keyboard shows no focus: the states are cleared, the node is
  remembered, and both come back with the window.
- App code focuses a node through its `NodeRef` (04); the focus moves in the next frame.

## Keys, contexts and actions

Three steps for a key press:

1. **IME and text**: if the focused node accepts text and the platform reports composition or
   committed text, it gets text events (see IME).
2. **Keymap**: the key, with pending keys of a chord, is matched against bindings. A binding
   maps a keystroke sequence, in a context, to an action.
3. **Raw key events**: key-down and key-up dispatch along the focus path, capture then bubble.

**Contexts** are names on nodes (`.key_context("editor")`), interned at mount. The context of a
dispatch is the list of names from the focused node to the root. A binding's context is a
predicate over that list, so the deepest matching binding wins.

**Actions** are Rust types:

```rust
#[derive(Clone, PartialEq)] pub struct Save;
impl Action for Save { const NAME: &'static str = "file::save"; }
```

- Dispatch is by `TypeId`; `NAME` is what a user's keymap file and the command palette use.
- `on_action::<Save>(..)` on a node handles it for that subtree. The action bubbles from the
  focused node; window and app handlers come last.
- An action can carry data and be built from a keymap entry's arguments.
- Menus, the palette and buttons dispatch the same actions, so every command has one
  implementation and its shortcut is shown from the keymap.

The keymap is data: default bindings from the kit and the app, then the user's file. Bindings
are checked when loaded; an unknown action name or a malformed keystroke is reported, not
ignored.

How a key press is decided:

- A keystroke is written `ctrl-shift-p`, `f5`, `alt-enter`; a binding may be a sequence
  (`ctrl-k ctrl-c`). While a sequence is being typed its keys are taken; a key that continues
  no binding drops the sequence and is an ordinary key. A modifier pressed on its way to a
  keystroke is not one.
- Among the bindings a keystroke completes, the one whose context is nearest the focus wins;
  of equals, the one added last, so a user's binding replaces a default.
- The action goes from the focused node up to the first node with a handler for its type. If
  none has one the tree handles the actions it knows (moving the focus: Tab and Shift+Tab
  are bindings to `FocusNext` and `FocusPrevious`); if it knows none, the key is not consumed
  and goes on as a raw key event.
- `Tree::dispatch` and `Handle::dispatch` send an action the same way without a key: what a
  button, a menu item or the palette does.

## IME

The focused text input declares that it accepts text and gives its caret rectangle. The
platform reports composition start, update (text, caret, clause ranges) and end, and committed
text. The input shows the composition inside its paragraph (08) and places the candidate window
from the caret. The Win32 details are those of behaviour/02.

As built:

- A widget that takes text says where its caret is (`Widget::caret`). After every entry
  point the app gives the window the caret of the focused node, or none: the platform turns
  the input method on only over text, and opens its candidates beside that caret.
- One event carries a composition: the text being composed and the caret in it; an empty
  text ends it. What is committed arrives as ordinary text.
- The text input shows what is being composed at its caret, in place of the selection,
  underlined. It is not in the value, and not in the undo history, until it is committed.
- Not yet: clause ranges (the parts of a long composition shown differently).

## Drag and drop

- **Inside the app**: a node marked draggable starts a drag after the pointer moves past a
  threshold with the button down. The payload is a typed value; the preview is an overlay that
  follows the pointer (a layer translation, so moving it repaints nothing). Nodes that accept
  the payload's type get `drop_target` while it is over them, and the drop event on release.
- The threshold is 4 px. A press that stays under it is a click; one that passes it is a
  drag and no click: what the press held (the active state, a captured pointer) is let go.
- While the drag lasts the pointer's moves are the drag's: no node gets them, and hover
  stays as it was. The target is the deepest node under the pointer that takes values of the
  payload's type; a node that takes another type is no target.
- A node can ask to be **torn off** instead (`.draggable_off(px, ..)`): the press is a drag
  only once the pointer is that far above or below where it pressed. Until then the moves
  are the node's own, and what the press held stays held: a tab is moved along its strip by
  the strip, and dragged out of it by the drag.
- A target can follow the pointer (`.on_drag_over::<T>(|at| ..)`): it is told where the
  pointer is in the window at every move over it while a `T` is dragged, and `None` when the
  pointer leaves it or the drag ends. This is how it shows where a drop would land.
- The preview is held where the dragged node was pressed, and goes out of the window with
  the pointer (`Anchor::Free`). It is not hit, and is not in the semantic tree.
- Escape gives the drag up: nothing is dropped, and the release that follows does nothing.
- In views: `.draggable(|| value)`, `.draggable_with(|| value, || preview)`,
  `.draggable_off(px, || value, || preview)`, `.on_drop(|value: &T, at| ..)`,
  `.on_drag_over::<T>(|at| ..)` and `.drop_target(|s| ..)` for the style while over.
- **From the OS**: files and text dropped on the window arrive as drop events with the same
  targeting.

## Semantic actions

Assistive technology and tests act through semantic actions (12): click, focus, set value,
toggle, expand, scroll into view. They are dispatched to the node's widget as events of their
own kind, not as synthetic pointer events, so they work on nodes that are covered or outside
the viewport.

## Events while the tree is changing

Dispatch reads the tree as it is. A handler may remove nodes, including itself: removal is
deferred to the end of the dispatch, so the path being walked stays valid. Pointer capture and
focus are fixed up then (04).

## Tests

Dispatch order (capture, bubble, stop); capture across moves; click only when press and release
share a node; hover enter and leave on scroll without a pointer move; focus order and traps;
chords and context precedence; an action reaching the right handler; removal during dispatch.

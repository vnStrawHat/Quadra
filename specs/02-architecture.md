# 02 — Architecture

## Terms

| Term | Meaning |
|---|---|
| **Signal** | A value that records who reads it and tells them when it changes (03) |
| **Memo** | A value computed from signals, cached, recomputed only when a source changed |
| **Effect** | Code that runs again when a signal it read changes |
| **Binding** | An effect whose only job is to write one property of one node |
| **Scope** | The owner of signals, effects, tasks, timers and nodes; ends as a unit |
| **Node** | One element of a window's retained tree (04) |
| **Widget** | A node's behaviour: how it measures, paints, handles events, describes itself |
| **View** | A description that is consumed once to mount nodes (05) |
| **Component** | A function or type that returns a view |
| **Pass** | One step of a frame that visits the nodes flagged for it |
| **Layer** | A subtree with its own display list, transform, clip and opacity (07) |
| **Surface widget** | A widget that owns a whole layer and its instances: terminal, editor, canvas |

"Render" is not a term of this design: nothing is rendered again. Views are mounted; bindings
update; passes run.

## Crates

```
quadra-prim            Point, Size, Rect, Edges, Color, Affine              no dependencies
quadra-reactive        signals, memos, effects, scopes, events, context     prim-free, UI-free
quadra-layout          layout styles and the incremental flex and grid engine
quadra-vector          paths, the part of SVG icons need, rasterizing to coverage
quadra-text            paragraphs, wrapping, caches, the TextBackend trait
quadra-render          layers, display lists, instances, atlases, the Renderer trait
quadra-a11y            semantic nodes, incremental updates, the mirror tests follow them with
quadra-platform        Platform and Window traits, events, keys, the headless platform
quadra-tree            nodes, widgets, passes, input dispatch, focus, keymap
quadra-ui              views, properties, control flow, built-in widgets (box, text, image, scroll, lists)
quadra-async           local executor, background pool, inbox, timers
quadra-app             App, windows, the frame loop, settings, assets, i18n, logging
quadra-native-win      Win32 platform, D3D11 renderer, DirectWrite backend, the accessibility
                       adapter, image codecs, their wiring
quadra-shell           tray, notifications, single instance, crash reports, file dialogs
quadra-testing         harness, fake text, null renderer, snapshots
quadra                 the facade apps depend on
quadra-kit             components and themes
quadra-term  quadra-code  quadra-md  quadra-data  quadra-viz        domain crates
```

Dependency rules:

- A crate depends only on crates listed above it. `reactive` knows nothing of UI; `layout`,
  `text`, `render` and `platform` know nothing of each other or of the tree.
- Only `quadra-native-win` and `quadra-shell` contain OS code and `unsafe` (01, R16).
- `quadra-testing` builds an app from the headless platform, the null renderer and the fake
  text backend. Everything above the traits runs under it unchanged.
- A domain crate depends on `quadra` and `quadra-kit`, never the reverse. An app that shows no
  terminal does not compile one.

## Threads

- **The UI thread** owns the reactive runtime, every tree, the text system and the renderer.
  None of these types is `Send`. All app code in views, handlers, effects and local tasks runs
  here.
- **The background pool** runs blocking work handed to it (11). It sees no signal and no node.
  Results come back as values through a future or an inbox.
- **Reader threads** belong to whoever needs a blocking read (a pseudo terminal, a log tail) and
  deliver through an inbox.
- **The device worker** creates the hardware GPU device at start-up while the first frame is
  drawn on the software device (07).

The compiler enforces the split: a signal handle cannot cross to another thread.

## The frame

A window draws a frame only when something is flagged. One frame:

```
1. update     run frame callbacks (05), then queued bindings; they write node properties,
              which set flags
2. style      resolve styles of nodes whose state or tokens changed; diff decides layout or paint
3. layout     lay out dirty subtrees from their boundaries; realize virtual rows
4. paint      repaint dirty nodes into their layers' display lists
5. compose    recompute layer transforms, clips and scroll offsets that changed
6. a11y       send semantic updates, if assistive technology listens
7. draw       upload dirty ranges, draw the layers, present
```

Rules:

- A pass may flag an earlier one (a virtual list realizes rows during layout, which mounts
  nodes). The frame then repeats from that pass. After four rounds the frame draws what it has,
  reports the loop through the error hook in debug builds, and continues in the next frame.
- Which pass a change needs is a property of what changed (04, "Properties and flags"). App
  code never names a pass.
- Entry points (an OS event, a timer, a task completion, a frame) each run inside a batch (03).
  Side effects run when the batch ends; bindings wait for the next frame's update pass, so
  several events before a frame cost one update.
- With nothing flagged and no animation running, no frame is drawn and the thread sleeps in the
  OS message wait.

## Data flow

```
OS event ─► dispatch (hit test, focus path) ─► handler ─► signal.set
                                                              │ batch ends
                               effects run ◄──────────────────┤
                               bindings queued ◄──────────────┘
frame ─► update ─► style ─► layout ─► paint ─► compose ─► a11y ─► draw ─► present
background work ─► future or inbox ─► UI thread ─► signal.set
```

## Multiple windows

The reactive runtime is one per UI thread, shared by all windows. Each window owns a tree, an
overlay stack and a swap chain. A signal read in two windows flags nodes in both; each window
draws its own frame. Atlases and the device are shared.

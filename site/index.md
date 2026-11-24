# Quadra

Quadra is a native, GPU-rendered UI framework in Rust for desktop applications, with a kit of components and a design language of its own. Windows first: Win32, Direct3D 11, DirectWrite.

Its point is that **work follows the change**. A view is built once into a retained tree; signals are bound to the properties they change; layout is incremental; painted output is kept per layer. A frame lays out, paints and uploads what changed and nothing else, whatever the size of the UI.

{{code examples/gallery/examples/guide.rs#counter}}

A click writes the signal; the one text node bound to it is measured and painted again. Nothing else runs.

## What is in it

| Part | What you get |
|---|---|
| The framework (`quadra`) | Signals, memos and effects; views built once; flex and grid layout; text with wrapping, selection, input methods; overlays; focus, keys and actions; timers and background tasks; a semantic tree for assistive technology and for tests |
| The kit (`quadra-design`) | More than 80 components with one look: buttons, fields, pickers, menus, dialogs, tables, trees, tabs, a dock, settings, a command palette, notifications, and helpers that lay a view out |
| Beside the kit | Charts (`quadra-viz`), a code editor (`quadra-code`), markdown with formulas and diagrams (`quadra-md`), JSON and log views (`quadra-data`), a terminal (`quadra-term`) |
| Apps built with it | 1Term, a terminal with SSH and SFTP; Markdown, a reader and editor |
| Testing | A harness with no OS and no GPU: mount a view, find a control by its role and name, click, type, advance time, read the tree |

## Measured

On an i5-12400F with an RTX 3050 (the full numbers and their method are in [`docs/benchmarks.md`](https://github.com/vnStrawHat/Quadra/blob/main/docs/benchmarks.md)):

| Numbers on screen | A frame changes | Laid out | Painted | Time |
|---|---|---|---|---|
| 600 | one number | 3 | 1 | 2 µs |
| 9,600 | one number | 3 | 1 | 3 µs |
| 9,600 | every number | 28,808 | 9,608 | 11.3 ms |

A window with a counter reaches its first frame in 34 ms and holds 11.5 MB at rest; the binary is 2.81 MB. A list of 100,000 rows scrolls at 0.38 ms of CPU a frame.

## Where to go

- **Build something**: [Getting started](guide/getting-started.md), then [Concepts](guide/concepts.md) and [Building a view](guide/views.md).
- **See what there is**: [Components](components/index.md), each with pictures in both themes and the code that draws it.
- **Make it look right the first time**: [the design language](design/README.md): its [layout rules](design/layout.md), [patterns](design/patterns.md) and [checklist](design/checklist.md).
- **You are an agent**: [For agents](guide/agents.md) and [`llms.txt`](llms.txt).

## Status

Early, and moving. The design is in [`specs/`](https://github.com/vnStrawHat/Quadra/tree/main/specs); what is built and how it is tested is in [`docs/progress.md`](https://github.com/vnStrawHat/Quadra/blob/main/docs/progress.md). Windows 10 and 11, `x86_64-pc-windows-msvc`. The API still changes between commits.

# Quadra — Specs

Quadra is a native, GPU-rendered UI framework in Rust for building real desktop applications.
It targets Windows first, with boundaries a second OS can implement, and puts performance and
headless testability first.

Status: **design drafted 2026-11-11** (`docs/progress.md` is the source of truth for what is
built).

This is a redesign of the framework first written in V, not a translation: every subsystem was
designed from its goals, and what survives from the V implementation survives on merit (00,
"What carries over").

| File | Contents |
|---|---|
| [00-migration.md](00-migration.md) | Why, goals, what carries over, the repository, toolchain, phases, risks, the user's decisions |
| [01-decisions.md](01-decisions.md) | Architecture decisions R1–R19 |
| [02-architecture.md](02-architecture.md) | Terms, crates, threads, the frame |
| [03-reactive.md](03-reactive.md) | Signals, memos, effects, scopes, batching, events, context |
| [04-tree.md](04-tree.md) | The retained node tree, widgets, dirty flags, passes |
| [05-views.md](05-views.md) | Building UI: views, components, properties, control flow, lists |
| [06-layout.md](06-layout.md) | The incremental layout engine, scrolling, virtual lists, overlays |
| [07-render.md](07-render.md) | Layers, display lists, instances, atlases, device loss, D3D11 |
| [08-text.md](08-text.md) | Text backend, retained paragraphs, shaping cache, glyph tiles |
| [09-input.md](09-input.md) | Hit testing, dispatch, hover, focus, keymap and actions, IME, drag and drop |
| [10-platform.md](10-platform.md) | Windowing traits, shell services, the per-OS native crate, headless |
| [11-async.md](11-async.md) | Tasks, the background pool, inboxes, timers |
| [12-a11y-testing.md](12-a11y-testing.md) | Semantic tree, accessibility, harness, counters, snapshots, devtools |
| [13-kit.md](13-kit.md) | Style values and tokens, themes, component conventions, domain crates |
| [14-performance.md](14-performance.md) | Structural budgets, time and memory budgets, baselines, benches |
| [15-app-services.md](15-app-services.md) | Settings, window state, local time, language, log and crashes, pages, start and end |
| [16-shell.md](16-shell.md) | Dialogs, opening, tray and notifications, one instance, crash reports, packaging |

Reading order: 00, 01, 02, 03, 04, 05, then the rest.

## Behaviour specs

`behaviour/` holds specs carried from the V repository, under their old numbers. They say what
the framework and its components do and look like. Where they describe code (V APIs, modules,
the old frame pipeline) they are superseded by the files above.

| File | Still the reference for | Superseded by |
|---|---|---|
| [02-platform](behaviour/02-platform.md) | Win32 behaviour: IME, per-monitor DPI, the modal resize loop, the custom titlebar | 10 |
| [04-text](behaviour/04-text.md) | Shaping, fallback, wrapping and URL rules | 08 |
| [05-layout](behaviour/05-layout.md) | The style values of flex and grid | 06 |
| [08-elements-style](behaviour/08-elements-style.md) | Style values, image limits, scrolling and list behaviour | 05, 13 |
| [09-components](behaviour/09-components.md) | Every component's behaviour and states | 13 |
| [10-app-services](behaviour/10-app-services.md) | Settings, assets, i18n, lifecycle, logging, packaging | 11 for tasks, 15 and 16 for the rest |
| [13-apps-roadmap](behaviour/13-apps-roadmap.md) | The After-v1 list | 00 for phases |
| [15-design-language](behaviour/15-design-language.md) | The visual language and themes | nothing |
| [16-devtool-kit](behaviour/16-devtool-kit.md) | Panels, tables, trees, status | 13 |
| [17-terminal](behaviour/17-terminal.md) | The terminal's behaviour and limits | 04, 07 for the widget |
| [18-editor](behaviour/18-editor.md) | The code editor's behaviour | 04, 07 for the widget |
| [19-live-data](behaviour/19-live-data.md) | Live table, log view, value viewer, charts | 13 |
| [21-markdown-app](behaviour/21-markdown-app.md) | Markdown rendering rules, math, diagrams | 13 |

Not carried: the V specs of the old architecture (00, 01, 03, 06, 07, 11, 12, 14), the gap
analysis (20), and the apps (22–27), which are ported after Quadra is complete.

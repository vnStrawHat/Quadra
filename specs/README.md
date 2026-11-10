# Quadra — Specs

Quadra is a native, GPU-rendered UI framework written in V for building real desktop applications. It targets Windows first, with an architecture ready for Linux and macOS, and puts performance and headless testability first.

| File | Contents |
|---|---|
| [00-overview.md](00-overview.md) | Goals, scope, layers, directory layout, code rules |
| [01-decisions.md](01-decisions.md) | Architecture decisions |
| [02-platform.md](02-platform.md) | Windowing, input, IME, OS services, headless platform |
| [03-render.md](03-render.md) | Scene, chunks, overlap ordering, atlases, images, D3D11 |
| [04-text.md](04-text.md) | Text backend, DirectWrite, caching, graphemes, IME |
| [05-layout.md](05-layout.md) | Flex and grid, persistent tree, incremental layout |
| [06-core.md](06-core.md) | App, contexts, elements, frame pipeline, dispatch, focus, overlays, semantics, animation |
| [07-state.md](07-state.md) | Models, ownership, dependency tracking, observe/subscribe, invalidation, view cache |
| [08-elements-style.md](08-elements-style.md) | Primitive elements, style values and builder, scrolling, lists |
| [09-components.md](09-components.md) | Theme system and component kit |
| [10-app-services.md](10-app-services.md) | Settings, assets, i18n, tasks, navigation, lifecycle, logging, packaging |
| [11-devtools-testing.md](11-devtools-testing.md) | Headless testing, harness, CI, inspector, profiler |
| [12-performance.md](12-performance.md) | Principles, budgets, tooling |
| [13-apps-roadmap.md](13-apps-roadmap.md) | Gallery, Notes reference app, milestones |
| [14-implementation-notes.md](14-implementation-notes.md) | Where the implementation differs from 00–13, and why |
| [15-design-language.md](15-design-language.md) | Visual language: palette, type, spacing, motion |
| [16-devtool-kit.md](16-devtool-kit.md) | Devtool kit: panels, tables, trees, status |
| [17-terminal.md](17-terminal.md) | Terminal emulator element |
| [18-editor.md](18-editor.md) | Code editor |
| [19-live-data.md](19-live-data.md) | Live table, log view, value viewer, charts, cluster app |
| [20-gaps.md](20-gaps.md) | Gap analysis against other GPU UI frameworks, and the plan to close it |
| [21-markdown-app.md](21-markdown-app.md) | Folio, the Markdown reader and editor: layouts, outline, math, diagrams |
| [22-oneterm.md](22-oneterm.md) | OneTerm, the terminal application: tabs, Spaces, shell integration, config |
| [23-oneterm-ssh.md](23-oneterm-ssh.md) | OneTerm's SSH2 client and SFTP engine in V, with Windows CNG for the asymmetric crypto |
| [24-oneterm-completion.md](24-oneterm-completion.md) | Command completion and semantic highlighting |
| [25-oneterm-services.md](25-oneterm-services.md) | Vault, session store, updater, crash reports, terminal logs |
| [26-oneterm-session-ui.md](26-oneterm-session-ui.md) | The sessions panel and its dialogs |
| [27-oneterm-sftp-ui.md](27-oneterm-sftp-ui.md) | The SFTP panel: panes, transfers, edit in place |

Suggested reading order: 00, 01, 07, 06, then the rest.

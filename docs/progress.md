# Progress

Status per phase of specs/00-migration.md. Run everything with `tools/check.sh`.

| Phase | Status | Evidence |
|---|---|---|
| R0 repository and toolchain | Done (2026-11-11) | `tools/check.sh` and `tools/check.sh prod` pass; `quadra-prim` (points, rects, edges, corners, colours, affine transforms) with its tests |
| R1 vertical slice | In progress: reactive and layout done; render, text and tree at their first stage | See "R1" below |
| R2 interaction | Not started | |
| R3 kit | Not started | |
| R4 domain crates | Not started | |
| R5 release | Not started | |

On hold by the user's decision: `quadra-term`.

## R1

| Crate | Status | Evidence |
|---|---|---|
| `quadra-reactive` | Done (2026-11-11) | 31 tests covering every item of specs/03 "Tests"; no `unsafe`; 93 ns per binding in the `grid` shape (docs/benchmarks.md) |
| `quadra-layout` | Done (2026-11-11) | 39 tests: the 16 flex and 5 grid cases of the V suite, the costs of a change as counters, 1,500 random changes each checked against a layout from scratch; 0.82 µs for a change in a list of 10,000 fixed-height rows (docs/benchmarks.md) |
| `quadra-tree` | Passes done (2026-11-11): nodes, widgets, update, layout, paint into layers, compose. Not yet: style variants and tokens, the scroll widget and virtual lists, input, semantics | 13 tests: scrolling and moving a layer paint nothing and upload nothing, the counts of a frame for each kind of change, one change among 10,000 nodes costs one node, and 1,000 random changes each checked against a paint from scratch |
| `quadra-text` with the fake backend | Paragraphs done (2026-11-11); glyph tiles wait for the atlases | 12 tests: wrapping, hanging spaces, breaks between graphemes, a new width breaks lines without shaping, a changed text shapes only new words, the cache bound, caret stops |
| `quadra-render` with the null backend | Layers and display lists done (2026-11-11); atlases not started | 7 tests: in-place overwrites upload only their range, rebuilds copy clean ranges, child layers split runs, scrolling uploads nothing, culling, removed layers, device loss; the null renderer's copy is checked against the scene |
| `quadra-testing` | Started (2026-11-11): a headless window that checks every frame against a paint from scratch. Finding and driving controls through semantics waits for input and semantics | Used by the view tests |
| `quadra-ui` (views, enough for the four programs) | Started (2026-11-11): boxes and text with constant or reactive properties, `each`, `show`, `dynamic`. Not yet: scroll, virtual list, handlers, style variants | 6 tests: a component runs once, a change runs one binding, a keyed list keeps its nodes, a part's state goes with it; grid and document scenes measured headless (docs/benchmarks.md) |
| `quadra-native-win` (Win32 window, D3D11, DirectWrite) | Not started | |

Then the four programs (counter, a 10,000-row list, a 1,000-button grid, a long document), the
structural budgets of specs/14 as tests, and the comparison bench.

## Notes

- R0: the gate's dependency check, unsafe check and 300-line rule run
  on a workspace of one crate. The release settings in `Cargo.toml` are left at cargo's
  defaults until phase R1 measures them.
- Not carried yet from the V repository: themes, fonts, icons, shaders, test corpora, the
  comparison bench. Each is brought by the phase that first needs it.

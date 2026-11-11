# Progress

Status per phase of specs/00-migration.md. Run everything with `tools/check.sh`.

| Phase | Status | Evidence |
|---|---|---|
| R0 repository and toolchain | Done (2026-11-11) | `tools/check.sh` and `tools/check.sh prod` pass; `quadra-prim` (points, rects, edges, corners, colours, affine transforms) with its tests |
| R1 vertical slice | In progress: 1 of 8 crates | See "R1" below |
| R2 interaction | Not started | |
| R3 kit | Not started | |
| R4 domain crates | Not started | |
| R5 release | Not started | |

On hold by the user's decision: `quadra-term`.

## R1

| Crate | Status | Evidence |
|---|---|---|
| `quadra-reactive` | Done (2026-11-11) | 31 tests covering every item of specs/03 "Tests"; no `unsafe`; 93 ns per binding in the `grid` shape (docs/benchmarks.md) |
| `quadra-layout` | Not started | |
| `quadra-tree` | Not started | |
| `quadra-text` with the fake backend | Not started | |
| `quadra-render` with the null backend | Not started | |
| `quadra-testing` | Not started | |
| `quadra-ui` (views, enough for the four programs) | Not started | |
| `quadra-native-win` (Win32 window, D3D11, DirectWrite) | Not started | |

Then the four programs (counter, a 10,000-row list, a 1,000-button grid, a long document), the
structural budgets of specs/14 as tests, and the comparison bench.

## Notes

- R0: the gate's dependency check, unsafe check and 300-line rule run
  on a workspace of one crate. The release settings in `Cargo.toml` are left at cargo's
  defaults until phase R1 measures them.
- Not carried yet from the V repository: themes, fonts, icons, shaders, test corpora, the
  comparison bench. Each is brought by the phase that first needs it.

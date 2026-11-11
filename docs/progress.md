# Progress

Status per phase of specs/00-migration.md. Run everything with `tools/check.sh`.

| Phase | Status | Evidence |
|---|---|---|
| R0 repository and toolchain | Done (2026-11-11) | `tools/check.sh` and `tools/check.sh prod` pass; `quadra-prim` (points, rects, edges, corners, colours, affine transforms) with its tests |
| R1 vertical slice | Not started | |
| R2 interaction | Not started | |
| R3 kit | Not started | |
| R4 domain crates | Not started | |
| R5 release | Not started | |

On hold by the user's decision: `quadra-term`.

## Notes

- R0: the gate's dependency check, unsafe check and 300-line rule run
  on a workspace of one crate. The release settings in `Cargo.toml` are left at cargo's
  defaults until phase R1 measures them.
- Not carried yet from the V repository: themes, fonts, icons, shaders, test corpora, the
  comparison bench. Each is brought by the phase that first needs it.

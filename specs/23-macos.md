# 23 — macOS

Quadra on a second operating system: macOS on Apple silicon (`aarch64-apple-darwin`). Started
by the user on 2026-11-26, on the branch `macos`. Windows stays the first system: nothing
here changes what a Windows build is or measures.

## Goals

- **The same framework above the traits.** A second OS is a second native crate and a second
  module of the shell (10, "The native crate"; 16, "Traits and fakes"): `Platform`,
  `PlatformWindow`, `Renderer`, `TextBackend`, `Clipboard` and `ImageDecoder` are implemented
  again, and nothing above them knows which system it runs on. Where the traits turn out to
  name a Windows idea, the trait changes, for both systems.
- **Native, as on Windows**: the system's own windows, GPU API, text and accessibility, with
  the budgets of 14 measured again on a Mac. No layer that draws for both systems.
- **Never a broken Windows build.** The Windows gate (`tools/check.sh`) stays what a commit
  must pass.
- **Quadra, not OneTerm.** The app has moved to its own repository (00, the decisions of
  2026-11-24); what is still under `apps/oneterm` here is not ported, fixed or checked for
  macOS (the user's decision, 2026-11-26). OneTerm on a Mac is that repository's work, on
  what Quadra gives it there.

## What is built

| Part | Windows | macOS |
|---|---|---|
| Windows, events, the loop, input methods, cursors, drops | Win32 (`quadra-native-win`) | AppKit (`quadra-native-mac`, new) |
| Renderer | D3D11 | Metal |
| Text | DirectWrite | Core Text |
| Accessibility | UI Automation, by `accesskit_windows` | NSAccessibility, by `accesskit`'s adapter for macOS (01, R11) |
| Image decoding | WIC | Image I/O |
| Shell services (16) | `quadra-shell/src/win` | `quadra-shell/src/mac` |
| Sealing, hashing, fetching (16) | CNG, WinHTTP | the system's (CommonCrypto, URLSession), decided in M3 |
| Pseudo terminal (18) | ConPTY | the POSIX pty of `oneterm-vt` |
| Updates (22) | an `.exe` replaced while it runs | an app bundle; decided when an app ships there |

The bindings are a decision of M2 (01, R15: a new crate is admitted there first).

## Open decision: the renderer

A Metal backend written for Quadra, as the D3D11 one was (07), or one backend over a crate
that draws on both systems. The first is what the goals above say and what `Renderer` was cut
for; it is the plan unless the user decides otherwise before M2. M0 and M1 do not depend on it.

## How it is developed without a Mac

The user's machine runs Windows; a Mac is not at hand yet.

- **From Windows: `tools/check_mac.sh`** runs clippy for `aarch64-apple-darwin`. That needs no
  Mac: nothing is linked, so no linker and no SDK (Apple's SDK may be used on Apple's hardware
  alone, so nothing is linked for macOS here). It finds what does not compile there.
- **On a Mac: the same script** runs clippy and the tests natively. The CI does
  (`.github/workflows/macos.yml`, a runner of GitHub's, on a push to `main` or `macos` and on
  a pull request). The tests of the framework need no OS and no GPU (12), so they say on a
  Mac what they say on Windows.
- **What a runner cannot say**: how a window feels. Resizing, the trackpad, input methods,
  Retina, the menu bar and the timing of a frame are tried on a real Mac, by hand, as the
  manual checks of R5 were on Windows. M2's window work waits for one.

## What is left out, for now

`tools/check_mac.sh` names the packages that are not built for macOS; the list is what is
left of the port, and a milestone ends by taking names off it:

- `quadra-native-win`, and the tools that draw or measure with it (the shots, the comparison
  bench and its runner, the fire demo): until the native crate of macOS draws (M2).
- The app `markdown`: its `main` asks the shell for the system's services (crash reports, one
  instance, dialogs) (M3).
- Every package of OneTerm, for good (Goals).

Until M2 the facade has no native crate to build an app on there: `quadra::App` is an app on
no GPU (`NullRenderer`), so that what names the type builds, and `quadra::app()` says that
Quadra has no native backend for this system yet.

## Milestones

| | Builds | Exit |
|---|---|---|
| **M0** The workspace builds for macOS | The facade takes its native crate by system; `tools/check_mac.sh`; the CI's job | `tools/check_mac.sh` passes on Windows; the job is green on a Mac |
| **M1** The same tests on a Mac | What the tests find that is Windows' alone above the traits (paths, the user's folders, line ends, the environment) | Every test of the packages built for macOS passes on the runner |
| **M2** The native crate | `quadra-native-mac`: Core Text, then Metal drawn without a window and compared with the D3D11 pictures of the same scenes, then AppKit windows and input | The gallery runs on a Mac; its pages match the Windows ones within the snapshot tolerance; the manual checks pass on a real Mac |
| **M3** The shell | `quadra-shell/src/mac`: dialogs, opening, one instance, crash reports, notifications; sealing and fetching | `markdown` runs on a Mac |
| **M4** The terminal and the updater | `quadra-term`'s session over the POSIX pty of `oneterm-vt`; the updater's app bundle | The terminal's tests pass on a Mac; a terminal runs in the gallery there |
| **M5** Measured | The budgets of 14 on a Mac, in `docs/benchmarks.md` | Each budget met or the gap explained |

Each ends as a phase does (00): its exit met, `docs/progress.md` updated.

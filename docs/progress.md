# Progress

Status per phase of specs/00-migration.md. Run everything with `tools/check.sh`.

| Phase | Status | Evidence |
|---|---|---|
| R0 repository and toolchain | Done (2026-11-11) | `tools/check.sh` and `tools/check.sh prod` pass; `quadra-prim` (points, rects, edges, corners, colours, affine transforms) with its tests |
| R1 vertical slice | Done (2026-11-11), with the evidence for its go/no-go: the four programs run in real windows and the comparison bench is at the V build's numbers or under them. The structural budgets that need input or style variants move to R2 and R3 with those features | See "R1" below and docs/benchmarks.md |
| R2 interaction | In progress (2026-11-11): pointer input (hit testing, dispatch with capture and bubble, hover, press, pointer capture, clicks with counts) from the platform's events to handlers on views; styles with state variants (`hover`, `active`, `when`), resolved by the style pass for the nodes whose state changed. Not yet: tokens and themes (R3), the wheel, focus, keys and actions, text input and IME, overlays, semantics | `crates/quadra-tree/tests/input.rs` (6), `tests/style.rs` (3: the budget "pointer enters a button" is one node styled, none laid out, one painted), `crates/quadra-ui/tests/input.rs` (3), one in `crates/quadra-app/tests/app.rs`; the bench's counter counts clicks posted to its real window |
| R3 kit | Not started | |
| R4 domain crates | Not started | |
| R5 release | Not started | |

On hold by the user's decision: `quadra-term`.

## R1

| Crate | Status | Evidence |
|---|---|---|
| `quadra-reactive` | Done (2026-11-11) | 31 tests covering every item of specs/03 "Tests"; no `unsafe`; 93 ns per binding in the `grid` shape (docs/benchmarks.md) |
| `quadra-layout` | Done (2026-11-11) | 39 tests: the 16 flex and 5 grid cases of the V suite, the costs of a change as counters, 1,500 random changes each checked against a layout from scratch; 0.82 µs for a change in a list of 10,000 fixed-height rows (docs/benchmarks.md) |
| `quadra-tree` | Passes done (2026-11-11): nodes, widgets, update, layout, paint into layers, compose; text painted from glyph tiles (`Fonts`). Pointer input: `hit_test`, `pointer`, handlers on nodes (`on_pointer`), hover and press state; a node's style as its own values plus what each state changes (`Patch`, `State`, `set_variant`, the style pass). Not yet: tokens, inherited text values, focus and keys, semantics | 13 tests: scrolling and moving a layer paint nothing and upload nothing, the counts of a frame for each kind of change, one change among 10,000 nodes costs one node, and 1,000 random changes each checked against a paint from scratch |
| `quadra-text` with the fake backend | Paragraphs and rasterization through the backend done (2026-11-11) | 12 tests: wrapping, hanging spaces, breaks between graphemes, a new width breaks lines without shaping, a changed text shapes only new words, the cache bound, caret stops |
| `quadra-render` with the null backend | Layers, display lists and atlases done (2026-11-11); large images as their own textures wait for images | 12 tests: tiles packed in shelves, a tile in use is never evicted, the budget, in-place overwrites upload only their range, rebuilds copy clean ranges, child layers split runs, scrolling uploads nothing, culling, removed layers, device loss; the null renderer's copy is checked against the scene |
| `quadra-platform` | Started (2026-11-11): the `Platform`, window and handler traits, events, keys, and the headless platform (a clock the test moves, queued events, stepping). Not yet: composition, cursors, placement, displays, appearance, the clipboard, the custom titlebar's hit test, each added with what first uses it | 6 tests: timers fire in order at their time on the virtual clock, a step delivers events then the frames asked for, a waker from another thread wakes the loop once, a dropped window is closed |
| `quadra-app` | Started (2026-11-11): `App` owns the platform, the renderer, the fonts and the windows; it draws a frame for a window when its tree has something to do, follows resizes, ends the loop when the last window closes, and makes a lost device again. Not yet: input dispatch, tasks and timers, settings, assets, i18n, logging | 4 tests on the headless platform and the null renderer: a frame is drawn when something changed and not otherwise, windows share signals, closing frees what the renderer kept, a lost device is recovered with every glyph rasterized again |
| `quadra` (the facade) | Started (2026-11-11): gathers what an app names and builds an `App` on the Win32 platform, the D3D11 renderer and DirectWrite (`quadra::app()`) | The comparison bench's four scenes (`bench/compare`) run in real windows and show what the V build's show; first numbers in docs/benchmarks.md |
| `quadra-testing` | Started (2026-11-11): a headless window that checks every frame against a paint from scratch. Finding and driving controls through semantics waits for input and semantics | Used by the view tests |
| `quadra-ui` (views, enough for the four programs) | Started (2026-11-11): boxes and text with constant or reactive properties, `each`, `show`, `dynamic`, `scroll`, `uniform_list`, frame callbacks (`each_frame`), `mount_with` (a window's view, built where it can reach its tree), `on_click` and `on_pointer`. Not yet: rows of different heights, handlers, style variants | 18 tests: the structural budgets of specs/14 that can be met so far (7 rows of its table, in a window with 10,000 other nodes), 100,000 rows mount 32, scrolling within them paints nothing, a row that leaves is given to the row that enters; a component runs once, a change runs one binding, a keyed list keeps its nodes, a part's state goes with it; the three scenes measured headless (docs/benchmarks.md) |
| `quadra-native-win` (Win32 window, D3D11, DirectWrite) | Started (2026-11-11): the D3D11 renderer draws a scene into an offscreen surface (layers, runs, scissors, opacity, uploads of ranges, recovery). Glyph and image atlases are texture arrays kept in step by `sync_atlas`. The Win32 platform (`WinPlatform`): windows sized in logical pixels at the display's scale, the message loop with frame pacing, timers, wakes, pointer, wheel, key and text events, frames inside the OS's resize loop. Window surfaces are flip-model swap chains whose frame-latency handle paces the loop (`D3d11Renderer::for_platform`); a lost device gives each window a new one. At start-up one thread makes the software device, which draws the first frames (copied to the window), then the hardware device, which the renderer moves to through the app's recovery (`D3d11Renderer::starting`). The DirectWrite text backend (`DirectWrite`): font matching, script analysis, fallback for what a face lacks, shaping with clusters, coverage per pixel or per subpixel (ClearType, as the Windows setting says), colour glyphs composed from their layers, fonts from memory. Not yet, of text: OpenType features, bidirectional analysis; of the platform: composition (IME), cursors, placement, the custom titlebar, appearance, the clipboard, file drops | 6 pixel tests on the software rasterizer, no GPU or window needed: one goes from a view and a signal through the tree and glyph tiles to pixels. 6 platform tests on windows that are never shown: posted input arrives as events in logical pixels, a resize is reported, what a handler sets off is delivered after it returns, timers, a wake from another thread, a frame when asked. 2 surface tests on such a window: the back buffer holds what the scene says through resizes and a new device, and frames are given as the swap chain takes them; a starting renderer copies its frames, wakes the loop when the other device is made and moves to it. 7 DirectWrite tests against fonts every Windows has: metrics per em, clusters of Vietnamese text as bytes, fallback faces for CJK on one pen, an emoji as a premultiplied colour image, a font loaded from memory, and text from a view to pixels |

### The exit of R1 (specs/00, Phases)

- **The four programs run**: `bench/compare` has the counter, a virtual list of 100,000 rows, a
  grid of 600 cells that all change every frame, and a document of 40 paragraphs wrapped to a
  width that changes every frame, each in a real window on the hardware device with
  DirectWrite text. `tools/shot.py` saves what a window shows.
- **The structural budgets** (specs/14) that need no input are tests: nothing, a text that
  changes with and without its size, one row among 10,000, scrolling plain content inside and
  past the painted band, a virtual list scrolled within and across rows. Hover, press, focus,
  transitions, the caret and a theme change wait for input (R2) and style variants (R3).
- **The comparison bench** (medians of three rounds, V then Rust): list 0.38 and 0.36 ms of
  CPU a frame, grid 1.31 and 1.29, document 0.73 and 0.70; start-up 44 and 41 ms; binary 1.63
  and 0.88 MB; idle memory 10.0 and 11.1 MB. docs/benchmarks.md has the table, and what it
  took to get there.

So the decisions the phase was to test stand (specs/01, R1 to R5): a retained tree, signals
with views built once, passes over what changed, display lists kept per layer, and the
layout engine of requests and answers. The one scene built to make this design lose, a grid
whose every cell changes every frame, costs what the V build's costs.

## Notes

- R0: the gate's dependency check, unsafe check and 300-line rule run
  on a workspace of one crate. The release settings in `Cargo.toml` are left at cargo's
  defaults until phase R1 measures them.
- Not carried yet from the V repository: themes, fonts, icons, shaders, test corpora, the
  comparison bench. Each is brought by the phase that first needs it.

# 11 — Testing and Devtools

Modules `quadra.testing` and `quadra.devtools`. Testing is a first-class target from M1, not an afterthought.

## Headless by design
Tests must run with **no interactive login session**: CI runners, SSH sessions, Windows services and scheduled tasks, Session 0, containers, and Linux CI. To achieve that, every OS boundary has a headless implementation that is part of the framework, not a mock written per test.

| Boundary | Headless implementation | Needs OS? |
|---|---|---|
| `Platform` | `platform/headless/`: in-memory windows, event queue, clipboard, dialogs with scripted answers, paths under a temp dir | No, pure V |
| Clock | Virtual clock: timers, animations, debounce, caret blink and toast expiry all advance only through `h.advance(ms)` | No |
| `Renderer` | `gpu/null/`: records the composed scene and stats, draws nothing | No |
| `Renderer` (pixels) | `gpu/d3d11` offscreen mode: forced WARP, render target texture, readback to PNG, no swap chain, no HWND | Windows only, no desktop needed |
| `TextBackend` | `text/fake/`: deterministic metrics (advance 0.6 em per grapheme, fixed ascent/descent) | No |
| `TextBackend` (pixels) | DirectWrite with bundled test fonts only and system fallback disabled, for identical output on every machine | Windows only, no desktop needed |

Selecting backends is a harness option, never a code change in the app. The same app code runs in the real window and in the harness.

## Harness API

```v
fn test_save_note() {
	mut h := testing.new_harness(size: core.size(900, 600))!
	h.open_window(notes.root_view)!
	h.click(testing.by_test_id('new-note'))!
	h.type_text('Tiếng Việt')                      // TextEvent path
	h.ime_compose(['t', 'ti', 'tiê', 'tiế'], commit: 'tiếng')  // IME preedit path
	h.press('ctrl-s')
	h.advance(600)                                 // virtual ms: debounced save runs
	assert h.find(role: .listitem, label: 'Tiếng Việt tiếng')!.exists()
	assert h.stats().renders_of('Sidebar') == 1    // caching regression check
	h.snapshot_scene('note_saved')!                // text snapshot, any OS
}
```
- **Queries** use the semantic tree: `by_role`, `by_label`, `by_test_id`, `by_text`, combinable and scoped (`within`). Tests that find elements by role double as accessibility checks.
- **Input simulation** goes through the same dispatch path as real events: mouse move, down, up, wheel, drag, key presses with chords, text, IME, file drops, window resize and DPI changes.
- **Stats** expose per-view render, layout and paint counts, cache hits, draw calls and allocation counts for the last step, so performance regressions fail tests.
- **Scene snapshots** serialize the composed primitives (kind, rounded bounds, color, glyph ids) to a text file under `tests/snapshots/`. They are stable across machines and reviewable in diffs.
- **Pixel snapshots** (`h.snapshot_png`) run only where the offscreen backend exists, and compare exactly on WARP, which M0 showed to be byte-deterministic across compilers, and with a perceptual threshold on hardware backends. Update both kinds with `tools/snapshots.sh --update`.
- Harnesses have no global state, so `v test` runs test files in parallel.

## CI
- Windows and Linux runners execute all headless tests. Windows additionally runs the pixel snapshots, from a non-interactive service account to keep the "no login session" guarantee honest.
- Benchmarks in `bench/` run on every main-branch build. A regression above 10% against the stored baseline fails the job.
- The portability grep from 01-decisions runs on every build.

## Devtools (debug builds, stripped from `-prod`)
- **Inspector** (`ctrl-shift-i`): pick an element to see its bounds, padding and margin overlay, resolved style, owning view, layout rect, semantics and chunk. Also browse the view tree and the model store with each model's `str()`.
- **Profiler overlay** (`ctrl-shift-p`): frame time per phase, renders, layouts and paints per view, cache hit rate, draw calls, instance count, atlas usage, GC allocations per frame.
- The devtools keys yield to the app: a keystroke the app binds (Folio's palette on `ctrl-shift-p`) runs the app's action instead.
- **Gallery HUD** (`apps/gallery/pages/hud.v`, every build): a corner overlay with FPS, the last frame's CPU time, process CPU (share of the machine) and memory (private working set and working set, from `native.process_stats`). It samples every 500 ms, so an idle gallery presents two frames a second; `--measure` turns it off.
- **Paint flashing**: tints every chunk that was repainted this frame, which makes unnecessary invalidation visible immediately.
- **Event trace**: dispatch path, handlers called, actions resolved by the keymap, effects flushed.
- Devtools are built with Quadra's own kit and open in a separate window, which also exercises multi-window support.

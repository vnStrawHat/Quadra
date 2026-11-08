# M0 Spike Results

Date: 2026-11-07. V 0.5.2 (7647ce1), Windows 11 Pro 10.0.26200, MSVC 14.43 (VS 2022 Build Tools), tcc bundled with V, Windows SDK 10.0.22621.0.

## How to run

```sh
tools/v.sh -cc msvc run spikes/m0/v_checks        # V language features
tools/v.sh -cc msvc run spikes/m0/layout_check    # V structs vs SDK headers (MSVC only)
tools/v.sh -cc msvc run spikes/m0/text_check      # DirectWrite through V COM objects
tools/v.sh -cc msvc -o .tmp/window_demo.exe spikes/m0/window_demo && .tmp/window_demo.exe          # interactive
tools/v.sh -cc msvc -o .tmp/headless_png.exe spikes/m0/headless_png && .tmp/headless_png.exe "C:\...\out.png" 1.5
spikes/m0/session0_check.sh                       # elevated Git Bash only
tools/v.sh -cc msvc test quadra                     # unit tests
```
`tools/v.sh` keeps every cache and temp file inside the project (`.cache/`, `.tmp/`).

## Results

| # | Check | Result | Consequence for the specs |
|---|---|---|---|
| 1 | Type-erased listeners: typed handlers in one list, called from non-generic code | PASS | 06/07 listener design stands |
| 2 | Generic method with extra type params on a generic receiver (`ModelCx[T].subscribe[M, E]`) | PASS | 07 API stands |
| 3 | `Renderable[T]` generic interface, and erasure to `AnyView` via generic fn pointers | PASS | 07 stands |
| 4 | `voidptr` to fn-type cast and vtable-slot calls | PASS | D9 stands |
| 5 | `malloc_noscan` under the default GC | PASS | 12 stands |
| 6 | 11 D3D11/DXGI struct layouts equal the SDK headers | PASS | bindings need no C headers |
| 7 | Win32 window, per-monitor DPI v2, message loop on `MsgWaitForMultipleObjectsEx` + frame-latency waitable | PASS at 100% | 02 stands |
| 8 | Flip-model swap chain, `SetMaximumFrameLatency(1)`, precompiled SM 5.0 shaders, debug layer | PASS on hardware; the debug layer was created, but its messages go to an attached debugger and were not captured (read them through `ID3D11InfoQueue` in M1) | 03 stands; feature level 11_0 is the minimum |
| 9 | Rounded quads with borders (SDF), instanced from a structured buffer | PASS, see `evidence/` | 03 stands |
| 10 | **Implementing COM interfaces in V** (`IDWriteTextAnalysisSource`/`Sink`) | PASS: script, bidi and line-break callbacks delivered | D9 extended: both directions of COM work |
| 11 | DirectWrite shaping of NFC Vietnamese | PASS: 17 glyphs for 17 units | 04 stands |
| 12 | DirectWrite shaping of NFD Vietnamese (e + U+0302 + U+0323) | PASS: composed to the precomposed glyph, same width as NFC | NFC remains for buffer semantics, not for rendering |
| 13 | Grayscale glyph coverage | PASS with `ALIASED_1x1`; `CLEARTYPE_3x1` returns empty bounds in GRAYSCALE mode | 04 records the texture type |
| 14 | `WM_CHAR` text including a surrogate pair (emoji) | PASS (posted to the window itself) | 02 stands |
| 15 | WARP offscreen render + DirectWrite text to PNG, no window | PASS in session 1 | 11 stands |
| 16 | tcc builds of every spike | PASS after loading `ProcessIdToSessionId` dynamically | D9: also load newer *kernel32* entry points at runtime |
| 17 | WARP output determinism | PASS: tcc and MSVC builds produce byte-identical PNGs | pixel snapshots can compare exactly on WARP |
| 18 | Frame CPU cost (debug build, shaping every frame, no caching) | ~0.2 ms per frame, ~2 ms for the first frame (glyph rasterization) | 12 budgets look reachable |

## Pending (need the user)

| Check | Why pending | How to run |
|---|---|---|
| Session-0 render as SYSTEM | the agent shell is not elevated | elevated Git Bash: `spikes/m0/session0_check.sh` |
| IME composition (Windows Vietnamese Telex, and Unikey/EVKey) | needs a human typing | run `.tmp/window_demo.exe`, type; the console logs `ime start/preedit/commit/end` and `text ...` |
| Real 150% DPI window | display scale is 100% here; 150% was verified offscreen only | set display scale to 150% or drag the window to a 150% monitor |

## Decisions taken during M0
- Per-OS implementations are separate modules (`quadra.platform.win`, `quadra.gpu.d3d11`, `quadra.text.dwrite`) importing the OS-independent module (`quadra.platform`), because V forbids import cycles. Selection of the default backend happens in a small `native` module per layer (M1).
- COM vtable slots are generated from the SDK headers by `tools/com_slots.sh` and cross-checked against the C `Vtbl` structs where the SDK ships them.
- Build helpers are bash scripts (`tools/*.sh`), not `.vsh`.

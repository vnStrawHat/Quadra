# 01 — Architecture Decisions

Each entry: choice, rationale, cost of reversing.

## D1. State: models with automatic dependency tracking
- **Choice**: all state lives in models (`Model[T]`) owned by the app. A view is a model that can render. When a view reads a model during `render()`, the dependency is recorded automatically. When a model is updated, exactly the views that read it are invalidated. Explicit `observe` and `subscribe` exist for side effects (07-state).
- **Rationale**: manual "observe this model so I re-render" wiring is a classic source of stale UI. Tracking reads makes the correct behavior the default and gives the framework precise invalidation for free.
- **Reversal cost**: high. Locked in M1.

## D2. Lifetime: ownership tree, handles are plain values
- **Choice**: every model has an owner (the app, a window, or another model). Releasing an owner releases everything it owns, plus their subscriptions and tasks. A handle is `(id, generation)`; using a stale handle is detected, never a dangling pointer.
- **Rationale**: V has no destructors or RAII to drive reference counting. An explicit tree gives deterministic cleanup, and value handles are cheap to copy and store.

## D3. Three invalidation levels and automatic view caching
- **Choice**: changes are classified as **render** (rebuild a view's elements), **layout** (recompute sizes), or **paint** (re-emit primitives). Every view is a cache boundary: a clean view reuses its elements, layout and recorded primitives. Hover, focus rings, caret blink, scrolling and paint-only animations never re-render anything (07-state).
- **Rationale**: in real apps most frames are a hover, a scroll or a keystroke. Cost must scale with the change, not with the window.
- **Reversal cost**: medium. Contained in `core`, invisible to app code.

## D4. Windowing: own Win32 layer
- **Choice**: windows, input, IME and OS services are written directly on Win32.
- **Rationale**: Vietnamese typing needs full IME composition control. Per-monitor DPI v2, the modal resize loop, Windows 11 snap layouts on a custom titlebar, tray icons and native dialogs all need direct access.
- **Reversal cost**: low, behind `Platform` / `PlatformWindow`.

## D5. GPU: D3D11 first, behind `Renderer`
- **Choice**: D3D11 with a flip-model swap chain and a frame-latency waitable object. Shaders are precompiled to `.cso` by `tools/build_shaders.sh` and embedded, so startup does no shader compilation. Later backends: Metal (macOS) and Vulkan (Linux).
- **Rationale**: the most stable Windows API, a WARP fallback for VMs, and the lowest input latency when paired with the waitable object. The renderer only draws instanced primitives, so each backend stays small.

## D6. Text: native shaping and rasterization behind `TextBackend`
- **Choice**: DirectWrite on Windows for font enumeration, fallback, itemization, shaping and rasterization. Later: CoreText on macOS, FreeType + HarfBuzz + fontconfig on Linux. Line layout, wrapping, caching and caret logic are shared pure V code (04-text).
- **Rationale**: system-quality text, correct fallback for any script, and color emoji, matching what users see in other native apps.

## D7. Layout: own engine with flex and grid
- **Choice**: flexbox and CSS-grid subsets in pure V, on a persistent node arena with per-node measurement caches and relayout boundaries (05-layout).
- **Rationale**: grids are the natural tool for forms, dashboards and galleries. Owning the engine lets layout caching integrate with view caching.

## D8. Primitive ordering by overlap, not by tree depth
- **Choice**: each primitive is assigned the lowest draw layer above everything it overlaps that was painted before it. Primitives in the same layer and of the same kind are drawn together (03-render).
- **Rationale**: painting a list of 1,000 buttons in tree order alternates quad, text, quad, text and would produce thousands of draw calls. Overlap-based layers produce a handful of draw calls without changing the visible result.

## D9. OS interop: dynamic loading and COM by vtable index
- **Choice**: DLL entry points outside the oldest `user32`/`kernel32` set (`D3D11CreateDevice`, `DWriteCreateFactory`, `Imm*`, DPI APIs, newer kernel32 functions such as `ProcessIdToSessionId`) are loaded with `LoadLibraryExW` restricted to System32, plus `GetProcAddress`. COM methods are called through a helper that reads the vtable slot by index, and COM interfaces DirectWrite calls back into (`IDWriteTextAnalysisSource`/`Sink`) are implemented in V with a heap vtable of V functions. Slots are generated from the SDK headers by `tools/com_slots.sh`. No C++ shim and no import libraries. Verified in M0 (spikes/m0/RESULTS.md).
- **Rationale**: builds with any C compiler V supports, including tcc for fast debug builds. Each COM method is declared once with its slot index, which keeps the binding auditable.

## D10. Single UI thread, structured background tasks
- **Choice**: models, views and dispatch live on the main thread. Background work runs through `cx.spawn`, is owned by the spawning model, and is cancelled when that model is released. Results come back through the main-thread queue (10-app-services).
- **Rationale**: no locks in UI code, and no orphaned work writing into released state.

## D11. Accessibility data from day one
- **Choice**: elements carry semantics (role, name, value, state, actions) collected during prepaint into a semantic tree. Providers for UIA/AT-SPI/NSAccessibility are added later on top of that tree.
- **Rationale**: retrofitting semantics means touching every component. Collecting them now costs little and keeps the provider work isolated.

## D12. Headless implementations are part of the framework
- **Choice**: every OS boundary ships a headless implementation: an in-memory platform with a virtual clock, a null renderer, and a fake text backend, all pure V. On Windows, an offscreen WARP renderer and DirectWrite with bundled fonts give pixel output without a desktop.
- **Rationale**: tests must run in CI, over SSH and under service accounts with no login session. Building the harness on the same interfaces as the real backends keeps those interfaces honest and makes the first port cheaper.
- **Reversal cost**: none; it only adds implementations.

## Portability check
Porting replaces only `platform/<os>/`, `gpu/<backend>/` and `text/<backend>/`. CI fails if any other module calls OS APIs.

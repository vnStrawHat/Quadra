# 00 — Overview

## Goals
- A native desktop UI framework in V that draws everything on the GPU. No OS controls, no webview.
- **Performance first**: zero CPU when idle, work proportional to what changed, input-to-photon within one frame (see 12-performance).
- Declarative API: views own state, `render()` returns an element tree, the framework handles layout, painting, input and caching.
- Fit for real applications, not only demos: multi-window, persistence, async work, accessibility data, testing and devtools (see 10, 11).
- **Headless from day one**: every test runs without a window, a GPU driver or an interactive login session (11-devtools-testing).
- A base component kit with themes, shown in a Gallery app and proven in a real sample app.
- Windows 10 (1809+) and 11 first. Linux and macOS are added by implementing the OS boundaries only.

## Out of scope for v1
- RTL/bidi editing, web target, hot reload, multi-threaded rendering.
- Accessibility providers for Linux and macOS (AT-SPI, NSAccessibility). UI Automation on Windows is built (06-core); the others read the same tree.
- Dock layout, data grid with editing, code editor (roadmap).

Vietnamese must render and be typeable through the system IME in v1.

## Layers

```
  apps/gallery  apps/notes      showcase + real sample app
  ─────────────────────────────────────────────────────────
  quadra.kit                      components + theme
  quadra.ui                       primitive elements + style
  quadra.app                      app services: settings, assets, i18n, tasks, tray, persistence
  quadra.core                     app loop, models, views, invalidation, caching, dispatch, focus
  quadra.layout                   flex + grid (pure V)
  ─────────────────────────────────────────────────────────
  quadra.platform                 windowing, input, OS services     <- OS boundary
  quadra.gpu                      renderer backends                 <- OS boundary
  quadra.text                     shaping + rasterization backends  <- OS boundary
```

Dependency rule: a layer imports only layers below it. OS-specific code exists only in `platform`, `gpu` and the backend files of `text`. CI greps every other module for `C.` calls into OS APIs.

## Directory layout

```
quadra/
  platform/  platform.v event.v keys.v cursor.v services.v
             win/  window.c.v  wndproc.c.v  ime.c.v  dpi.c.v  clipboard.c.v  dialogs.c.v
                   tray.c.v  notify.c.v  system_theme.c.v  com.c.v  dll.c.v
             headless/  (in-memory, pure V, used by tests)   linux/  macos/  (stubs until ported)
  gpu/       renderer.v scene.v primitives.v atlas.v order.v
             d3d11/  device.c.v  pipeline.c.v  buffers.c.v  offscreen.c.v  shaders/*.hlsl  shaders/*.cso
             null/   (records the scene, used by tests)
  text/      text_system.v backend.v font_db.v word_cache.v line_layout.v wrap.v nfc.v grapheme.v
             dwrite/  factory.c.v  fonts.c.v  shape.c.v  raster.c.v
             fake/    (deterministic metrics, used by tests)
  layout/    style.v tree.v flex.v grid.v measure.v cache.v
  core/      app.v window.v model.v effects.v subscription.v view.v view_cache.v
             invalidation.v element.v frame.v dispatch.v hitbox.v focus.v keymap.v
             element_state.v overlay.v semantics.v animation.v
  app/       settings.v assets.v i18n.v tasks.v window_state.v logging.v single_instance.v
  ui/        div.v styled.v style_values.v text.v rich_text.v icon.v img.v scroll.v list.v canvas.v color.v
  kit/       theme.v tokens.v button.v ... input/ select/ table/
  testing/   harness.v query.v simulate.v snapshot.v
  devtools/  inspector.v profiler.v
apps/gallery/  apps/notes/  bench/  tools/  (v.sh, build_shaders.sh, com_slots.sh)
```

Each OS-independent layer (`platform`, `gpu`, `text`) holds only interfaces and shared types. Every implementation is its own sub-module (`platform/win`, `platform/headless`, `gpu/d3d11`, `gpu/null`, `text/dwrite`, `text/fake`) that imports its parent, because V forbids import cycles. A small `native` sub-module per layer picks the default implementation for the current OS through V's file suffixes (`_windows.c.v`, ...). No scattered `$if windows` blocks.

## Code rules
- **One file ≤ 300 lines**, aiming for about 150. Split by responsibility, never mechanically.
- One component per file in `kit/`. Large components get a folder.
- Interfaces only at OS boundaries (`Platform`, `PlatformWindow`, `Renderer`, `TextBackend`) and where users plug in (`Renderable`, `Element`).
- Hot paths do not allocate per frame: buffers are reused, cleared, and keep their capacity (12-performance).
- GPU-free logic has `*_test.v` tests. Performance-sensitive code has a benchmark in `bench/`.
- Docs and code comments are in English.
- Toolchain: V 0.5.2 with the closure patch (`tools/v.sh`). Release builds use `-cc clang -prod` with the portable clang (llvm-mingw) that `tools/setup.sh` puts in `.tools/llvm-mingw`, so no system C compiler is needed and apps do not need the VC++ runtime. MSVC also works when installed. tcc is fine for quick debug builds because no import libraries are needed (01-decisions D9).

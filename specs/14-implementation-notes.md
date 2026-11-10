# 14 — Implementation Notes

Where the implementation deliberately differs from specs 00–13, and why. Each entry names the spec section it amends.

## Rendering (03, D8)
- **Uber-instance.** Every primitive kind shares one 144-byte instance layout and one shader. A scene draws in paint order with one draw call per atlas-page combination, so ordering is exact without layering. Overlap layers (D8) only run when a scene spans several atlas pages, which is rare.
- **No GPU `Path` primitive.** Canvas paths are rasterized on the CPU by `quadra.vector` into atlas tiles, cached by geometry and device size. Charts that change every frame are the exception; a GPU path pipeline can be added behind the same `Painter` API.

## Images and icons (03, D6)
- **Icons are SVG**, rasterized by the pure-V `quadra.vector` rasterizer at the exact device size, not an icon font. The same code renders SVG files, works headless on every OS, and keeps strokes crisp at any DPI.
- **Raster images are decoded with V's `stbi`** (PNG, JPEG, BMP, GIF) on a worker thread, not WIC. Portable and headless-friendly; WIC can be added for WebP/ICO later.

## Layout (05)
- **The layout tree is rebuilt every build**, from cached element trees. Clean views skip rendering, text measurement hits the text caches, and layout of a cached subtree is cheap. Persistent incremental layout with relayout boundaries is deferred until profiling shows layout dominating. (Profiled 2026-11-07: typing in the 49-page gallery spent 75 % of layout in text-cache hits; the flat cache fixed that, and the flex pass itself is about 0.1 ms.)
- **`min-width/height` default to 0**, except children of a scroll container on its main axis, which never shrink (CSS's `min-height: auto` effect where it matters most).
- `compute_fit` lays out content-sized roots (overlays); `compute` fills the available size (window roots).
- **App shells need no `min-height: 0`.** Three rules replace CSS's automatic minimum sizes:
  - Auto height is the content height. Only width, and the height of a container that clips y, is limited by the available space.
  - On overflow, items that grow give space back first. Fixed-size siblings, such as a header above a scrolling body, shrink only if that is not enough.
  - A container that scrolls on its cross axis stretches items to at least the viewport, never below their content.

## State and contexts (06, 07)
- **One generic context, `ViewCx[T]`**, serves render, event handlers, updates and timers. It embeds the base `Cx`. The phase contexts for elements stay separate (`LayoutCx`, `PrepaintCx`, `PaintCx`).
- **`emit[E]` lives on `Cx`**, and holders for observe/subscribe are built in free generic functions, to avoid a V generics bug (docs/v-notes.md).
- **View `render` methods must be `pub`**; views declared in `_test.v` files need `@[markused]`.
- **Timers are discrete events**: a timer callback marks its owner changed, like a click.
- **Invalidation levels** implemented: render (view changed), prepaint (scroll, focus), paint (hover, press, devtools). The compose level is covered by prepaint for now.

- **`observe` and `spawn_task` are free functions** (`core.observe(mut cx, m, f)`, `core.spawn_task[T, R](mut cx, work, done)`). As methods of `ViewCx[T]` they made V instantiate the other generic methods with their type arguments.

## Elements (08)
- State styles are `ui.Patch` values (`hover`, `active`, `focus_ring`) applied at paint time, never re-rendering.
- Divs that clip skip children outside the visible area in prepaint and paint, so long scrolled lists cost only their visible rows. Culled elements register no hitboxes or test ids.
- Overlays (`ui.anchored`) are laid out as separate roots and drawn above everything; clicks on an overlay do not reach the content under it.

## Devtools (11)
- The profiler, paint flashing and inspector are an in-window overlay toggled with `ctrl-shift-p`, `ctrl-shift-f` and `ctrl-shift-i`, rather than a separate window.

## App services (10) and Notes (13)
- **Tasks run one OS thread each** (`core.spawn_task`); a fixed pool is deferred until apps spawn many at once. `debounce` ships; `throttle`, `timeout` and `retry` are deferred.
- **Directories come from V's `os.config_dir()` and `os.cache_dir()`**, so `quadra.app` has no OS code. `AppInfo.root` (Notes: `--data`) keeps everything in one folder.
- **Atomic file replace is `platform.replace_file`** (MoveFileExW on Windows), because MSVCRT `rename` fails on an existing target.
- **With nothing focused, actions go to every handler, topmost first.** App shortcuts work before anything has focus. Key-down events still need focus.
- **`ui.text(s).selectable()` selects within one text element**: drag, shift-click, double-click (word), triple-click (all), ctrl-a, ctrl-c. The selection shows while the text has focus, so clicking elsewhere clears it. Selecting across elements is deferred; editors, terminals and logs bring their own selection (specs/16).
- **Files dropped on a window go to window-level handlers** (`cx.on_files_dropped`). Drop targets per element are deferred.
- **Kit components take their few words from `kit.Strings`** (English default, `kit.strings_vi()`). Apps switch it with the locale.
- **Deferred from M5**:
  - Tray icon and quick note (optional in the spec).
  - Dragging notes between folders. The row context menu has "Move to …" instead.
  - Fonts loaded from assets.
  - `ui.img(url)` with a disk cache.
  - Per-component `persist_key`. Notes persists its split ratio through its own settings instead.
  - Keymap overrides in settings.
  - Log stream in the devtools console.
  - Custom titlebar (M6).

## Hardening (M6)
- **Paint patches instead of paint chunks.**
  - During a full paint, every hover- or press-styled Div records its instance range and paint state.
  - A frame that only restyles such elements paints just them again and overwrites the range in place.
  - A changed primitive count, kind or atlas page falls back to a full paint, and so does a reordered scene.
  - Hover-styled Divs always emit their background quad, so the count stays stable.
  - The scene tracks a dirty range per revision. The D3D11 instance buffer (now USAGE_DEFAULT) and the null device upload only that range.
- **Layout**:
  - Four measure-cache entries per node.
  - Cache keys ignore the available size on an axis the parent fixed.
  - The only growing item of a container with a definite main size is not measured for its base size, because it always takes the leftover space.
- **Text caches are keyed by numbers first** (face id and float bits), then by the text. Float formatting in cache keys was the top cost of a frame. Face lookups are memoized.
- **Parsed SVG documents are cached by content hash** (`app.images.sources`). Icons rebuilt every frame no longer parse their source.
- **Startup**:
  - The D3D11 device and pipeline are created on a worker thread (`d3d11.new_backend_async`).
  - The first frames draw on WARP until the hardware device is up (`upgrade_windows.c.v`, specs/03).
  - Window surfaces are attached at the first frame, after the first layout.
- **Custom titlebar**:
  - `WindowOptions.custom_titlebar`, `Div.window_region(.caption | .minimize | .maximize | .close)` and `kit.titlebar`.
  - On Win32, WM_NCCALCSIZE keeps the resize borders, and WM_NCHITTEST asks `EventSink.hit_test`. HTMAXBUTTON gives Windows 11 snap layouts.
  - The window buttons act on non-client button up. Their click handlers serve headless tests.
- **CI gate** `tools/check.sh`:
  - Formatting.
  - Portability: OS files and backend imports only in backends; no `C.` calls in core, ui, kit, app or layout.
  - No PowerShell.
  - The 300-line rule.
  - Tests.

## Memory: closure lifetimes

V frees no closure by itself, so Quadra assigns every closure made in a frame an owner:

| Made during | Owner | Freed |
|---|---|---|
| A kit component's build (`Lazy.build_tree`, runs at every layout) | the window's layout lifetime | after the next build |
| A list row or overlay built in prepaint | the window's prepaint lifetime | after the next prepaint |
| A view's `render` | the view | when the view renders again, at the end of that build; when it is released, after its window rebuilds |
| Handlers, constructors, startup | nobody | never (keep them out of per-event paths) |

**Rule for app code:** a closure made during `render` belongs to that render. Make
long-lived closures in constructors or handlers. When a render creates something long-lived
(`new_model`, `new_view`, a global, an observer, a subscription, a timer, a task, a debounce,
an action or key binding), the framework pins that render and never frees it. Creating a
model or updating a global from a handler pins the current render of every view, because what
gets stored may capture them (popups, dialogs). Opening a popup or a modal also keeps the window's latest layout and
prepaint lifetimes (`cx.keep_build`), since an overlay opened from a kit component or a list
row holds closures of that build (a tab strip's overflow menu items); they are freed once the
window has no overlay left (`app.free_kept`). Kit builders (`core.Builder`) must only build
elements.

Engine hot paths (notifications, timers, paint animation ticks, hover delays through
`request_prepaint_after`, SVG, stroke and canvas paint on a hit or a miss via `image_put`,
kit element handlers as holder + function, focus order) use no closures. A closure made in
layout, prepaint or paint outside a Lazy build has no owner: `gallery_test.v` revisits the
pages that used to leak one and checks the live count stays put. Prepaint layouts (virtual list rows, overlays) are
scratch: each prepaint truncates the layout tree back to the nodes of the last build.
`quadra.core.life` wraps `closure.Lifetime`, because vfmt 0.5.2 drops `import builtin.closure`
from `quadra.core`'s files.

Posting to the main thread is a per-event path too: `core.Inbox` builds its drain closure
once and posts that same closure every time (bench/term checks closures stay flat while
output streams).

Live sources that change a model every tick do not call `cx.update` (it takes a closure):
they mutate through `cx.peek(model)` and then call `cx.touch(model)`, which marks it
changed and flushes. Cell functions for live tables are made once and read the rows through
a holder pointer.

### Avoiding closures (policy)

Lifetimes keep closures from leaking, but they are not free: creating one costs 110–200 ns
(a global lock, a global map entry, a lifetime record), and freeing one 0.3–3 µs, growing
with the number of closures alive (measured: a lifetime pass of 300 closures costs 0.35 ms).
Typing in the gallery freed 86 closures a keystroke, about 5 % of the frame. A capture-less
`fn (...) {...}` is a plain function pointer and costs nothing. So Quadra code avoids `fn [..]`,
in this order of preference:

| Method | Where | How |
|---|---|---|
| 1. State in the view | app and kit views | listeners receive `mut v T`; keep what the callback needs in the view |
| 2. Payload in the event | component callbacks | the component passes the value (`on_select` gets the id, a menu reports the picked index) |
| 3. Data in a typed holder | handlers, themed builders, list rows, popups | `core.handler_with(data, f)`, `core.listener_with(id, data, f)`, `core.bind_fn` / `bind_fn_ref`, `kit.themed_with`, `ui.uniform_list_with`, `kit.open_popup_with`: f captures nothing; the data (a small struct when there are several values) rides in a generic holder. The one `unsafe` cast back lives in `quadra/core` (listener.v, bound.v), in a generic `call_*` made for the same types. The data type must not be a pointer (a V 0.5.2 bug, bound.v) |
| 4. Interface objects | kit values that store an action | `kit.MenuItem.action` (`kit.MenuAction`: `kit.PickIndex`, a tab strip's `SelectTab`), the month grid's `CalendarTarget`: fields are the data, the method the behaviour; the GC frees them |

Model updates from handlers use `cx.peek` plus `cx.touch` (or plain mutation without
`touch` where `update_silent` was used) instead of `cx.update*` closures. Closures that
remain are made once (window and app setup, the inbox drain, lifetime work functions) or
belong to app code in `render`, where lifetimes free them. `tools/check.sh` caps the number
of `fn [` in `quadra/` (23 left of 117) so new ones need a reason. The closure-taking APIs stay
for app code (`open_popup`, `uniform_list`, `MenuItem.on_select`, `DragSource.preview`, a
ready `preview_el` being the closure-free form).

Measured after the change (bench, closures freed per operation): typing in the gallery 86 → 4,
a full relayout on resize 82 → 0, scrolling the million-row table 14 → 0. Typing went from
about 750 to 675 µs a keystroke, the table scroll from 458 to 428 µs.

## Performance: GC pins around calls

Under `-prod` with `boehm_full_opt`, V emits a deep keep-alive walk around every call
statement for each local in scope that is a struct or an array of pointerful structs: the
walk visits every nested pointer. A loop that makes calls while a large such array is in
scope therefore costs O(n²). Terminal reflow with 10 000 history lines took 200–300 ms per
resize because of it, and 3.6 ms once the history pass moved into a function of its own
that assigns its result once. Rule for hot loops: keep big arrays of structs out of scope
(their own function, or a field reached through a pointer), and prefer bulk assignment over
a call per element.

## Performance: text that changes every frame, and element size
Found with the comparison scenes (docs/benchmarks.md). A dashboard of 600 changing numbers
spent 5 ms a frame in layout, almost all of it shaping new strings, and built 4 MB of garbage.
- **Shaping runs script analysis only** (bidi and line breaks were computed and unused), and
  **printable ASCII skips it**: one run whose analysis is known (Latin when it has a letter,
  Common otherwise) and no font fallback. Buffers are reused between calls. 4.7 → 2.8 µs a string.
  `tests/shaping_test.v` checks both paths give the same glyphs.
- **Numbers are composed, not shaped** (`text/numeric.v`): once per face, every pair of
  `0-9 . , - + % : /` is shaped and compared with its two single glyphs; if no pair kerns
  or ligates, a number is laid out from glyphs shaped once per size, and is not cached.
- **Wrapping allocates little**: no token arrays, no substring copies (`substr_unsafe`
  views; caches copy what they keep), flat line ranges, and a one-line text keeps its layout
  only in the wrap cache.
- **A text element keeps its last wrap.** Wrapping at any width from the widest line up to
  the width it was made for gives the same lines (up without limit when no line broke for
  width, `WrappedText.width_breaks`), so the second measure and paint reuse it.
- **Text lines outside the clip are not painted** (a long document in a scroll view).
- **Elements are small**, because every render allocates them: `Div` 1944 → 912 bytes and
  `TextEl` 752 → 176 bytes (`Div` 704 since the memory work below). Optional fields cost about 40 bytes each in V, so `TextPatch` is
  plain fields with a bit set, the hover/active/focus patches sit behind one pointer that is
  nil for most divs, and text leaves share one default layout style.

## Memory: what the collector costs and how Quadra keeps it low
Collection time follows the bytes allocated a frame and the size of the live heap. The
rules that follow from it (measured in docs/benchmarks.md, "Memory work"):
- **Share instead of copying.** V slices and `substr_unsafe` share memory, and the
  collector (all interior pointers) keeps the whole array or string alive while a view
  exists. Wrapped lines are views into their paragraph's layout (`LineLayout.cut`, with
  `origin` and `base` so x and byte offsets read as the line's own); a rewrap allocates
  the views only.
- **Scratch owned by a long-lived object** for results used before the next call: shaping
  buffers, the composed number (`TextSystem.num`), line layout's byte table. Only
  pointer-free arrays, so no GC pin (the section above).
- **Rare parts behind a pointer**, nil by default: a div's state patches and drag source,
  a style's grid part. Every element and layout node pays for inline fields, every frame.
- **Small enums** (`enum X as u8`) in structs copied per node.
- **Elements from render arenas** (specs/07, Element memory): a view's render reuses the
  divs and texts of its render before last, reset at the end of the build that replaced
  them, on the closure lifetimes' schedule and pins. An arena text element showing a number
  lays it out into a wrap it owns (`TextSystem.wrap_number`), kept with the element, and
  does not cache it; elements made elsewhere (list rows in prepaint) keep using the cache,
  where an unchanged number is found again. Layout keeps its flex items and lines in scratch per nesting depth.
- **No hidden allocations per character or element**: UTF-8 decoding in grapheme stepping
  allocated a substring and a rune array per character; prepaint ids were two interpolated
  strings per element; `'${v:.2f}'` costs about 220 bytes (a string builder), `ui.number`
  16.
- **The heap grows by a third between collections** (`AppOptions.gc_divisor` 3, Boehm's
  default here was 1): about a third less peak memory for a few percent CPU. Incremental
  collection measured worse on every scene and stays off.

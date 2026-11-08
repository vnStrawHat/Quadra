# 10 — App Services

Module `quadra.app`. What a real application needs beyond widgets. Built on `core` and `platform.Services`; no OS code here.

## Settings and persistence

```v
pub struct Settings[T] { /* model wrapper */ }
pub fn load_settings[T](mut cx core.Cx, name string, default T) !Settings[T]  // <config_dir>/<app id>/<name>.json
```
- Settings are a model, so views that read them update automatically and `observe` can react.
- Writes are debounced (500 ms) and atomic: write to a temp file, then rename. A corrupt file is moved aside and defaults are used, with a warning in the log.
- A `version` field plus an optional migrate function handles schema changes.
- **Window state** (`window_state.v`): position, size, maximized and monitor are saved per window key and restored, clamped to the current monitor layout.
- **UI state**: split ratios, column widths, sidebar collapse and selected tabs persist through the same mechanism when a component is given a `persist_key`.

## Assets
- `assets.v` registers embedded files: `app.assets.add('icons/logo.svg', $embed_file('assets/logo.svg'))`. Assets are referenced by path everywhere (`ui.svg('icons/logo.svg')`).
- Fonts registered as assets are loaded into the text backend at startup, before the first frame, and become available by family name.
- Debug builds can read assets from disk instead of the embedded copy, so editing an asset needs no rebuild.

## Internationalization
- String tables in JSON per locale: `i18n/vi.json`, `i18n/en.json`. `t(cx, 'files.count', n: 3)` with plural rules and named arguments.
- Locale formatting for numbers, dates, times, relative times and file sizes. Data for `vi` and `en` ships with v1; more locales are data files only.
- The current locale is a global model, so switching language re-renders exactly the views that read strings.
- Missing keys are logged once and fall back to the default locale, then the key itself.

## Background tasks

```v
// cx is ModelCx[Doc]; only the result type is passed
cx.spawn[string](fn (tc &core.TaskCx) !string { return os.read_file(path) },
	fn (mut d Doc, r core.TaskResult[string], mut cx core.ModelCx[Doc]) {
		if r.ok() { d.text = r.value } else { d.error = r.err.msg() }
	})
```
- The work function runs on a worker pool (size = cores - 1). The completion function runs on the main thread with the owner's state.
- A task is owned by the model that spawned it. Releasing the model cancels the task: the cancel flag is visible to the work function through `tc.cancelled()` and its completion is never delivered.
- Helpers: `debounce(ms)`, `throttle(ms)`, `timeout(ms)`, `retry(n, backoff)`.
- HTTP uses V's `net.http` inside tasks. `ui.img(url)` uses the same pool with an on-disk cache in `cache_dir`.

## Navigation
`Router` is a model holding a stack of typed routes, with `push`, `replace`, `back`, and `forward`. The root view renders the current route. Back and forward mouse buttons and `alt-left` / `alt-right` are bound by default.

## Application lifecycle
- **Single instance** (opt-in): a second launch forwards its arguments to the first instance and exits. The first instance receives them as an event.
- **Command line**: parsed arguments are available in `AppOptions` before the first window opens.
- **Close flow**: `CloseRequested` goes to the root view first, so an app can ask "save changes?" through `confirm` and cancel the close.
- **Tray apps**: `quit_on_last_window: false` plus `platform.Services.add_tray`.

## Logging and crashes
- `log.v`: leveled, structured logging to `data_dir/logs/` with daily rotation and a size cap. The devtools console shows the same stream.
- Errors returned from listeners and tasks are logged with the view and event that produced them, and the app keeps running.
- Native crashes: an unhandled-exception filter on Windows writes a minidump and the last log lines to `data_dir/crashes/`, which the app can offer to send on next start.

## Packaging (Windows)
`tools/package.sh` produces a release build with an embedded manifest (DPI awareness, UTF-8 code page, Common Controls v6 for native dialogs), version info, and the app icon. An MSIX package and an auto-update channel are on the roadmap.

## Tests
`settings_test.v` (atomic write, corruption recovery, migration), `i18n_test.v` (plurals, Vietnamese formatting), `tasks_test.v` (cancellation on release, completion ordering) on the headless platform with a virtual clock.

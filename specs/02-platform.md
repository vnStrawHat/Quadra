# 02 — Platform

Module `quadra.platform`. Only this module, `gpu` backends and `text` backends call OS APIs.

## Core interfaces

```v
pub interface Platform {
mut:
	run(mut sink EventSink)            // main loop, blocks until quit
	quit()
	open_window(opts WindowOptions) !&PlatformWindow
	post(task fn ())                   // any thread -> main thread, wakes the loop
	set_timer(ms int, repeat bool, cb fn ()) TimerId
	cancel_timer(id TimerId)
	services() &Services
}

pub interface EventSink {             // implemented by core.App, avoids closure capture
mut:
	on_event(win WindowId, e Event)
	on_frame(win WindowId)             // the window may present a frame now
}

pub interface PlatformWindow {
	id WindowId
mut:
	size() Size                        // logical px
	scale() f32                        // 1.0 = 96 DPI
	position() Point
	set_bounds(r Rect)
	state() WindowState                // .normal | .minimized | .maximized | .fullscreen
	set_state(s WindowState)
	monitor() Display
	set_title(s string)
	set_cursor(c Cursor)
	request_frame()                    // coalesced; delivered as on_frame at the next vsync slot
	set_ime_rect(r Rect)
	enable_ime(on bool)
	set_titlebar_regions(regions []TitlebarRegion) // drag, minimize, maximize, close
	native_handle() voidptr            // used only by gpu backends
	close()
}
```

`WindowOptions`: title, size, min_size, position (or `centered` on the primary display, the
size first capped per side at `fit` of that display's logical size, 0 for no cap), resizable, `titlebar: .system | .custom`, kind (`.normal | .popup | .tool`), parent, transparent.

## Services (one struct, filled per OS)

| Service | API | Windows impl |
|---|---|---|
| Clipboard | `read_text` `write_text` `read_image` `write_image` `read_files` | `OpenClipboard`, `CF_UNICODETEXT`, `CF_DIBV5`, `CF_HDROP` |
| Dialogs | `open_files` `save_file` `pick_folder` `message_box` | `IFileOpenDialog` / `IFileSaveDialog`, `TaskDialogIndirect` |
| Shell | `open_url` `reveal_in_folder` `open_path` | `ShellExecuteW`, `SHOpenFolderAndSelectItems` |
| System theme | `dark_mode()` `accent_color()` `high_contrast()` `text_scale()` `on_change` | registry `AppsUseLightTheme`, `DwmGetColorizationColor`, `WM_SETTINGCHANGE` |
| Tray | `add_tray(icon, tooltip, menu)` `remove_tray` | `Shell_NotifyIconW` |
| Notifications | `notify(title, body, actions)` | Toast via tray balloon in v1; WinRT toasts post-v1 |
| Single instance | `acquire(app_id) bool` `forward_args(args)` `on_args` | named mutex + named pipe |
| Paths | `config_dir` `data_dir` `cache_dir` `temp_dir` | `SHGetKnownFolderPath` (`%APPDATA%`, `%LOCALAPPDATA%`) |
| Power | `on_suspend` `on_resume` `on_battery()` | `WM_POWERBROADCAST`, `GetSystemPowerStatus` |
| Images | `decode_image(bytes)` `render_svg(bytes, size)` (thread-safe) | WIC, Direct2D SVG document |

## Events

```v
pub type Event = MouseEvent | WheelEvent | KeyEvent | TextEvent | ImeEvent | ResizeEvent
	| ScaleEvent | FocusEvent | VisibilityEvent | CloseRequested | FileDropEvent | DragOverEvent
```
- Coordinates are logical px. Wheel events carry `precise bool` to tell touchpad from mouse wheel.
- `KeyEvent { key Key, code u32, mods Mods, repeat bool, down bool }` with an OS-independent `Key`.
- `TextEvent` carries committed text (UTF-16 surrogates joined to UTF-8).
- `ImeEvent { kind: .start | .preedit | .commit | .end, text string, cursor int }`.
- `VisibilityEvent` fires on minimize/occlusion. Core stops producing frames while hidden.
- Mouse moves are coalesced: at most one `MouseEvent.move` per window per frame reaches core.

## Windows implementation notes
- Process: per-monitor DPI v2, `OleInitialize`, UTF-8 active code page and DPI awareness set in the embedded manifest.
- Loop: `MsgWaitForMultipleObjectsEx` waits on messages **and** the swap chain's frame-latency waitable, so a frame starts as soon as the GPU can accept it. Idle means fully blocked.
- Modal resize/move loop (live resize): the OS loop does not return to `run()`, so frames are produced from `WM_SIZE`, after the swap chain took the new size, and from `WM_PAINT` only for pending changes (`WM_SIZE` has drawn that size). Each one is paced like `run()`: it first waits for the frame-latency waitable (presents that skip it queue up, and the screen fell up to 460 px behind the window's edge), then calls `DwmFlush` after Present, so the loop's next window move lands in the same composition as the frame drawn for it. The cost is a step of about two display intervals instead of one (27 ms at 75 Hz). The swap chain's background colour (`SetBackgroundColor`) is the window's clear colour, so pixels no buffer covers show the theme background, not black: these are frames slower than a composition interval, which still lag the edge by one step, and maximize. Drawing in `WM_WINDOWPOSCHANGING`, before the rect moves, would remove that lag for the right and bottom edges; it is not done. `bench/resize` measures all of this on screen.
- IME: IMM32 composition strings and candidate window placement from `set_ime_rect`.
- Custom titlebar: `WM_NCCALCSIZE` plus `WM_NCHITTEST` from `set_titlebar_regions`. The maximize region returns `HTMAXBUTTON` so Windows 11 snap layouts work.
- `WM_DPICHANGED`: apply the suggested rect and emit `ScaleEvent`.

## Headless implementation (`platform/headless/`)
A complete, pure-V implementation used by the test harness. Windows are in-memory, events are injected, timers run on a virtual clock, the clipboard is a variable, dialogs return scripted answers, and paths point into a per-test temp dir. Image services decode through V's bundled `stbi` module, so tests stay OS-independent. SVG returns a solid bitmap of the requested size, since headless tests check layout rather than SVG pixels. It needs no window station, no desktop and no GPU (11-devtools-testing).

## Requirements for other OSes
Implement every interface and service above, or return a clear "unsupported" error for services that do not exist there (for example tray on some Linux desktops). Nothing OS-specific leaks above this module.

## Tests
`keys_test.v`, `utf16_test.v`, and `services_test.v` for path resolution.

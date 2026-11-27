# 15 — App Services

What a real application needs beyond widgets and windows: its settings and the state of its
windows, its language and the time where the user is, a log and a report of a crash, pages
it goes back and forth between, and how it starts and ends. All of it is in `quadra-app`, in
plain Rust over `quadra-platform`; what needs the OS beyond windowing is in `quadra-shell`
(16). behaviour/10 says what these services do; its code sections are history, superseded
by this file.

## Where files go

An app writes nothing unless it is told where: `App::dirs(AppDirs::for_app("name"))` (then
`Rc<AppDirs>` in the context of views) takes
the folders of the user's account (`%APPDATA%\name` for settings, `%LOCALAPPDATA%\name` for
logs and crash reports; on macOS `~/Library/Application Support/name` and
`~/Library/Logs/name`), read from the environment. Without them settings live in memory,
the log in its ring, and a crash report on the standard error stream. So tests, the shots
tool and the benches never write outside the project; a test that does gives a folder under
`.tmp/`.

## Settings

- `Settings` is one JSON object in one file. A key is a path of names with dots between,
  and the file holds objects in objects, as a person writes JSON: `editor.font_size` is
  `"editor": { "font_size": 15 }`. (Until 2026-11-24 each key was written whole at the top,
  `"editor.font_size": 15`, the store's map as it was: a file of that shape still opens to the
  same values, and is written as objects at its first change.) A key is not also the start of
  another: `editor` holds settings or is one.
  `App::settings()` opens `settings.json` in the config folder (one per app); any other file
  opens with `Settings::open(path)`, and `Settings::memory()` is one that is never written.
- `settings.value(key, default)` is a `Signal<T>` for a `T: Stored` (`bool`, the integers,
  `f32`, `f64`, `String`, `Option<T>`, `Vec<T>`; an app's own type implements the two
  functions of the trait over `serde_json::Value`). Views read it as any signal and are
  updated; writing it writes the setting. A value of the wrong shape in the file is the
  default.
- `settings.choice(key, options)` is a setting that is one of a few words, given as the index
  of its word in `options` (what a select or a segmented control writes) and kept as the word,
  so the file reads as the settings page does; where none is written, and for words the app
  does not know, it is the first, or `settings.choice_or(key, options, default)` the one at
  `default`.
- Writes are debounced (500 ms after the last change) and atomic: the text goes to
  `name.json.tmp` in the same folder, then is renamed over the file. `save_now` writes at
  once; the app calls it when it ends.
- A file that is not a JSON object is moved aside to `name.json.corrupt`, the defaults are
  used, and the log says so. A file of a newer version is read, never written over.
- `"version"` is the version of the file's shape: `settings.migrate(version, |from, map|)`
  changes a map of an older one and the file is written in the new shape.
- **Window state**: `WindowOptions::remember(key)` restores the window's size, place and
  whether it was maximized from the settings (`window.<key>`), and saves them when the
  window closes, or when the app ends with it open (`Platform::quit`: a tray's Quit). A place is a `Placement` (the frame when not maximized, in the units of the OS's
  desktop: device pixels on Windows, points on macOS, where one space of points holds
  displays of every scale; and whether it is maximized; `PlatformWindow::placement`,
  `WindowOptions::placement`). The platform opens a window at a place only when that place
  is on a display; elsewhere as its other options say (`centered`).
- UI state (split ratios, column widths, a folded sidebar) is a setting the app reads and
  writes with `value`: a component takes the signal.

## Time where the user is

- `Platform::utc_offset(unix_seconds)` is the offset of the user's time zone then, in
  seconds (summer time included). The headless platform's is set by the test, 0 unless.
- `quadra_app::time`: `Date { year, month, day }` (the proleptic Gregorian calendar,
  `days_in`, `weekday`, `add_days`, days from 1970), `Time { hour, minute, second }`,
  `DateTime`, `DateTime::from_unix(seconds, offset)`, and `local_now()`, which reads the
  clock and the offset of the app's platform; `utc_offset()` is that offset, for a thread
  without the app to say local times. The kit's calendar is on this `Date`.

## Language

- **Strings**: one JSON object a language, of flat keys:
  `{"files.count": {"one": "{n} file", "other": "{n} files"}, "hello": "Hello, {name}"}`.
  `I18n::new(default)` holds the tables (`add(locale, json)`); `App::i18n(i18n)` provides
  it, and its locale is a signal: what is said in a view is said again when the language
  changes, and nothing else is.
- `t(key)` and `t_with(key, &[("n", 3.into())])` look a key up in the app's i18n: in the
  current locale, then the default one, then the key itself; a key missing from a locale
  is logged once. They read the context, so they are for while a view is built; what runs
  again later (`text(move || ..)`) keeps `language()`, taken while it is built, and asks it
  (`lang.t(key)`, `lang.locale()`). Plurals are by language: Vietnamese has one form, English `one` for 1
  and `other` for the rest; a language of no rule here takes the English one.
- **Formats** by locale (`Locale`, a language tag; `vi` and `en` are known, any other is
  formatted as `en`): numbers (`1.234,5` and `1,234.5`), sizes of files (`1,5 MB` and
  `1.5 MB`, by 1024), dates short and long (`14/11/2026`, `14 tháng 11 năm 2026`;
  `11/14/2026`, `November 14, 2026`), times (`15:05`; `3:05 PM`), times relative to now
  (`3 phút trước`, `3 minutes ago`, `in 3 minutes`), the names of the months and of the
  days of the week, and the first day of the week (Monday; Sunday).
- More languages are data: a table of strings and, for formats, a `Locale` with its words.

## Log and crashes

- `quadra_app::log`: `error`, `warn`, `info`, `debug`, each `(target, message)`. Lines are
  kept in a ring of the last 500 and, with folders, appended to `logs/<date>.log` in the
  data folder; a day's file stops at 8 MiB, and files older than 14 days are deleted at
  start. `log::recent(n)` gives the last lines (the devtools console, a crash report).
- An error an app's hook does not handle (`App::on_error`) is logged.
- **Crashes**: with folders the app installs a panic hook that writes
  `crashes/<date-time>.txt`: the message, where, the backtrace and the last 100 lines of the
  log. `App::last_crash()` is the newest report written since the app last asked, so an
  app can offer to send it. A crash that is no panic (an access violation) is reported by
  `quadra-shell` (16).

## Pages

`Router<R>` holds a stack of routes, `R` any `Clone + PartialEq` value of the app; it is a
handle (`Copy`) over signals: `router(first)`, `current()` (read as a signal is), `push`
(forgets the pages ahead; the page shown is not pushed again), `replace`, `back`,
`forward`, `can_back`, `can_forward`. `router.view(|route| ..)` shows the current one, made
again only when the route changes (what the page reads does not remake it), and goes back
and forth on Alt-Left and Alt-Right (the actions `GoBack`, `GoForward`, bound in every
window's keymap) while the focus is in it or it is the window's view, and on the back and
forward buttons of a mouse over it.

## Start and end

- **The command line**: `App::args()` are the arguments it was started with, after the
  program's name (`App::with_args` in tests).
- **Closing**: a view says whether its window may close with `.on_close(|| bool)` on a box:
  when the user asks a window to close (the OS's close button, or `window().close()`, the
  app's own), such handlers are asked until one says no, and the window closes only if none
  does. A handler that says no asks the user (a dialog) and closes the window itself later
  with `window().close_now()`, taken while the view is built, which asks nothing.
- **Windows a view opens**: `Windows` is in the context of every window's views;
  `windows.open(options, || view)` opens a window after the entry point that asked (views
  cannot reach the app while a handler runs). The window belongs to the one it was asked from
  and closes with it. One kept under a name (`WindowOptions::remember`) that is open already
  closes first, so asking again brings the window to the front, its place kept. The harness
  has no `Windows`: a view that finds none shows the same thing in its own window.
- **The last window**: the app quits when its last window closes, unless
  `quit_on_last_window(false)` (an app of the tray).
- **One instance**: opt-in, in `quadra-shell` (16).

## Assets

Files an app carries are `include_bytes!` where they are used: fonts through
`App::load_fonts`, images through `image(&'static [u8])`. There is no registry by path.
ponytail: an app that looks files up by name, or reads them from disk in a debug build,
adds one.

## Tests

`crates/quadra-app/tests/settings.rs` (values, debounced and atomic writes, a corrupt file,
a migration, a window that is remembered), `tests/i18n.rs` (lookups and fallbacks, plurals,
Vietnamese and English formats), `tests/time.rs` (dates from Unix time and offsets, weeks),
`tests/log.rs` (the ring, files by day, a crash report), `tests/router.rs`,
`tests/close.rs`: all on the headless platform, with a clock the test moves.

# 16 — Shell Services

What an app asks of the operating system that is not windowing (10, "What is not in
`Platform`"): dialogs, opening files and links, the tray and notifications, one instance,
crash reports, and the package an app ships as. `quadra-shell` holds them; like the native
crate it has OS code and `unsafe` (01, R16), and an app that uses none of it links none.

## Traits and fakes

Each service is a trait with a Windows implementation and a fake in the crate that records
the calls and answers what a test queued (`FakeShell`). Views reach the app's through
context (`provide`, `use_context::<Rc<dyn Dialogs>>()` and the like); `quadra_shell::install`
provides the Windows ones. No test opens a real dialog, a real tray icon or a real
notification: they are manual checks (00, R5).

## Dialogs

- `Dialogs::open_files(&FileDialog) -> Vec<PathBuf>` (`FileDialog { title, filters:
  [("Images", "*.png;*.jpg")], multiple, folder, start }`; empty: given up),
  `save_file(&FileDialog) -> Option<PathBuf>`, `pick_folder(&FileDialog) ->
  Option<PathBuf>`, `message(&Message) -> Answer` (`Message { title, text, kind: Info |
  Warning | Error | Question }`; `Answer::{Ok, Yes, No, Cancel}`).
- They are modal over the window that asks and run on the UI thread, which they hold until
  they are answered, as the OS dialogs do: the OS's own loop runs meanwhile, so the app's
  windows still paint. The UI thread is a single-threaded COM apartment from the start of
  the native platform, which these dialogs need.

## Opening

`Opener::open(target)` opens a file in the app the user chose for it, or a link in the
browser; `reveal(path)` shows a file in the file manager, selected.

## Tray and notifications

`Tray::add(icon, tooltip, menu)` puts an icon in the notification area; its clicks and the
items of its menu come back to the app as messages of an inbox (11): the icon has a hidden
window of its own, and no `PlatformEvent` is added for it. `notify(title, text)` shows a
notification from the icon. An app of the tray says `quit_on_last_window(false)` (15).

## One instance

`single_instance(id) -> Instance`: the first process to ask holds a named mutex and a hidden
window of its own; a later one finds the window, sends it its arguments and is told to end
(`Instance::Second`). The first receives them through an inbox, on its UI thread.

## Crash reports

With the app's folders (15), `crash_reports(folder)` installs a handler of unhandled
exceptions that writes a minidump and the last lines of the log to `crashes/`, beside the
panic reports of the app.

## Packaging

`tools/package.sh <example>` builds the release binary of an example with a manifest of the
application (per-monitor DPI awareness, the UTF-8 code page, version 6 of the common
controls, which the dialogs use) embedded by the linker, and puts it with its assets in
`target/package/<name>/`. The gallery is packaged so. ponytail: no installer, version
resource or icon of the executable yet; an MSIX package when an app ships.

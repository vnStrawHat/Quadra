# 16 — Shell Services

What an app asks of the operating system that is not windowing (10, "What is not in
`Platform`"): dialogs, opening files and links, the tray and notifications, one instance,
crash reports, and the package an app ships as. `quadra-shell` holds them; like the native
crate it has OS code and `unsafe` (01, R16), and an app that uses none of it links none.

## Traits and fakes

Each service is a trait with an implementation for each OS and a fake in the crate that
records the calls and answers what a test queued (`FakeShell`). One crate holds every OS:
the traits, their types and the fake are the same everywhere, and each OS has a module
built only for it (`src/win/`, `#[cfg(windows)]`, with the `windows` crate a dependency of
that target alone), which gives `install`, `single_instance` and `crash_reports` of the
same signatures. Another OS is another module (`src/mac/`), not another crate; the
platform and renderer are crates by OS (`quadra-native-win`) because they are far larger. Views reach the app's through
context (`App::provide`; `dialogs()`, `opener()`, `Rc<dyn Sounds>`, taken while a view is
built);
`quadra_shell::install(app)` provides the Windows ones, `FakeShell::provide` a fake in a
test's scope. No test opens a real dialog, a real tray icon or a real
notification: they are manual checks (00, R5).

## Dialogs

- `Dialogs::open_files(&FileDialog) -> Vec<PathBuf>` (`FileDialog { title, filters:
  [("Images", "*.png;*.jpg")], multiple, folder, start }`; empty: given up),
  `save_file(&FileDialog) -> Option<PathBuf>`, `pick_folder(&FileDialog) ->
  Option<PathBuf>`, `message(&Message) -> Answer` (`Message { title, text, kind: Info |
  Warning | Error | Question }`; `Answer::{Ok, Yes, No, Cancel}`).
- They are modal over the app's active window and run on the UI thread, which they hold
  until they are answered, as the OS dialogs do. The handler that asked is still running,
  so what the platform delivers meanwhile waits for it: the app's windows keep their last
  frame. ponytail: a window resized behind a dialog paints when it is answered; dialogs
  that do not hold the handler when that matters. A dialog makes the UI thread a
  single-threaded COM apartment if it is none yet, which these dialogs need.

## Opening

`Opener::open(target)` opens a file in the app the user chose for it, or a link in the
browser; `reveal(path)` shows a file in the file manager, selected; `open_with(program, args,
file)` starts a program the user named with the arguments, then the file (no console window
for it, no shell between: the arguments are not read again). What opens a program
opens it: a link from content that is not the user's goes through
`quadra_text::url_openable` first.

## Sounds

`Sounds::play(Sound)` plays one of the system's sounds and returns at once: `Sound::{Default,
Info, Warning, Error, Question}`, the sounds the user set for those events (`MessageBeep`). The
fake records `sound <kind>`; a test never plays one.

## Tray and notifications

`Tray::add(image, tooltip, menu, events)` puts an icon in the notification area (the
app's picture, `TrayImage` in RGBA, or the system's) and gives a `TrayIcon`, which takes it
away when dropped; its clicks and the items of its menu (`TrayEvent::Click`,
`DoubleClick`, `Item(i)`) come back to the app through the `Sender` of an inbox (11): the
icon has a hidden window of its own, and no `PlatformEvent` is added for it. When the
taskbar starts again (Explorer restarted, the message `TaskbarCreated`) the icon is added
again, with its tooltip as it is.
`icon.notify(title, text)` shows a notification from the icon. An app of the tray says
`quit_on_last_window(false)` (15). `tray()` finds the app's in a view's context; the fake
records what it is asked and `FakeShell::click_tray` is the user.

## One instance

`single_instance(id, args) -> Instance`: the first process to ask in the user's session
holds a named mutex and a hidden window of its own (`Instance::First(inbox)`); a later one
finds the window, sends it the folder it started in and its arguments (`WM_COPYDATA`) and
is told to end (`Instance::Second`). The first makes its window just after its mutex, so a
later one that starts in between looks for it again for up to two seconds. The first receives each `Launch { folder, args }`
through the inbox, on its UI thread (`.on_data(..)` in a view).

## Crash reports

`crash_reports(folder)`, with the app's `crashes` folder (15), installs a handler of
unhandled exceptions that writes `<date-time>.dmp`, a minidump of the process at the
exception, and `<date-time>.txt`, the last 100 lines of the log, beside the panic reports
of the app; the process then ends as it would have.

## Process usage

`process_usage() -> Option<Usage>`: what the app's own process has used, `Usage { cpu,
uptime, memory, working_set, peak_working_set, private_bytes, threads }`: the processor time
of all its threads since it started and the time since it was created (`GetProcessTimes`);
its private working set (`memory`, what the task manager shows as its memory), working set,
peak working set and private bytes (the commit) in bytes (`GetProcessMemoryInfo` with
`PROCESS_MEMORY_COUNTERS_EX2`); and its count of threads (the process's entry in a Toolhelp
snapshot of the processes, `CreateToolhelp32Snapshot`, closed at once; 0 when not found).
`None` where the OS has no reading. A status bar takes two readings and divides the processor
time between them by the uptime between them and the count of processors.

The trait `Meter { fn usage(&self) -> Option<Usage> }` is how views reach it: `install`
provides the Windows one (`process_usage`), and `FakeShell::provide` the fake's, which
answers what `FakeShell::set_usage` gave it last (`None` before), so a test sets the numbers
and moves them on.

For the comparison bench's runner, which measures another process from outside
(`bench/runner`, docs/benchmarks.md, "Method"): `process_cycles(&Child)`, the cycle count of
all the child's threads (`QueryProcessCycleTime`: exact, where the times of `GetProcessTimes`
count whole clock ticks of 15.6 ms); `cycles_per_second()`, the rate of the cycle counter,
measured once by spinning the calling thread for 200 ms against the wall clock (the counter
runs at a constant rate) and kept, so the runner asks for it before it measures anything;
`private_memory(&Child)`, the child's private working set in bytes
(`PROCESS_MEMORY_COUNTERS_EX2`), what the task manager shows as a process's memory;
`window_shown(&Child)`, whether a window of the child is visible and not minimized. Each is
`None` (`false`) where the OS has no reading. They borrow the child's own handle, so the
child outlives every reading.

## Passwords

`hash_password(password) -> Option<String>` and `verify_password(password, stored) -> bool`:
a password kept as PBKDF2 with HMAC-SHA256 from the OS's crypto (`BCryptDeriveKeyPBKDF2` with
the HMAC-SHA256 pseudo handle), a salt of 16 random bytes (`BCryptGenRandom`), 600,000 rounds
and a key of 32 bytes, written `pbkdf2-sha256$<rounds>$<salt hex>$<key hex>` so the rounds can
grow later. Verifying reads the rounds and salt from what is stored and compares the keys in
time that does not depend on where they differ. Both take a noticeable part of a second: an
app runs them on the pool. `None` where the OS has no random bytes.

Sealing, for a secret an app keeps (a saved password): `derive_key(password, salt, rounds)`
is that PBKDF2 key of 32 bytes, `random_bytes::<N>()` the OS's random bytes, and
`seal(key, plain, bound)` / `open(key, sealed, bound)` AES-256-GCM through the OS's crypto
(the AES-GCM pseudo handle): a fresh random nonce of 12 bytes, then the ciphertext, then the
tag of 16; `bound` is the associated data, checked by the tag and not kept. `open` gives
`None` for a wrong key, other associated data or bytes changed. Nothing here writes a key anywhere.
What is secret comes back wiped on drop (`zeroize`'s `Zeroizing`, re-exported with
`Zeroize`): the derived key, what `open` opened, and `random_key()`, 32 random bytes made in
place; the derived keys `verify_password` compares are wiped too.

## Console

For a program of the bench that writes to the console it runs in (`bench/doomfire`):
`console::size()` (the window's columns and rows, from the screen buffer's window),
`console::enable_vt()` (virtual terminal processing and no auto return after a line feed on
the output, the UTF-8 code page; false when the output is no console), `console::write(bytes)`
(`WriteConsoleA`, all of it; the bytes as they are when the output is no console).

## Packaging

`tools/package.sh <example>` builds the release binary of an example with
`tools/app.manifest` (per-monitor DPI awareness, the UTF-8 code page, Windows 10 and later,
version 6 of the common controls, which the dialogs use) embedded by the linker
(`cargo rustc` with `/MANIFEST:EMBED /MANIFESTINPUT:`, so only the final crate is linked
again), and puts it in `target/package/<name>/`: fonts and images are in the binary, so it
is alone there. The script fails when the manifest is not in the binary. The gallery is
packaged so. ponytail: no installer, version
resource or icon of the executable yet; an MSIX package when an app ships.

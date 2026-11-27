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
fake records `sound: Error` and the like; a test never plays one.

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
is told to end (`Instance::Second`). The later one lets any process take the foreground
(`AllowSetForegroundWindow`) before it sends, as it is the one the user just started: so the
first may bring its window to the front. The first makes its window just after its mutex, so a
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

`shell_process(pid) -> Option<ShellProcess>`: a shell's process and what it runs, for a
terminal to say. `ShellProcess { path, running }`: the full path of the process `pid`
(`QueryFullProcessImageNameW`), and the command line of the newest process it started
(its children in a Toolhelp snapshot, the console hosts `conhost.exe` and `OpenConsole.exe`
left out, the newest by `GetProcessTimes`; its command line by `NtQueryInformationProcess`
with `ProcessCommandLineInformation`), `None` when it runs nothing. `None` when `pid` cannot
be opened (gone, or not ours to read). Read once a call, on the caller's thread: a snapshot
and a few handles, closed at once.
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

## Fetching

For an app that reads something from the network (the updater, specs/22):
`http_get(url, headers, network, body) -> Result<u16, String>` is a GET by the OS's own HTTP
(WinHTTP), so its certificates and its TLS are the ones used, and no HTTP or TLS code is
linked in. `network` is a `Network { proxy, verify_tls }`, by default the proxy the system is
set to (found by itself) and certificates checked. `proxy: Some("host:port")` names an HTTP
proxy by hand (an `http://` before it is taken off; an address that is none, SOCKS or one
with a name and a password in it, is an error before anything is asked: a proxy that asks
for a name is not served yet; a server of this machine itself, `127.0.0.1`, is asked
without the proxy, which is the OS's rule). `verify_tls: false` takes a server's certificate whoever
signed it and whatever name, date and use it says: for a network that opens TLS on the way,
at the cost that whoever stands there can answer in the server's place. `headers` are sent as given, the
`User-Agent` among them being the agent; `body` is given what comes piece by piece (64 KiB
at most) whatever the status, and stops the GET by answering `false` (the error `stopped`).
The answer is the status; an error says in words what failed (no connection, cut off).
Redirects are followed, never from https to http. Resolving and connecting give up after
10 s, a silence of the server after 30 s; there is no limit on the whole, so a large body on
a slow line still arrives. It blocks: an app calls it on the pool. Only `http` and `https`
addresses are asked; which of them an app accepts is the app's rule.

`sha256(bytes) -> Option<[u8; 32]>` is the OS's SHA-256 of bytes in memory, to check what
was fetched against a digest published for it.

## Console

For a program of the bench that writes to the console it runs in (`bench/doomfire`):
`console::size()` (the window's columns and rows, from the screen buffer's window),
`console::enable_vt()` (virtual terminal processing and no auto return after a line feed on
the output, the UTF-8 code page; false when the output is no console), `console::write(bytes)`
(`WriteConsoleA`, all of it; the bytes as they are when the output is no console).

## File types

An app says which kinds of file it opens (`FileTypes`: its name as registered, its vendor,
what it is, what its documents are called, their extensions, its executable), and the trait
`Associations` registers it for them, says which program opens a kind now, and shows where
the user makes it the default.

- **What is written** is a pure function, `registration(&FileTypes)`, of values under
  `HKEY_CURRENT_USER` alone (nothing machine-wide, no administrator):
  - the program identifier `<Vendor>.<Name>.Document`, named as the documents are, with the
    executable's first icon (`DefaultIcon`) and `"<exe>" "%1"` as its open command;
  - each extension's `OpenWithProgids` naming it, and the executable's `SupportedTypes` under
    `Software\Classes\Applications\<exe name>`: it is offered in "Open with";
  - its capabilities (`Software\<Vendor>\<Name>\Capabilities`: its name, its description,
    the extensions it takes), named in `Software\RegisteredApplications`.
  `WinShell` writes them as strings and tells the shell associations changed
  (`SHChangeNotify`). The fake records what it was asked.
- **The default is the user's to choose.** Windows lets no program make itself the default
  for a kind of file: `choose_default` opens the system's Default apps page at the app
  (`ms-settings:defaultapps?registeredAppUser=<name>`, Windows 11 21H2 with the update of
  2023-04 and later), where the user chooses.
- **Which program opens a kind** now is the shell's answer (`AssocQueryStringW`, the
  executable), so an app can say whether it is the one.
- Tests never write the registry: they use the fake, and the function's values are unit
  tested.

## On macOS

The same traits behind the same `install`, by `quadra-shell/src/mac` (23, M3):

- **Dialogs** are AppKit's, on the main thread: the open and the save panel, and an alert.
  A dialog's title is the words above the files, as a panel shows no title of its own; its
  filters are extensions, and one that takes any file leaves the panel open to all. A
  question has Yes, No and Cancel; the buttons are in English.
- **Opening** is the system's `open`: a file or a link; `open -R` shows a file in the
  Finder; an app (a bundle) the user named opens the file as the Finder would
  (`open -a`), and any other program is run with the arguments and the file.
- **Sounds**: the alert sound the user set, for every kind.
- **One instance**: the first process listens on a socket in the user's folder of temporary
  files (`quadra-<id>.sock`); a later one hands it the folder it started in and its
  arguments, and returns once the first has them. A socket nobody listens on, left by a
  process that ended, is taken away. Two started in the same instant may both be the first.
- **Crash reports**: macOS writes the report of a crash itself
  (`~/Library/Logs/DiagnosticReports`). What it cannot know, the app adds: `crash_reports`
  handles the signals of a crash (an instruction that is none, a trap, an abort, arithmetic,
  the two of memory) on the stack kept for signals, writes `<date-time>.txt` with the signal
  and the log's last 100 lines in the folder, and returns, so what crashed crashes again
  under the system's handler, which writes its report and ends the process. A panic that
  aborts is the report of panics' to tell. The handler allocates and writes a file, which a
  handler of signals is not promised it can: an alarm ends a process that waits there for
  itself.
- **File types** are those the app's bundle names (`Info.plist`), which Launch Services
  reads when it registers the bundle. `Associations::register` has it read the bundle the
  program is in again (`LSRegisterURL`), and says why not for a program in no bundle.
  `opener_of` asks which app opens the kind an extension is of
  (`UTTypeCreatePreferredIdentifierForTag`, `LSCopyDefaultApplicationURLForContentType`) and
  gives the program its bundle names, so an app compares it with its own as on Windows. On
  macOS an app may ask to be the default, and the system lets the user agree first:
  `choose_default` asks, for each of its extensions' kinds, the workspace
  (`setDefaultApplicationAtURL:toOpenContentType:`, macOS 12 and later) and Launch Services
  (`LSSetDefaultRoleHandlerForContentType`, for macOS 11). It is true when the system took
  the asking, false when it did not or the program is in no bundle; whether the app is the
  one is `opener_of`'s to say, which an app asks again when its window comes back to the
  front. Asked of the runner's macOS 26 (run 45935041715): Launch Services' call answers
  that it did and changes nothing, and the workspace's, asked in Swift too, never says it
  is done while nobody answers. The other calls are Launch Services' calls of C,
  deprecated since macOS 12 and kept.
- **The tray** is an item in the menu bar (`NSStatusItem`) with the app's picture (RGBA, at
  18 points) or a copy of the app's icon, and the tooltip. As the menu bar's items do, one
  with a menu opens it when it is clicked, and an item chosen is `TrayEvent::Item`; one
  with no menu tells its click (`TrayEvent::Click`). There is no double click there.
  Dropping the icon takes the item away.
- **Notifications** are said by the system's `osascript` (`display notification`, the
  title and the text passed as arguments, never as script): they show under Script
  Editor's picture, not the app's. `UNUserNotificationCenter`, which wants the app in a
  bundle and the user's leave, when an app on macOS has much to say.
- **Process usage** is read from the system's `libproc` (`proc_pid_rusage`, the task's
  info), where Activity Monitor reads: the processor time (the system's clock ticks, made
  time), the time since the process started (which leaves out the time the Mac slept), the
  memory Activity Monitor shows (the physical footprint, as `memory` and as
  `private_bytes`), what is resident and the most that was, the threads. Another process
  from outside, for the runner: its processor time in nanoseconds stands for the cycle
  count (`cycles_per_second` is 10^9), its footprint for its memory, and its window is
  shown when the window server lists one of its own at the layer of apps' windows.
  `shell_process`: the path (`proc_pidpath`), the processes it started (`proc_listpids`),
  the newest's arguments (`sysctl`, `KERN_PROCARGS2`) with a space between two.
  `user_environment` is the process's own: macOS keeps none beside the processes, and a
  shell makes its own from its profiles.
- **Passwords and sealing** are CommonCrypto's, in the form Windows writes, so a vault
  sealed on one system opens on the other (`tests/seal.rs` opens on every system what
  Windows sealed): `CC_SHA256`, `CCKeyDerivationPBKDF` (PBKDF2 with HMAC-SHA256),
  AES-256-GCM by `CCCryptorGCMOneshotEncrypt` and `Decrypt` (in the system's library since
  macOS 10.13; declared in the header Apple's own code uses, not in the public one), random
  bytes from `/dev/urandom`. The hash a password is kept as (`pbkdf2-sha256$...`) is made
  and verified by code shared by the systems (`secret.rs`).
- **Fetching** is the `curl` macOS ships (`/usr/bin/curl`), so the system's certificates
  and TLS are used and nothing of HTTP is linked in: the body comes on its output piece by
  piece, the status last on the other stream, an error is `curl`'s words. Redirects are
  followed, never from https to http (`--proto-redir`); connecting gives up after 10 s, a
  silence after 30 s. The proxy is the one named by hand, or the one System Settings names
  for the address's scheme with the hosts it leaves out (`scutil --proxy`); the
  environment's variables are not read. A proxy found by a script (a PAC file) is not
  asked: the request goes straight. The updater (22) checks, fetches and verifies on macOS
  by it; installing there waits for the bundle (23).
- **The console** (`console`): the size is the terminal's (`TIOCGWINSZ`), `enable_vt` is
  whether the output is a terminal (it reads the sequences with nothing to turn on), and
  `write` writes to the output at once.
- **Files the system asks the app to open** (the Finder, `open`, a file dropped on the
  app's icon) reach an app in a bundle as an event of the application, not as arguments,
  whether it runs already or is started for them. The first instance's inbox is given them
  as a `Launch` whose arguments are the files' paths, so an app that listens for later
  starts opens them with no code of its own.
- **The bundle**: `tools/package_mac.sh <app>` on a Mac puts the shipped binary in
  `target/package/<Name>.app` with an `Info.plist` that names it (the identifier
  `io.github.vnstrawhat.<name>`, or `PACKAGE_ID`), its version (the workspace's), the kinds
  of file it opens (Markdown: `md`, `markdown`, and the kind by the system's name for it,
  `net.daringfireball.markdown`, a kind of text, which the bundle says again for a Mac
  where no other app has: asked which app opens a kind, Launch Services answers among the
  apps that name the kind, not its extensions alone) and that it draws at the display's own
  scale; the icon is made of the app's `.ico` by the system's `sips` when it can; the bundle
  is signed ad hoc, which runs on the Mac that made it. Not yet: a Developer ID signature
  and notarization, which an app that ships to others needs.
- **Tests**: the tests of fetching, hashing, sealing, passwords, usage and a process read
  from outside are the same on both systems and run on the CI's Mac. There alone
  (`CI` set) an item is put in the menu bar and taken away (`tests/tray_mac.rs`), and the
  bundle the job made is registered and asks to be the default for `.md`
  (`tests/assoc_mac.rs`, `QUADRA_BUNDLE`; what the runner's screen shows then is kept as
  `default.png`): no test changes a person's menu bar or which app
  opens their files.

## Packaging

`tools/package.sh <example>` builds the release binary of an example with
`tools/app.manifest` (per-monitor DPI awareness, the UTF-8 code page, Windows 10 and later,
version 6 of the common controls, which the dialogs use) embedded by the linker
(`cargo rustc` with `/MANIFEST:EMBED /MANIFESTINPUT:`, so only the final crate is linked
again), and puts it in `target/package/<name>/`: fonts and images are in the binary, so it
is alone there. The script fails when the manifest is not in the binary. The gallery is
packaged so, and Markdown (`tools/package.sh markdown`).

**An executable's icon.** An app's build script turns an `.ico` file into a resource file of
an icon group (resource 1, its images 1 to n) and links it with the binaries: written by the
script itself, as the format is a few headers, so no resource compiler is needed. The
code is the crate `quadra-build`, which depends on nothing: an app names it under
`[build-dependencies]` and its `build.rs` calls `quadra_build::icon("assets/logo.ico")`
(Markdown's and OneTerm's do). It is a crate, where it was a file of `tools/` taken by path
until 2026-11-25, so that an app in a repository of its own has it as it has the rest of
Quadra. `tools/package.sh`
fails when an app's binary has no icon (`tools/exe_icon.py`), as OneTerm's own release script does. ponytail: no installer or version resource yet; an
MSIX package when an app ships.

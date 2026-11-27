# 23 — macOS

Quadra on a second operating system: macOS on Apple silicon (`aarch64-apple-darwin`). Started
by the user on 2026-11-26, on the branch `macos`. Windows stays the first system: nothing
here changes what a Windows build is or measures.

## Goals

- **The same framework above the traits.** A second OS is a second native crate and a second
  module of the shell (10, "The native crate"; 16, "Traits and fakes"): `Platform`,
  `PlatformWindow`, `Renderer`, `TextBackend`, `Clipboard` and `ImageDecoder` are implemented
  again, and nothing above them knows which system it runs on. Where the traits turn out to
  name a Windows idea, the trait changes, for both systems.
- **Native, as on Windows**: the system's own windows, GPU API, text and accessibility, with
  the budgets of 14 measured again on a Mac. No layer that draws for both systems.
- **Never a broken Windows build.** The Windows gate (`tools/check.sh`) stays what a commit
  must pass.
- **Quadra, not OneTerm.** The app has moved to its own repository (00, the decisions of
  2026-11-24); what is still under `apps/oneterm` here is not ported, fixed or checked for
  macOS (the user's decision, 2026-11-26). OneTerm on a Mac is that repository's work, on
  what Quadra gives it there.

## What is built

| Part | Windows | macOS |
|---|---|---|
| Windows, events, the loop, input methods, cursors, drops | Win32 (`quadra-native-win`) | AppKit (`quadra-native-mac`, new) |
| Renderer | D3D11 | Metal |
| Text | DirectWrite | Core Text |
| Accessibility | UI Automation, by `accesskit_windows` | NSAccessibility, by `accesskit`'s adapter for macOS (01, R11) |
| Image decoding | WIC | Image I/O |
| Shell services (16) | `quadra-shell/src/win` | `quadra-shell/src/mac` |
| Sealing, hashing, fetching (16) | CNG, WinHTTP | the system's: CommonCrypto, and the `curl` it ships (16, "On macOS") |
| Pseudo terminal (18) | ConPTY | the POSIX pty of `oneterm-vt` |
| Updates (22) | an `.exe` replaced while it runs | an app bundle; decided when an app ships there |

## Decisions taken by the user (2026-11-26)

| Question | Decision |
|---|---|
| The renderer | A Metal backend written for Quadra, as the D3D11 one was (07): no crate that draws for both systems |
| Bindings to Apple's APIs | A crate may provide them, as `windows` does on Windows. Which ones is written in 01 (R15) and `tools/check_deps.py` when M2 first names them, as for any crate |
| OneTerm | Not ported here (Goals) |
| Pushes | Claude may push the branch `macos` unasked, to have the CI run; every other branch is asked for, as before |
| Where Windows and macOS do a thing differently | On macOS, as macOS does: Command for shortcuts and the Mac's signs for them, the system's own window buttons at the left |

## How it is developed without a Mac

The user's machine runs Windows; a Mac is not at hand yet.

- **From Windows: `tools/check_mac.sh`** runs clippy for `aarch64-apple-darwin`. That needs no
  Mac: nothing is linked, so no linker and no SDK (Apple's SDK may be used on Apple's hardware
  alone, so nothing is linked for macOS here). It finds what does not compile there.
- **On a Mac: the same script** runs clippy and the tests natively. The CI does
  (`.github/workflows/macos.yml`, a runner of GitHub's, on a push to `main` or `macos` and on
  a pull request). The tests of the framework need no OS and no GPU (12), so they say on a
  Mac what they say on Windows.
- **The runner has Metal** (asked once, 2026-11-26, run 45794320487; macOS 26.7 on Apple
  silicon): its one device is `Apple Paravirtual device`, a virtual machine's GPU. A probe
  compiled shaders from source, made a pipeline of 4 samples, drew a triangle into a texture
  without a window, resolved it and read back the pixel it wanted, the command buffer done
  in 150 ms. So the Metal backend is drawn and compared on the CI. What it says of time is a
  virtual GPU's: nothing measured there is a budget (M5).
- **What a runner cannot say**: how a window feels. Resizing, the trackpad, input methods,
  Retina, the menu bar and the timing of a frame are tried on a real Mac, by hand, as the
  manual checks of R5 were on Windows. M2's window work waits for one.
- **What the runner's screen shows**: the job opens Markdown's bundle as the Finder opens a
  file, asks it for a second file while it runs and asks it to quit, and keeps pictures of
  the whole screen with the run (`tools/window_mac.sh`, in `.tmp/shots/window`): the one
  place the system's three buttons are seen over the app's title bar. A try that fails no
  job: a runner may capture nothing. It did capture (run 45922708564, 2026-11-27): the
  window with the system's buttons at the left of the app's bar, a few points above its
  middle, the app's own menu and Window in the menu bar, the bundle's icon in the Dock; the
  file opened by `open` shown, the second file a tab of the one process, and the app
  ended by Quit. The buttons have been put at the bar's middle since.

## What is left out, for now

`tools/check_mac.sh` names the packages that are not built for macOS; the list is what is
left of the port, and a milestone ends by taking names off it:

- `quadra-native-win`, and the tools that measure with it (the comparison bench, the fire
  demo) (M2, M5). The bench's runner, which measures an app from outside by the shell,
  builds for macOS.
- Every package of OneTerm, for good (Goals).

Where Quadra has no native crate at all (not Windows, not macOS), `quadra::App` is an app on
no GPU (`NullRenderer`), so that what names the type builds, and `quadra::app()` says that
Quadra has no native backend for the system.

## The native crate (M2)

`quadra-native-mac` is built for macOS alone: on another system it is an empty crate, so the
workspace builds everywhere. Its bindings are the `objc2` family's crates of Apple's
frameworks (01, R15), each with the features used and no more.

### Text: Core Text (`CoreText`, as built)

- **Families.** The installed ones by their names, as on Windows; fonts loaded from memory
  (`load_fonts`: every face of a file, a collection too), searched first and matched by
  slant, then by the nearest weight. The system's own font has no name a program may ask it
  by, so it answers to `system-ui` (`SYSTEM_FAMILY`), which is the default family: the
  interface's font at any weight. A family that is not installed is no face (Core Text
  answers such a name with another font; that is told apart by the family of what came back).
- **A face is a font at a size.** A font of Core Text is of one size, and the system's own
  is another design at another size (its optical size and its tracking), so a face's font is
  made again at each size it is shaped or drawn at, and kept. Metrics are read at 16 and
  given per em, as the trait asks.
- **Shaping** is a line of Core Text: its runs come in the order they are drawn, and each
  names its font, the face asked for or the one the system's fallback chose, which becomes a
  face known by its PostScript name. A glyph's cluster is the byte its grapheme begins at: a
  mark is of the cluster of its base.
- **Rasterizing** draws a glyph into a bitmap just large enough, a pixel more each way: one
  byte of coverage a pixel, or premultiplied RGBA when the font has colours of its own
  (emoji). macOS smooths no font by subpixel, so `GlyphFormat::Subpixel` never comes from
  here and the flag `GRAYSCALE` changes nothing. A glyph stands where its pen is, to a quarter
  of a pixel.
- **Features and fallback families** are given to a face's font when it is made at a size:
  the OpenType features its family is shaped with (`set_features`; the family a face was
  asked by, so `system-ui` for the system's font), and the families tried for what a face
  lacks before the system's fallback (`set_fallbacks`: Core Text's cascade list; a family not
  installed is passed over). Setting either forgets the fonts made at a size.
- **Bidirectional levels** (`bidi_levels`) are the Unicode algorithm's as the system's ICU
  runs it (`libicucore`, four functions declared here): Core Text tells a run's direction,
  not its level.

Its tests (`tests/coretext.rs`) run on a Mac alone: the CI.

### Rendering: Metal (`MetalRenderer`, as built)

The Windows renderer's twin (07): the same instances, a buffer a layer brought in step with
what the layer says changed, the scene's steps drawn as runs of instances of one pipeline.

- **The shader** is `shaders/prims.metal`: the Windows shader (`prims.hlsl`) in Metal's
  language, line for line. One thing is not carried: the second output that blends a glyph
  smoothed by subpixel a channel at a time, as macOS smooths none so; blending is
  premultiplied alpha. It is compiled from its source when the device is made (a library
  built ahead by Xcode's tools waits for a Mac and for start-up to be measured, M5).
- **A frame** is one command buffer. A layer's instances are a buffer in memory the CPU and
  the GPU share, bound to both stages, read by index; a run's constants are copied into the
  commands; the run is a strip of four vertices an instance. A layer of which nothing is
  inside its scissor draws nothing.
- **A blurring box**: the pass ends, what is drawn under the box is copied into a texture
  and its mips are made, and a pass that keeps what is drawn begins.
- **Atlases** are texture arrays, grown by a copy of their slices; a slot whose texture does
  not exist yet is bound to a texture of one pixel (a draw may read none that is not bound).
- **Surfaces**: a texture that is drawn into and read back, RGBA of 8 bits, in shared
  memory (Apple's silicon). A window's surface comes with the AppKit platform; `draw` waits
  for the GPU, as what was drawn is read next.
- **A window's surface** is a `CAMetalLayer` given to the window's view (BGRA of 8 bits,
  which a layer takes; a second pipeline state for it). `draw` draws into the layer's next
  drawable and keeps the commands; `present` has them show it and sends them. One frame is
  with the GPU at a time: the next waits for it, as it writes into the buffers that one
  reads. A window that is on no display has nothing to draw into: no frame, and no error.
- **Not yet**: what a frame cost the GPU (`RenderStats::gpu`).

Its tests are the Windows renderer's, on this one (`tests/pixels.rs`, `text.rs`,
`gradient.rs`, `mask.rs`, `backdrop.rs`): the two are held to the same pixels.

### The platform: AppKit (`MacPlatform`, as built)

On the main thread alone, as AppKit is. `quadra::app()` builds an app on it, the Metal
renderer and Core Text.

- **The loop** is Quadra's own turn of AppKit's queue (`nextEventMatchingMask`), not
  `NSApplication.run`: between two events it gives the windows that asked their frames,
  fires the timers that are due and delivers a wake. It sleeps until the next timer, or not
  at all while a window asks for a frame. A wake from another thread, and `quit`, post an
  event of the app's own that only turns the loop.
- **Frames** are paced by the layer: taking its next drawable waits for the display.
- **A window** is an `NSWindow` whose content is one view, a subclass of `NSView` counted
  from its top left; a window's handle is that view's address (`WindowHandle::AppKit`, as
  `SurfaceTarget::AppKit`: the trait's types name the second system now). The view tells the
  handler presses, releases, moves and leaves of the pointer (a tracking area), the wheel
  (lines from a wheel, points from a trackpad), keys by their code, and the text a key types
  (`interpretKeyEvents`, which gives it as `insertText:`). The window's delegate tells a
  request to close (answered no: the app closes a window by dropping it) and the focus.
  While the user drags the window's edge the loop does not turn, so a resize gives the app
  its frame at once. A size is told once: AppKit sets a view's size more often than it
  changes.
- **The pointer's shape** is a cursor rectangle over all of the view, made again when the
  app sets another shape. Only the cursors AppKit names are used: a move is the open hand,
  a diagonal resize is the cursor of a frame's corner (macOS 15; the arrow before it), and
  waiting, progress and help are the arrow (the system shows its own wheel over an app that
  does not answer). The ones the system's own apps show without a name are not asked for:
  macOS 26 says it answers `_moveCursor` and raises an exception when asked for it, so
  whether one is there cannot be known before it is asked for.
- **The appearance** is read from AppKit's own: dark or light, more contrast (the user's
  "Increase contrast"), the accent colour. The view tells a change of it
  (`viewDidChangeEffectiveAppearance`). The system's own colours and a size of text are not
  said: macOS has no contrast themes, and no text size for all apps.
- **A window with its own frame** has the content under a transparent title bar, with the
  system's three buttons over it: the window says where (`controls`: as far as the zoom
  button reaches and as much again as is before the close button), and the kit's title bar
  leaves them that room and draws no buttons of its own (10, 13). A press where the app says its title bar is
  (`hit_test`: `Caption`) is the system's: the window is dragged by it
  (`performWindowDragWithEvent`), and zoomed by a second press. `Maximize` is the app's
  own, as the system shows nothing over it. Never run yet: the test's window is not shown.
- **Text and input methods**: the view is an `NSTextInputClient` that keeps no text. While
  something takes text (`set_text_input` with a caret) a key goes through the input method
  (`interpretKeyEvents`), which commits text (`Text`) or composes it (`Composition`: its
  text, the end of what it selects as the caret, what it selects as the clause being
  converted); the candidates open beside the caret (`firstRectForCharacterRange`). While an
  input method composes, a key is its own and is not told as a key. While nothing takes
  text no input method is asked: a key types its own characters, and what was being composed
  is dropped. With Command held a key types nothing.
- **Images** are decoded by the system's codecs (Image I/O, `ImageIo`): a size from the
  header as the image is seen (a camera's orientation tag turns it), a frame drawn by Core
  Graphics at the size asked for into premultiplied RGBA in sRGB, the frames of a GIF whole
  with their times. The other formats that move show their first frame.
- **The handler is never entered twice**, as on Windows: what arrives while it runs waits.
- **Keys, as a Mac has them** (the user's decision, 2026-11-26: on macOS, as macOS does).
  Command is the key shortcuts are held with, `Mods::CTRL`, and Control the system's other
  one, `Mods::SUPER` (10, "Modifiers"); Option is `Mods::ALT`. So the kit's bindings, a
  Command-click that adds to a selection and a Command-wheel that zooms need nothing of
  their own. A press of the left button with Control held is the right button's. Shortcuts
  are written in the Mac's signs (`KeyNames::Signs`: `⇧⌘P`). The terminal sends
  control characters for `SUPER` here, and copies with `CTRL` (18, "Input").
  The keys of text are the Mac's (09, "On a Mac"): Option with an arrow goes by a word,
  Command with one to an end; back and forward are Command with a bracket.
- **Drops** from other programs (10, "Drops from the OS"): the view takes files and text
  (`NSPasteboardTypeFileURL`, `NSPasteboardTypeString`), files when there are both, and
  says a copy is taken wherever it is. What a pasteboard carries is tested on one of the
  test's own; a drag over a window is tried by hand.
- **The menu bar** has what a Mac's user finds in every app's, with the keys the system
  keeps for it: the app's own menu (Hide with Command-H, Hide Others, Show All, Quit with
  Command-Q) and Window (Minimize with Command-M, Zoom, and the windows the system lists).
  Quit ends no process: every window is asked to close, as its close button asks, and the
  app decides (15, "Closing"). No item takes a key an app may want for itself: closing with
  Command-W is the app's binding. Its titles are in English.
  The code editor's keys follow (09).
- **A placement** (15, "Window state") is the window's frame in the desktop's points, from
  the bottom left of the first display, as AppKit keeps it: one space holds displays of
  every scale, so nothing is counted in device pixels. A window opens at a place it is given
  only when a display shows 64 points of it each way (`appkit/place.rs`); a zoomed window
  comes back by its frame.
- **The buttons** stand at the middle of the app's title bar (`appkit/buttons.rs`). AppKit
  puts them at the middle of its own bar, which is lower than an app's: after every frame
  the window asks the handler how far down the point under the close button is caption
  (the question the system asks of a point; two questions while the height holds), makes
  the view that holds the system's bar as tall, and puts each button at its middle, again
  whenever AppKit has put them back. Seen on the runner's screen.
- **Filling the screen** the window shows no buttons, and says so (`controls` with no
  room): the kit's title bar gives their room back, and takes it again when the window
  leaves the full screen (the delegate tells the handler the size again both times).
- **Readers** (12, "Accessibility"): the window's semantic tree is told to the system's
  accessibility by the adapter of `accesskit` for macOS (`appkit/access.rs`), which gives
  the view a class of its own over the view's before the view is in a window. Tested by
  asking the view what a reader asks (`tests/access.rs`); VoiceOver itself is tried by hand.
- **Not yet**: an app's own menus in the menu
  bar, and the bar in the app's language; what
  the user chose a second press on a title bar to do.

Its test (`tests/window.rs`) has a `main` of its own, as AppKit must be on the main thread:
a real window that is not shown, what the loop gives the handler (timers, a frame, a wake
from another thread), AppKit's events sent to the window from inside the handler and what
they arrive as (a press, a key and its text with and without something that takes text,
what an input method composes and commits, the other appearance, a new size, a request to
close), a cursor for every shape, and frames drawn into the window's layer.

### The gallery's pages on a Mac

`bench/shots` draws with the renderer, the text and the codecs of the system it runs on: on
a Mac Metal, Core Text and Image I/O, with the headless platform, as on Windows. The CI's
job draws every page of the dark theme after the tests (`tools/shots.sh dark`): a page that
does not draw fails the job. The pictures are kept with the run for fourteen days
(`gallery-dark-macos`, for who is signed in to GitHub), and the run's notice says how many
there are. They are not compared with the Windows ones yet. `tools/ci_mac.sh` (through
`gh`, signed in) waits for the job, says what it did and puts the pictures in
`.tmp/ci/mac-dark`: the way to look at a page drawn on a Mac from a machine that is none.

## The shell (M3, as built)

`quadra-shell/src/mac` (16, "On macOS") gives on macOS every service the shell gives on
Windows, under the same names: AppKit's panels and alert for the dialogs, the system's
`open` for opening, the alert sound, one instance over a socket, an item in the menu bar
for the tray, notifications, the app's kinds of file (Launch Services), what a process
used, a shell's process, sealing, hashing and passwords (CommonCrypto, in the form Windows
writes), fetching (the system's `curl`), the console, and the log's last lines beside the
system's report of a crash. `quadra_shell::install` gives an app's views the same services
as on Windows, so the app `markdown` builds for macOS and its tests run on the runner, and
the updater checks, fetches and verifies there. The shell's tests that ask nobody are the
same on both systems and run on the runner; what shows something to the user (a panel, an
alert, what `open` opens, a notification) is tried by hand on a real Mac. Markdown's
Settings say what Set as default does on a Mac (the app makes itself the one that opens
`.md`, from its bundle) and what draws it there.

The tool that draws Markdown's screens (`bench/markdown-shots`) draws with the system it
runs on, as the gallery's does: the CI's job draws them on a Mac after the gallery's pages
and keeps them with the run, so the app is seen as Metal and Core Text draw it.

### The app bundle

`tools/package_mac.sh` makes an app's bundle (16, "On macOS"); the CI's job makes
Markdown's, with the binary the tests built, and so checks its `Info.plist` and its
signature. The application's delegate takes the files the system asks it to open
(`application:openURLs:`) and gives their paths to who listens
(`quadra_native_mac::on_open_files`; what is asked before anyone listens is kept), which
is the shell's first instance. A path taken from a URL (here, a drop, a panel) has its
accented letters whole (`file_path`): the system writes a file's URL as the file system
writes names, with those letters taken apart, and takes either form for the same file. Tested by sending the delegate what AppKit sends
(`tests/opens.rs`); a file opened from the Finder is tried by hand.

## The terminal (M4, started)

`quadra-term` with its `pty` feature builds for macOS: the local shell is behind the POSIX
pty of `oneterm-vt`, on the same poller and thread as the pseudo console of Windows.
`tools/check_mac.sh` checks the feature, which nothing else built for macOS turns on, and
on a Mac runs its test (a command through `sh`). Keys are a Mac's (18, "Input"). Not yet:
a terminal in an app built for macOS (the gallery has none on either system), and the
updater's app bundle.

## What a real Mac is asked

The runner has no user: what follows is tried by hand, once, on a real Mac, and what it
finds is fixed there. Until then each is built and tested as far as the runner goes, and
only what its screen showed (above, "What the runner's screen shows") is known to work.

- A window that shows: the gallery (`cargo run -p quadra-gallery`) and Markdown from its
  bundle (`tools/package_mac.sh markdown`, then the Finder). The system's three buttons
  over the title bar and what they do; the window dragged by the bar, zoomed by two presses
  on it, resized; the full screen and back (the bar takes the buttons' room, then gives it).
- Retina, and a window moved between displays of two scales.
- Keys: Command for shortcuts, the keys of text and of the editor, Control with a press as
  the other button; an input method (Vietnamese Telex, Japanese) with its candidates beside
  the caret.
- The menu bar: Hide, Quit (the windows are asked), Minimize, Zoom.
- The shell: the open and save panels, an alert, `open` of a file and of a link, the alert
  sound; a second start of an app that is one instance; a `.md` file opened from the
  Finder, and one dropped on the app's icon, with the app running and not. Set as default
  in Markdown's Settings, then a `.md` file opened from the Finder. An item in the menu
  bar: its picture, its menu, an item chosen, a notification. A fetch behind a proxy named
  in System Settings.
- A file and text dragged from another program onto a window.
- VoiceOver reading a window: the controls, a text input's caret and selection, an action.
- A window closed and opened again where it was, with a display taken away between.
- A local shell in a terminal view: Control with C, Command with C and V, Option.
- The budgets of 14 (M5): the time of a frame, memory, an idle app that does not wake.

What the port leaves, and why: an app's own menus in the menu bar (the keys of an Edit
menu are taken before the view sees them, and what a disabled item does with its key wants
a Mac to try); installing an update (the updater's app bundle, decided when an app ships
there); notifications under the app's own picture and a proxy found by a script (16, "On
macOS"); a Developer ID signature and notarization (the user's account).

## Milestones

| | Builds | Exit |
|---|---|---|
| **M0** The workspace builds for macOS | The facade takes its native crate by system; `tools/check_mac.sh`; the CI's job | `tools/check_mac.sh` passes on Windows; the job is green on a Mac |
| **M1** The same tests on a Mac | What the tests find that is Windows' alone above the traits (paths, the user's folders, line ends, the environment) | Every test of the packages built for macOS passes on the runner |
| **M2** The native crate | `quadra-native-mac`: Core Text, then Metal drawn without a window and compared with the D3D11 pictures of the same scenes, then AppKit windows and input | The gallery runs on a Mac; its pages match the Windows ones within the snapshot tolerance; the manual checks pass on a real Mac |
| **M3** The shell | `quadra-shell/src/mac`: dialogs, opening, one instance, crash reports, notifications; sealing and fetching | `markdown` runs on a Mac |
| **M4** The terminal and the updater | `quadra-term`'s session over the POSIX pty of `oneterm-vt`; the updater's app bundle | The terminal's tests pass on a Mac; a terminal runs in the gallery there |
| **M5** Measured | The budgets of 14 on a Mac, in `docs/benchmarks.md` | Each budget met or the gap explained |

Each ends as a phase does (00): its exit met, `docs/progress.md` updated.

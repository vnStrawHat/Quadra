# 10 — Platform

The OS boundary. Traits here name no operating system concept; whatever is specific to one OS
lives in that OS's native crate.

## Traits

```rust
pub trait Platform {
    fn run(&self, app: &mut dyn PlatformHandler) -> Result<()>;   // the message loop
    fn quit(&self);
    fn open_window(&self, options: &WindowOptions) -> Result<Box<dyn PlatformWindow>>;
    fn waker(&self) -> Waker;                    // Send: wakes the loop from any thread
    fn set_timer(&self, after: Duration, repeat: bool) -> TimerId;
    fn cancel_timer(&self, id: TimerId);
    fn now(&self) -> Instant;                    // virtual in the headless platform
    fn displays(&self) -> Vec<Display>;
    fn appearance(&self) -> Appearance;          // dark, accent, high contrast, text scale
    fn clipboard(&self) -> Rc<dyn Clipboard>;    // plain text
}

pub trait PlatformWindow {                       // dropping it closes the window
    fn id(&self) -> WindowId;
    fn handle(&self) -> WindowHandle;            // what a renderer needs to make a surface
    fn size(&self) -> Size;                      // logical pixels
    fn scale(&self) -> f32;
    fn placement(&self) -> Placement;
    fn set_title(&self, title: &str);
    fn set_cursor(&self, cursor: Cursor);
    fn request_frame(&self);
    fn set_text_input(&self, caret: Option<Rect>);   // text is typed there, or nowhere
    fn update_semantics(&self, update: &TreeUpdate); // for assistive technology (12)
    fn minimize(&self);  fn toggle_maximize(&self);  fn is_maximized(&self) -> bool;
    fn controls(&self) -> WindowControls;        // who draws the window's buttons
    fn to_front(&self);  // shown again if minimized, and given the keyboard
}

pub trait PlatformHandler {
    fn event(&mut self, window: WindowId, event: PlatformEvent);
    fn frame(&mut self, window: WindowId);
    fn timer(&mut self, id: TimerId);
    fn wake(&mut self);                          // the waker fired: run posted work
    fn hit_test(&mut self, window: WindowId, at: Point) -> HitRegion;   // custom titlebar
}
```

- `WindowHandle` is an enum with one variant per windowing system (`Win32 { hwnd }`,
  `Headless`), defined in `quadra-platform`. No untyped pointer crosses the boundary.
- `PlatformEvent` covers pointer, wheel, key, text and composition, focus, resize (one event
  carries the size and the scale: a change of either is the same work), close request,
  appearance change, file drop, and assistive technology's two: `SemanticsWanted` (it started
  reading the window) and `Semantic { node, action }` (it asks a node to do something). The
  types of the semantic tree come from `quadra-a11y`, which has no OS code.
- **Every method takes `&self`.** The handler calls the platform and its windows from inside
  `run`, so nothing can hold them exclusively; implementations keep their state in cells. All of
  it lives on the UI thread; only `Waker` is `Send`.
- **The handler is never entered twice.** An OS call made by a handler can make the OS deliver
  an event at once (setting a title, moving a window). The platform queues such an event and
  delivers it when the handler returns. `hit_test`, which must answer at once, answers
  `Client` in that case.
- **A minimum size.** `WindowOptions::min_size(w, h)` is the smallest content size, in logical
  pixels, the user can drag the window to. A window asked to open smaller, by `size`, `fit` or a
  remembered placement, opens at the minimum; the minimum wins over `fit`. The headless
  platform holds `resize` at it, and the `Resize` event carries the size held.
- **Windows with their own frame.** `WindowOptions::custom_frame()` asks for a window whose
  content covers where the OS would draw a caption: the app draws the title bar. The OS
  keeps moving, resizing and snapping the window, and for that asks what a point is:
  `HitRegion::Caption` (the window is dragged by it, a double click maximizes it),
  `Maximize` (the app's maximize button: the OS shows its snap layouts over it), or
  `Client`, the app's own. The app answers from the window's tree: a node is given a region
  (`.window_region(..)` on a box, `Tree::set_window_region`), and a point is the region of
  the deepest node under it that has one, unless a node that takes the pointer (a handler
  for it, the focus) is deeper: a button in the bar stays a button, with nothing to say.
  Over `Caption` and `Maximize` the pointer still reaches the app as moves, and a press and
  release on `Maximize` as a press and release: the button shows them and its click is the
  app's to act on. Minimize and close are buttons of the app's like any other.
  **Who draws the buttons** is the system's to say (`PlatformWindow::controls`):
  `WindowControls::App` on Windows, where a title bar draws them at its right end;
  `System { leading }` on macOS, where the system draws its three over the content's left
  end and a title bar leaves them `leading` logical pixels. The app gives it to the
  window's views as context; a platform that opens no window of the system's (the headless
  one) says the app draws them, so that a test says the same on every machine, until it
  is told to be as the system it is built for (`HeadlessPlatform::as_the_system`, for the
  tool that draws pictures: a picture drawn on a Mac is a Mac's).
- **Modifiers.** `Mods::CTRL` is the key shortcuts are held with: Ctrl on Windows, Command
  on a Mac. `Mods::SUPER` is the system's other one: the Windows key, and Control on a Mac.
  A platform reports its keys so (as Qt does), and the keys themselves the same way
  (`Key::Ctrl` is a Command key on a Mac). So a binding written `ctrl-s`, a click with
  `CTRL` held that adds to a selection, a wheel with it that zooms, are what each system's
  user expects, with nothing above the platform asking which system it is. What needs the
  Control key itself on a Mac (a terminal's control characters) reads `SUPER` there.
- **How keys are written** is the system's too (`Platform::key_names`): `KeyNames::Words`
  ("Ctrl-Shift-P") or `KeyNames::Signs`, the Mac's ("⇧⌘P"). The app gives it to views as
  context, and what shows a shortcut follows it (13, `key_label`); the headless platform
  says words until it is told to be as the system.
- **What a view asks of its window.** `quadra_view::window()` gives the window the calling
  view is in: `.minimize()`, `.toggle_maximize()`, `.close()` (as the close button of the
  OS: the app decides), `.set_title(words)` (what the OS shows of it: its frame, the taskbar),
  `.to_front()` (shown again if it was minimized, and brought to the front with the
  keyboard, as far as the OS lets a process that is not in front: one instance, 16), and
  `.is_maximized()`, `.is_active()` (it has the keyboard) and
  `.is_minimized()`, reads that what they are in runs again for;
  `.frames()` counts the frames its tree made (`Handle::frames`, not a signal: read twice a
  second apart, the frames a second, and reading it makes none).
  The asks wait on the tree (`Tree::take_window_requests`) and the app does them after the
  handler that asked; the tree is told of the window's state with each resize.
- **The system's appearance** (`Platform::appearance()`): whether apps are asked to be
  dark, whether high contrast is on, and the accent colour the user chose, if there is
  one. When it changes every window gets `PlatformEvent::Appearance` with the new one. The
  app keeps it as a signal in the context of views (`Signal<Appearance>`), and
  `App::theme_by(|appearance| theme)` makes the app's theme follow it (the kit's
  `install_system(app)` takes its dark or light theme so). The headless platform's is set
  by its test (`set_appearance`), which delivers the event.
- **Drops from the OS**: files or text dragged from another program over a window arrive as
  `DragEnter { at, what }` (`Dropped::Files(paths)` or `Dropped::Text(text)`; files when
  there are both), `DragOver { at }`, `DragLeave` and `Drop { at }`. The tree treats them as
  a drag inside the app whose payload is a `DroppedFiles` or a `DroppedText` (09, "Drag and
  drop"): the deepest node under the pointer that takes that type is the target, has
  `drop_target`, is told where the pointer is, and gets the value on the drop. The OS is
  told a copy is taken wherever the window is (the target is the app's to find, after the
  OS asked).
- A window is owned by whoever holds its box. Dropping it closes the OS window; a close the
  user asks for arrives as an event, and the app decides.
- Errors are returned, not printed (01, R18).
- The app provides the platform's clipboard as context to each window's view; a text input
  takes it from there. No test reaches the real clipboard: the headless platform's is a
  variable, and the Win32 one is never run by a test.

## What is not in `Platform`

The V `Platform` interface also carried pseudo terminals, the tray, notifications, message
boxes, file dialogs, single-instance handling and crash dumps. They are separate here:

| Service | Lives in | Why |
|---|---|---|
| File and folder dialogs, message boxes | `quadra-shell`, trait `Dialogs` | An app without dialogs links none |
| Tray icon, notifications | `quadra-shell`, trait `Tray` | Same |
| Single instance and argument forwarding | `quadra-shell` | App start-up policy, not windowing |
| Crash reports | `quadra-shell` | Same |
| Open a URL or file, reveal in the file manager | `quadra-shell`, trait `Opener` | Same |
| Pseudo terminal | `quadra-term`, trait `Pty` with a ConPTY implementation | Only a terminal needs it |
| Accessibility | `quadra-a11y`; the OS bridge in the native crate (12) | Driven by the tree, not by the window |
| Image decoding | trait `ImageDecoder` (`info` from the header, `decode` at a size, `frames` for an image that moves, premultiplied RGBA; both as the image is seen, so a photograph whose orientation tag turns it is measured and decoded turned, `Decoded::oriented`), called on the pool; `Platform::image_decoder` gives the system codecs in the native crate, and a decoder of flat test images (`FlatImages`) on the headless platform; `admit` holds a header to the limits (11) | No decoder of untrusted files to own or to depend on |

Each trait has a headless implementation that records calls and returns what a test queued.

## The native crate

`quadra-native-win` implements `Platform`, `Renderer` and `TextBackend` for Windows and builds
an app from them. It is the one place where the three may know each other:

- **Frame pacing.** The swap chain's frame-latency waitable is waited on in the message loop
  together with messages. The platform trait has no method for it; the native crate hands the
  handle from its renderer to its loop. A wait takes a frame from the waitable and only a
  present gives one back, so a wait not followed by a present would leave the window waiting
  for ever, drawing nothing while its input and its work go on. So a minimized window is not
  waited for nor drawn (the app draws nothing at a size of zero); its request stays until it
  is restored. And a window that has waited 100 ms draws anyway, timed from when it began
  to wait, however many messages wake the loop meanwhile: that present gives back a frame a
  failed draw took.
- **Device start-up.** The window opens, the first frame is drawn on the software device and
  blitted, and the swap chain is attached when the hardware device is ready (07).
- **Device loss and display changes** reach both the renderer and the windows.

A second OS adds a second native crate. Nothing above the traits changes but the types
that name a system's window: `WindowHandle` and `SurfaceTarget` have a variant a system. The
facade takes the native crate of the system it is built for; macOS is the second (23).

## Win32 behaviour

As behaviour/02 notes it (first written for the V implementation):

- Per-monitor DPI awareness v2; a scale change resizes the window as the OS suggests.
- **A window's icon** is the executable's icon resource 1, where it has one, at the system's
  sizes for an icon and a small icon (the title bar, the taskbar, Alt+Tab); without one the
  OS's. An app gives its executable an icon at build time (16, "Packaging"), so there is one
  icon for the file, its windows and the files it opens.
- A minimum size is answered in `WM_GETMINMAXINFO`: the minimum content times the window's DPI
  of the moment (read at each message, so right after a DPI change), plus the frame
  `AdjustWindowRectExForDpi` gives for the window's style. With a frame of its own only the
  borders that resize are added, as the content covers the caption, so the minimum is the
  content the user sees.
- The modal move and resize loop: frames keep being drawn while the user drags a border, and
  content stays in step with the window edge. The loop turns from the press on the caption (or
  a border, or the window menu's Move and Size) until the button is up, also while the
  pointer holds still: through all of it a timer of the window draws what is pending, paced
  as the frame loop is, so a terminal's output and an animation go on.
- Custom titlebar: the app draws it; the OS keeps resizing, snapping and the snap layouts
  over the maximize button through `hit_test`. `WM_NCCALCSIZE` gives the caption's room to
  the content (less the border a maximized window hangs over the screen by); `WM_NCHITTEST`
  keeps the borders that resize, a band as tall as one at the top of the content included,
  and asks the handler for the rest; the frame's pointer messages over the bar and the
  maximize button become the app's pointer events (`frame.rs`).
- IME through the text services of the OS, with composition reported to the focused input;
  Vietnamese typing is a required test case.
- Windows open at a fitted, centred or restored placement; a restored placement is used only if
  it is still on a connected display.
- The window answers `WM_SETCURSOR` over its content with the cursor it was given, one of the
  system's own; over its frame the system keeps its cursors. A cursor given while the pointer
  is over the content shows at once.
- Assistive technology through UI Automation, by the adapter of `accesskit` (12): made with
  the window, it loads nothing and builds nothing until a reader asks.
- OS bindings come from the `windows` crate with only the needed features. COM objects are
  owned by its smart pointers; no vtable is indexed by hand. One object is Quadra's own: the
  window's drop target (`drop.rs`), an `IDropTarget` written as its vtable and a count,
  since the framework has no procedural macros to make one. It is registered with the
  window (`RegisterDragDrop`, OLE started on the UI thread) and revoked when it closes; it
  reads `CF_HDROP` and `CF_UNICODETEXT` from what is dragged.
- The appearance is read from the user's settings (`AppsUseLightTheme`, the accent colour
  of the window manager, `SPI_GETHIGHCONTRAST`) and read again when the OS says they
  changed (`WM_SETTINGCHANGE` for `ImmersiveColorSet`, `WM_SYSCOLORCHANGE`).

## The headless platform

In `quadra-platform`, used by `quadra-testing` and by benches:

- Windows with a size and scale, no OS window.
- A virtual clock: `now` returns it, timers fire when a test advances it, in order.
- An event queue a test fills; `run` is replaced by stepping.
- A clipboard that is a variable in memory. The real clipboard is never touched by a test.
- A window keeps the semantic updates it is given; a test queues `SemanticsWanted` to be the
  reader.
- A window remembers that it was minimized and whether it is maximized; a test asks the
  app's `hit_test` itself.

It is the second implementation of every trait above, which is what keeps the traits honest
until a second OS exists.

## App start-up

`quadra-app` builds on the traits:

```rust
fn main() -> quadra::Result<()> {
    quadra::app()?                        // native backends for this OS
        .window(WindowOptions::new("Notes").size(960.0, 600.0), app_view)?
        .run()
}
```

`App` owns the platform, renderer, text system, the reactive runtime's root scope and the
windows. Making the backends and opening a window can fail, and say so. A window's view is
mounted in a scope of its own, which ends when the window closes; the last window closing
ends the loop. It implements `PlatformHandler`: events go to dispatch (09), frames to the passes
(02), timers and wakes to the task executor (11).

The app's **error hook** receives what the framework could not handle: a lost device that could
not be recreated, a failed present, a pass loop, an effect cycle. The default logs it; an app
may show it.

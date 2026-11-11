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
    fn set_text_input(&self, state: Option<TextInputState>);   // accepts text, caret rect
    fn minimize(&self);  fn toggle_maximize(&self);
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
  appearance change, file drop.
- **Every method takes `&self`.** The handler calls the platform and its windows from inside
  `run`, so nothing can hold them exclusively; implementations keep their state in cells. All of
  it lives on the UI thread; only `Waker` is `Send`.
- **The handler is never entered twice.** An OS call made by a handler can make the OS deliver
  an event at once (setting a title, moving a window). The platform queues such an event and
  delivers it when the handler returns. `hit_test`, which must answer at once, answers
  `Client` in that case.
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
| Image decoding | trait `ImageDecoder`; the system codecs in the native crate | No decoder of untrusted files to own or to depend on |

Each trait has a headless implementation that records calls and returns what a test queued.

## The native crate

`quadra-native-win` implements `Platform`, `Renderer` and `TextBackend` for Windows and builds
an app from them. It is the one place where the three may know each other:

- **Frame pacing.** The swap chain's frame-latency waitable is waited on in the message loop
  together with messages. The platform trait has no method for it; the native crate hands the
  handle from its renderer to its loop.
- **Device start-up.** The window opens, the first frame is drawn on the software device and
  blitted, and the swap chain is attached when the hardware device is ready (07).
- **Device loss and display changes** reach both the renderer and the windows.

A second OS adds a second native crate. Nothing above the traits changes.

## Win32 behaviour

Unchanged from the V implementation, whose notes in behaviour/02 remain the reference:

- Per-monitor DPI awareness v2; a scale change resizes the window as the OS suggests.
- The modal move and resize loop: frames keep being drawn while the user drags a border, and
  content stays in step with the window edge.
- Custom titlebar: the app draws it; the OS keeps resizing, snapping and the window buttons'
  behaviour through `hit_test`, including snap layouts over the maximize button.
- IME through the text services of the OS, with composition reported to the focused input;
  Vietnamese typing is a required test case.
- Windows open at a fitted, centred or restored placement; a restored placement is used only if
  it is still on a connected display.
- OS bindings come from the `windows` crate with only the needed features. COM objects are
  owned by its smart pointers; no vtable is indexed by hand.

## The headless platform

In `quadra-platform`, used by `quadra-testing` and by benches:

- Windows with a size and scale, no OS window.
- A virtual clock: `now` returns it, timers fire when a test advances it, in order.
- An event queue a test fills; `run` is replaced by stepping.
- A clipboard that is a variable in memory. The real clipboard is never touched by a test.

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

//! The AppKit platform with a real window that is not shown: what the loop gives the
//! handler (timers, wakes, frames), what AppKit's events arrive as, and a frame drawn into
//! the window's layer by the Metal renderer. A `main` of its own: AppKit runs on the main
//! thread alone, which the tests' harness keeps for itself.

#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    mac::run();
    // As the harness says it, for who counts what passed.
    println!("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out");
}

#[cfg(target_os = "macos")]
mod mac {
    use std::time::Duration;

    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{MainThreadMarker, msg_send};
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
        NSApplication, NSEvent, NSEventModifierFlags, NSEventType, NSView, NSWindow,
    };
    use objc2_foundation::{NSNotFound, NSPoint, NSRange, NSRect, NSSize, NSString};
    use quadra_native_mac::{MacPlatform, MetalRenderer};
    use quadra_platform::{
        Cursor, Key, Mods, MouseButton, Platform, PlatformEvent, PlatformHandler, PlatformWindow,
        PointerKind, TimerId, WindowControls, WindowHandle, WindowId, WindowOptions,
    };
    use quadra_prim::{Color, Point, Rect, Size};
    use quadra_render::{Clip, DeviceSize, Quad, Renderer, Scene, SurfaceTarget};

    /// What the loop gave, in order, and what the test does at each timer.
    struct Seen {
        platform: MacPlatform,
        window: Box<dyn PlatformWindow>,
        events: Vec<PlatformEvent>,
        timers: u32,
        frames: u32,
        wakes: u32,
        /// The timer that repeats, the one that fires once.
        ticks: (TimerId, TimerId),
    }

    /// Says an exception of Objective-C and where it was raised, before it ends the process:
    /// Rust, which cannot catch one, says neither.
    unsafe extern "C" fn said(exception: *mut AnyObject) -> *mut AnyObject {
        // SAFETY: an exception is an object, and every object answers `description`.
        let what: Retained<NSString> = unsafe { msg_send![exception, description] };
        eprintln!("an exception of Objective-C: {what}");
        eprintln!("{}", std::backtrace::Backtrace::force_capture());
        exception
    }

    /// The window's view and its own object.
    fn ns_window(window: &dyn PlatformWindow) -> (&NSView, Retained<NSWindow>) {
        let WindowHandle::AppKit { view } = window.handle() else { panic!("not an AppKit window") };
        // SAFETY: the address is of the window's content view, alive as long as the window.
        let view = unsafe { &*(view as *const NSView) };
        (view, view.window().expect("the view is in its window"))
    }

    /// What an input method does: it composes, asks where the caret is, and commits.
    fn compose(view: &NSView) {
        let (composed, committed) = (NSString::from_str("việ"), NSString::from_str("việt"));
        let nowhere = NSRange::new(NSNotFound as usize, 0);
        // SAFETY: messages of `NSTextInputClient`, which the view answers, with what they take.
        unsafe {
            let _: () = msg_send![
                view,
                setMarkedText: &*composed,
                selectedRange: NSRange::new(3, 0),
                replacementRange: nowhere
            ];
            let composing: bool = msg_send![view, hasMarkedText];
            assert!(composing);
            let caret: NSRect = msg_send![
                view,
                firstRectForCharacterRange: NSRange::new(0, 3),
                actualRange: std::ptr::null_mut::<NSRange>()
            ];
            assert_eq!((caret.size.width, caret.size.height), (2.0, 16.0));
            let _: () = msg_send![view, insertText: &*committed, replacementRange: nowhere];
        }
    }

    /// A press and a release of the left button, keys that type "a" and "b", a composition
    /// of an input method, the other appearance
    /// (dark, of a light one), a new size and a request to close, as AppKit gives them: from
    /// inside the handler, so each arrives after it. The key goes through the window; the press is given to the view itself,
    /// as a window that is not shown passed none on (seen on GitHub's Mac).
    fn post(shown: &dyn PlatformWindow, dark: bool) {
        let (view, window) = ns_window(shown);
        let number = window.windowNumber();
        // In the window's points, from its bottom left: 30 from the left, 40 from the top of
        // a content 200 high.
        let at = NSPoint::new(30.0, 160.0);
        let none = NSEventModifierFlags::empty();
        let presses = [NSEventType::LeftMouseDown, NSEventType::LeftMouseUp];
        let held = [NSEventModifierFlags::Shift, NSEventModifierFlags::Control];
        for (held, kind) in held.into_iter().flat_map(|held| presses.map(|kind| (held, kind))) {
            let press = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                kind, at, held, 0.0, number, None, 0, 1, 1.0,
            );
            let press = press.expect("a mouse event");
            match kind {
                NSEventType::LeftMouseDown => view.mouseDown(&press),
                _ => view.mouseUp(&press),
            }
        }
        // The first key with nothing that takes text, the second with a caret: through the
        // input method then. Its context is made active here, as AppKit does in the window
        // that has the keys, which one that is not shown never is.
        for (typed, code) in [("a", 0), ("b", 11)] {
            if code == 11 {
                shown.set_text_input(Some(Rect::new(10.0, 20.0, 2.0, 16.0)));
                view.inputContext().expect("a view that takes text").activate();
            }
            let typed = NSString::from_str(typed);
            let key = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
                NSEventType::KeyDown, at, none, 0.0, number, None, &typed, &typed, false, code,
            );
            window.sendEvent(&key.expect("a key event"));
        }
        compose(view);
        shown.set_text_input(None);
        // SAFETY: AppKit's constants, read and never written.
        let name = unsafe { if dark { NSAppearanceNameDarkAqua } else { NSAppearanceNameAqua } };
        window.setAppearance(NSAppearance::appearanceNamed(name).as_deref());
        window.setContentSize(NSSize::new(400.0, 240.0));
        // The window's own close, then the menu bar's Quit, which asks the window too and
        // ends no process.
        window.performClose(None);
        NSApplication::sharedApplication(MainThreadMarker::new().unwrap()).terminate(None);
    }

    impl PlatformHandler for Seen {
        fn event(&mut self, _: WindowId, event: PlatformEvent) {
            self.events.push(event);
        }

        fn frame(&mut self, _: WindowId) {
            self.frames += 1;
        }

        fn timer(&mut self, id: TimerId) {
            self.timers += 1;
            if id == self.ticks.1 {
                // The one that fires once: the events, a frame asked for, a wake from another
                // thread.
                post(&*self.window, !self.platform.appearance().dark);
                self.window.request_frame();
                let waker = self.platform.waker();
                std::thread::spawn(move || waker.wake()).join().unwrap();
            } else if self.timers >= 4 {
                self.platform.cancel_timer(self.ticks.0);
                self.platform.quit();
            }
        }

        fn wake(&mut self) {
            self.wakes += 1;
        }
    }

    pub fn run() {
        assert!(MainThreadMarker::new().is_some(), "the test's main is the main thread");
        // SAFETY: a function that reads the exception it is given and hands it on.
        unsafe { objc2::ffi::objc_setExceptionPreprocessor(said) };
        let platform = MacPlatform::new().unwrap();
        let window =
            platform.open_window(&WindowOptions::new("Quadra test").size(320.0, 200.0).hidden()).unwrap();
        assert_eq!(window.size(), Size::new(320.0, 200.0));
        assert!(window.scale() >= 1.0);
        // The system's three buttons, and the room a title bar leaves them.
        let controls = window.controls();
        assert!(
            matches!(controls, WindowControls::System { leading } if (50.0..=120.0).contains(&leading)),
            "{controls:?}"
        );
        assert!(platform.utc_offset(0).abs() <= 14 * 3600);
        let looks = platform.appearance();
        assert!(looks.accent.is_some() && looks.text_scale == 1.0, "{looks:?}");
        // Every shape has a cursor.
        for shape in [
            Cursor::Text,
            Cursor::Pointer,
            Cursor::Crosshair,
            Cursor::Move,
            Cursor::NotAllowed,
            Cursor::ResizeHorizontal,
            Cursor::ResizeVertical,
            Cursor::ResizeNwse,
            Cursor::ResizeNesw,
            Cursor::Wait,
            Cursor::Progress,
            Cursor::Help,
            Cursor::Arrow,
        ] {
            window.set_cursor(shape);
        }

        // A frame into the window's layer: drawn and shown without an error, on a window
        // that is on no display (there may be nothing to draw into then, which is no error).
        let mut renderer = MetalRenderer::new().unwrap();
        let WindowHandle::AppKit { view } = window.handle() else { panic!("not an AppKit window") };
        let size = DeviceSize { width: 320, height: 200 };
        let surface = renderer.create_surface(SurfaceTarget::AppKit { view }, size).unwrap();
        let mut scene = Scene::new();
        let root = scene.add_layer();
        scene.set_roots(&[root]);
        let quad = Quad {
            bounds: Rect::new(10.0, 10.0, 50.0, 50.0),
            background: Color::rgb(0xFF0000),
            ..Quad::default()
        };
        scene.layer_mut(root).set(&[quad.instance(&Clip::NONE)]);
        for _ in 0..2 {
            renderer.draw(surface, &mut scene, Color::WHITE).unwrap();
            renderer.present(surface).unwrap();
        }
        renderer.resize_surface(surface, DeviceSize { width: 400, height: 240 }).unwrap();
        renderer.draw(surface, &mut scene, Color::WHITE).unwrap();
        renderer.present(surface).unwrap();
        assert!(renderer.read_pixels(surface).is_err(), "a window is not read back");
        renderer.drop_surface(surface);

        let ticks = (
            platform.set_timer(Duration::from_millis(20), true),
            platform.set_timer(Duration::from_millis(5), false),
        );
        let mut seen = Seen {
            platform: platform.clone(),
            window,
            events: Vec::new(),
            timers: 0,
            frames: 0,
            wakes: 0,
            ticks,
        };
        platform.run(&mut seen).unwrap();

        assert!(seen.timers >= 4, "{} timers", seen.timers);
        assert_eq!((seen.frames, seen.wakes), (1, 1), "a frame asked for once, a wake");
        let events = &seen.events;
        let said = format!("{events:#?}");
        let at = Point::new(30.0, 40.0);
        let press = |kind| PlatformEvent::Pointer { kind, button: MouseButton::Left, at, mods: Mods::SHIFT };
        let place = |wanted: &PlatformEvent| events.iter().position(|it| it == wanted);
        let (down, up) = (place(&press(PointerKind::Down)), place(&press(PointerKind::Up)));
        assert!(down.is_some() && down < up, "a press and a release, from the top left: {said}");
        // With Control held the press is the right button's, and Control is `SUPER`.
        let other = |kind| PlatformEvent::Pointer { kind, button: MouseButton::Right, at, mods: Mods::SUPER };
        let (other_down, other_up) = (place(&other(PointerKind::Down)), place(&other(PointerKind::Up)));
        assert!(up < other_down && other_down < other_up, "a press with Control: {said}");
        let key = PlatformEvent::Key { key: Key::A, code: 0, mods: Mods::NONE, down: true, repeat: false };
        let (key, text) = (place(&key), place(&PlatformEvent::Text("a".to_string())));
        assert!(up < key && key < text, "the key, then what it types: {said}");
        // What the keys typed and the input method composed: the caret in bytes of the text.
        let composing =
            |text: &str, caret| PlatformEvent::Composition { text: text.to_string(), caret, target: 0..0 };
        let wanted = [
            PlatformEvent::Text("a".to_string()),
            PlatformEvent::Text("b".to_string()),
            composing("việ", 5),
            PlatformEvent::Text("việt".to_string()),
            composing("", 0),
        ];
        let texts = events
            .iter()
            .filter(|it| matches!(it, PlatformEvent::Text(_) | PlatformEvent::Composition { .. }));
        assert!(texts.eq(wanted.iter()), "typed and composed: {said}");
        let sizes: Vec<Size> = events
            .iter()
            .filter_map(|it| if let PlatformEvent::Resize { size, .. } = it { Some(*size) } else { None })
            .collect();
        assert_eq!(sizes, [Size::new(320.0, 200.0), Size::new(400.0, 240.0)], "each size once: {said}");
        let turned =
            events.iter().any(|it| matches!(it, PlatformEvent::Appearance(now) if now.dark != looks.dark));
        assert!(turned, "the window's other appearance: {said}");
        let asked = events.iter().rev().take_while(|it| **it == PlatformEvent::CloseRequested).count();
        assert_eq!(asked, 2, "the window's close, then Quit: {said}");
        let bar = NSApplication::sharedApplication(MainThreadMarker::new().unwrap()).mainMenu();
        assert_eq!(bar.map(|bar| bar.numberOfItems()), Some(2), "the app's menu and the window's");
        assert_eq!(seen.window.size(), Size::new(400.0, 240.0));
    }
}

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

    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSEvent, NSEventModifierFlags, NSEventType, NSView, NSWindow};
    use objc2_foundation::{NSPoint, NSSize, NSString};
    use quadra_native_mac::{MacPlatform, MetalRenderer};
    use quadra_platform::{
        Key, Mods, MouseButton, Platform, PlatformEvent, PlatformHandler, PlatformWindow, PointerKind,
        TimerId, WindowHandle, WindowId, WindowOptions,
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

    /// The window's own object, from its view.
    fn ns_window(window: &dyn PlatformWindow) -> objc2::rc::Retained<NSWindow> {
        let WindowHandle::AppKit { view } = window.handle() else { panic!("not an AppKit window") };
        // SAFETY: the address is of the window's content view, alive as long as the window.
        let view = unsafe { &*(view as *const NSView) };
        view.window().expect("the view is in its window")
    }

    /// Sends the window a press and a release of the left button, a key that types "a", and
    /// a request to close, as AppKit would: from inside the handler, so each arrives after it.
    fn post(window: &dyn PlatformWindow) {
        let window = ns_window(window);
        let number = window.windowNumber();
        // In the window's points, from its bottom left: 30 from the left, 40 from the top of
        // a content 200 high.
        let at = NSPoint::new(30.0, 160.0);
        let none = NSEventModifierFlags::empty();
        for kind in [NSEventType::LeftMouseDown, NSEventType::LeftMouseUp] {
            let press = NSEvent::mouseEventWithType_location_modifierFlags_timestamp_windowNumber_context_eventNumber_clickCount_pressure(
                kind, at, NSEventModifierFlags::Shift, 0.0, number, None, 0, 1, 1.0,
            );
            window.sendEvent(&press.expect("a mouse event"));
        }
        let a = NSString::from_str("a");
        let key = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            NSEventType::KeyDown, at, none, 0.0, number, None, &a, &a, false, 0,
        );
        window.sendEvent(&key.expect("a key event"));
        window.setContentSize(NSSize::new(400.0, 240.0));
        window.performClose(None);
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
                post(&*self.window);
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
        let platform = MacPlatform::new().unwrap();
        let window =
            platform.open_window(&WindowOptions::new("Quadra test").size(320.0, 200.0).hidden()).unwrap();
        assert_eq!(window.size(), Size::new(320.0, 200.0));
        assert!(window.scale() >= 1.0);
        assert!(platform.utc_offset(0).abs() <= 14 * 3600);

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
        let key = PlatformEvent::Key { key: Key::A, code: 0, mods: Mods::NONE, down: true, repeat: false };
        let (key, text) = (place(&key), place(&PlatformEvent::Text("a".to_string())));
        assert!(up < key && key < text, "the key, then what it types: {said}");
        let resized = events.iter().any(|it| {
            matches!(it, PlatformEvent::Resize { size, scale } if *size == Size::new(400.0, 240.0) && *scale >= 1.0)
        });
        assert!(resized, "the content's new size: {said}");
        assert_eq!(events.last(), Some(&PlatformEvent::CloseRequested), "{said}");
        assert_eq!(seen.window.size(), Size::new(400.0, 240.0));
    }
}

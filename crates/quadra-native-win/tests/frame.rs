//! A window with its own frame (specs/10, "Windows with their own frame"), on a window that
//! is never shown: the content covers the caption, and the OS is told what the app says each
//! point is. The questions come from another thread, as the OS's own do from outside a
//! handler.

use quadra_native_win::WinPlatform;
use quadra_platform::{
    HitRegion, MouseButton, Platform, PlatformEvent, PlatformHandler, PointerKind, TimerId, WindowHandle,
    WindowId, WindowOptions,
};
use quadra_prim::{Point, Size};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::WindowsAndMessaging::{
    GetClientRect, GetWindowRect, HTCAPTION, HTCLIENT, HTMAXBUTTON, HTTOP, PostMessageW, SendMessageW,
    WM_NCHITTEST, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP, WM_NCMOUSEMOVE,
};

/// A bar of 32 px: it drags the window left of 200, is the app's up to 300, and from there
/// the maximize button.
struct Bar {
    platform: WinPlatform,
    pointer: Vec<(PointerKind, MouseButton, Point)>,
}

impl PlatformHandler for Bar {
    fn event(&mut self, _: WindowId, event: PlatformEvent) {
        if let PlatformEvent::Pointer { kind, button, at, .. } = event
            && kind != PointerKind::Leave
        {
            self.pointer.push((kind, button, at));
        }
    }

    fn frame(&mut self, _: WindowId) {}

    fn timer(&mut self, _: TimerId) {}

    fn wake(&mut self) {
        self.platform.quit();
    }

    fn hit_test(&mut self, _: WindowId, at: Point) -> HitRegion {
        match (at.x, at.y) {
            (_, y) if y >= 32.0 => HitRegion::Client,
            (x, _) if x >= 300.0 => HitRegion::Maximize,
            (x, _) if x >= 200.0 => HitRegion::Client,
            _ => HitRegion::Caption,
        }
    }
}

/// A point of the content, as the frame's messages carry it: on the screen.
fn on_screen(origin: POINT, scale: f32, x: f32, y: f32) -> isize {
    let (x, y) = (origin.x + (x * scale) as i32, origin.y + (y * scale) as i32);
    ((y as u16 as isize) << 16) | (x as u16 as isize)
}

#[test]
fn the_content_covers_the_caption_and_the_os_is_told_what_each_point_is() {
    let platform = WinPlatform::new().unwrap();
    let options = WindowOptions::new("frame").size(400.0, 300.0).hidden().custom_frame();
    let window = platform.open_window(&options).unwrap();
    let WindowHandle::Win32 { hwnd: raw } = window.handle() else { panic!("not a Win32 window") };
    let (hwnd, scale) = (HWND(raw as *mut _), window.scale());
    let (mut client, mut outer, mut origin) = (RECT::default(), RECT::default(), POINT::default());
    // SAFETY: questions about a window of this thread.
    unsafe {
        GetClientRect(hwnd, &mut client).unwrap();
        GetWindowRect(hwnd, &mut outer).unwrap();
        let _ = ClientToScreen(hwnd, &mut origin);
    }
    // The content is as large as asked, and starts where the window does: no caption.
    assert_eq!(window.size(), Size::new(400.0, 300.0));
    assert_eq!((origin.y, client.bottom), (outer.top, (300.0 * scale) as i32));
    assert!(!window.is_maximized());

    // The maximize button is pressed through the frame's messages: the app hears a move, a
    // press and a release where they were, in its own pixels.
    let over = on_screen(origin, scale, 350.0, 16.0);
    for msg in [WM_NCMOUSEMOVE, WM_NCLBUTTONDOWN, WM_NCLBUTTONUP] {
        // SAFETY: posting a message to a window of this thread.
        unsafe { PostMessageW(Some(hwnd), msg, WPARAM(HTMAXBUTTON as usize), LPARAM(over)).unwrap() };
    }
    let waker = platform.waker();
    let asked = std::thread::spawn(move || {
        let ask = |x: f32, y: f32| {
            let at = LPARAM(on_screen(origin, scale, x, y));
            // SAFETY: a message sent to a window of another thread waits for its answer.
            unsafe { SendMessageW(HWND(raw as *mut _), WM_NCHITTEST, Some(WPARAM(0)), Some(at)) }.0 as u32
        };
        let hits = [ask(50.0, 20.0), ask(250.0, 20.0), ask(350.0, 20.0), ask(50.0, 100.0), ask(50.0, 2.0)];
        waker.wake();
        hits
    });
    let mut bar = Bar { platform: platform.clone(), pointer: Vec::new() };
    platform.run(&mut bar).unwrap();
    // The bar, a button of the app's in it, the maximize button, the content, and the band
    // at the top that still resizes the window.
    assert_eq!(asked.join().unwrap(), [HTCAPTION, HTCLIENT, HTMAXBUTTON, HTCLIENT, HTTOP]);
    let at = Point::new((350.0 * scale).floor() / scale, (16.0 * scale).floor() / scale);
    assert_eq!(
        bar.pointer,
        [
            (PointerKind::Move, MouseButton::None, at),
            (PointerKind::Down, MouseButton::Left, at),
            (PointerKind::Up, MouseButton::Left, at)
        ]
    );
}

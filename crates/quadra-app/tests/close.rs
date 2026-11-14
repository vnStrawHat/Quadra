//! Closing (specs/15, "Closing"): what is in a window says whether the user may close it; a
//! view that said no closes it later itself, asking nothing.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use quadra_app::App;
use quadra_platform::{HeadlessPlatform, Platform, PlatformEvent, WindowOptions};
use quadra_render::NullRenderer;
use quadra_text::FakeBackend;
use quadra_ui::{Window, column, window};

#[test]
fn a_window_closes_when_what_is_in_it_says_it_may() {
    let platform = HeadlessPlatform::new();
    let mut app = App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()));
    let (saved, asked, handle) =
        (Rc::new(Cell::new(false)), Rc::new(Cell::new(0)), Rc::new(RefCell::new(None)));
    let (may, count, keep) = (saved.clone(), asked.clone(), handle.clone());
    let id = app
        .open_window(WindowOptions::new("doc"), move || {
            *keep.borrow_mut() = Some(window());
            column().on_close(move || {
                count.set(count.get() + 1);
                may.get() // not saved: the app would ask the user here
            })
        })
        .unwrap();
    platform.step(&mut app);
    let window: Window = handle.borrow().clone().unwrap();

    // The OS's close button, and the app's own: both ask.
    platform.push(id, PlatformEvent::CloseRequested);
    platform.step(&mut app);
    window.close();
    platform.waker().wake();
    platform.step(&mut app);
    assert_eq!((app.window_count(), asked.get()), (1, 2));

    // The user said to close without saving: nothing is asked.
    window.close_now();
    platform.waker().wake();
    platform.step(&mut app);
    assert_eq!((app.window_count(), asked.get()), (0, 2));

    // Saved: the window closes when asked.
    saved.set(true);
    let (count, may) = (asked.clone(), saved.clone());
    let id = app
        .open_window(WindowOptions::new("doc"), move || {
            column().on_close(move || {
                count.set(count.get() + 1);
                may.get()
            })
        })
        .unwrap();
    platform.push(id, PlatformEvent::CloseRequested);
    platform.step(&mut app);
    assert_eq!((app.window_count(), asked.get()), (0, 3));
}

//! Windows a view opens (specs/15, "Start and end"): opened after the handler that asks,
//! closed with the window they were asked from, and one kept under a name opened once.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_app::{App, Windows};
use quadra_platform::{HeadlessPlatform, Platform, WindowOptions};
use quadra_reactive::expect;
use quadra_render::NullRenderer;
use quadra_text::FakeBackend;
use quadra_view::{Window, column, window};

#[test]
fn a_view_opens_a_window_that_closes_with_its_own() {
    let platform = HeadlessPlatform::new();
    let mut app = App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()));
    let kept: Rc<RefCell<Option<(Windows, Window)>>> = Rc::default();
    let keep = kept.clone();
    app.open_window(WindowOptions::new("main"), move || {
        *keep.borrow_mut() = Some((expect::<Windows>(), window()));
        column()
    })
    .unwrap();
    platform.step(&mut app);
    let (windows, main) = kept.borrow().clone().unwrap();

    // Asked twice under one name: the second replaces the first, so one stands.
    for _ in 0..2 {
        windows.open(WindowOptions::new("settings").remember("settings"), column);
        platform.waker().wake();
        platform.step(&mut app);
    }
    windows.open(WindowOptions::new("about"), column);
    platform.waker().wake();
    platform.step(&mut app);
    assert_eq!(app.window_count(), 3);

    // The main window goes, and what it opened with it.
    main.close_now();
    platform.waker().wake();
    platform.step(&mut app);
    assert_eq!(app.window_count(), 0);
}

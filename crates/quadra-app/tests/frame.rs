//! A window with its own frame, on the headless platform (specs/10, "Windows with their own
//! frame"): the app says what each point of the window is to the OS, and does what a view
//! asks of its window.

use std::rc::Rc;

use quadra_app::App;
use quadra_platform::{
    HeadlessPlatform, HitRegion, Mods, MouseButton, PlatformEvent, PlatformHandler, PointerKind, WindowId,
    WindowOptions,
};
use quadra_prim::{Point, Size};
use quadra_render::NullRenderer;
use quadra_text::FakeBackend;
use quadra_view::{column, div, row, text, window};

/// A bar of 32 px that drags the window, with a menu button of the app's at its left and,
/// at its right, buttons that minimize, maximize and close. Under it, the content.
fn page() -> impl quadra_view::View {
    let window = window();
    let (min, max, close, state, is) =
        (window.clone(), window.clone(), window.clone(), window.clone(), window);
    let button = || div().size(40.0, 32.0);
    let bar = row()
        .height(32.0)
        .window_region(HitRegion::Caption)
        .child(button().on_click(|_| {})) // the app's own: it stays a button
        .child(div().grow(1.0).child(text("Title")))
        .child(button().on_click(move |_| min.minimize()))
        .child(button().window_region(HitRegion::Maximize).on_click(move |_| max.toggle_maximize()))
        .child(button().on_click(move |_| close.close()));
    let says = text(move || if state.is_maximized() { "maximized" } else { "restored" });
    // A minimized window makes no frame: what it hears is kept here.
    quadra_reactive::effect(move || HEARD.with(|heard| heard.set((is.is_active(), is.is_minimized()))));
    column().child(bar).child(div().grow(1.0).child(says))
}

thread_local! {
    /// Whether the window is active and minimized, as the view last heard.
    static HEARD: std::cell::Cell<(bool, bool)> = const { std::cell::Cell::new((false, false)) };
}

fn heard() -> (bool, bool) {
    HEARD.with(|heard| heard.get())
}

fn click(platform: &HeadlessPlatform, app: &mut App<NullRenderer>, window: WindowId, x: f32) {
    for (kind, button) in [
        (PointerKind::Move, MouseButton::None),
        (PointerKind::Down, MouseButton::Left),
        (PointerKind::Up, MouseButton::Left),
    ] {
        let at = Point::new(x, 16.0);
        platform.push(window, PlatformEvent::Pointer { kind, button, at, mods: Mods::NONE });
    }
    platform.step(app);
}

#[test]
fn the_app_says_what_a_point_is_and_does_what_a_view_asks_of_its_window() {
    let platform = HeadlessPlatform::new();
    let mut app = App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()));
    let options = WindowOptions::new("frame").size(400.0, 300.0).custom_frame();
    let window = app.open_window(options, page).unwrap();
    platform.step(&mut app);

    // The bar drags the window, text and all; a button in it is the app's; the maximize
    // button is the one the OS shows its snap layouts over; the content is the app's.
    let mut what = |x: f32, y: f32| app.hit_test(window, Point::new(x, y));
    assert_eq!(
        [what(20.0, 16.0), what(100.0, 16.0), what(300.0, 16.0), what(340.0, 16.0), what(100.0, 100.0)],
        [HitRegion::Client, HitRegion::Caption, HitRegion::Client, HitRegion::Maximize, HitRegion::Client]
    );

    // The buttons ask the window: it is maximized, and the view hears of it with the resize.
    let said = |app: &App<NullRenderer>, words: &str| {
        let tree = app.tree(window).unwrap().semantic_tree();
        tree.nodes.iter().any(|node| node.semantics.name == words)
    };
    click(&platform, &mut app, window, 340.0);
    assert!(platform.is_maximized(window) && said(&app, "restored"));
    platform.push(window, PlatformEvent::Resize { size: Size::new(400.0, 300.0), scale: 1.0 });
    platform.step(&mut app);
    platform.step(&mut app);
    assert!(said(&app, "maximized"));
    click(&platform, &mut app, window, 300.0);
    assert!(platform.is_minimized(window));
    // Minimized, it lost the keyboard and its size: the view hears of both.
    platform.push(window, PlatformEvent::Focus(false));
    platform.resize(window, Size::new(0.0, 0.0), 1.0);
    platform.step(&mut app);
    platform.step(&mut app);
    assert_eq!(heard(), (false, true));
    platform.push(window, PlatformEvent::Focus(true));
    platform.resize(window, Size::new(400.0, 300.0), 1.0);
    platform.step(&mut app);
    platform.step(&mut app);
    assert_eq!(heard(), (true, false));
    click(&platform, &mut app, window, 380.0);
    assert_eq!(app.window_count(), 0);
}

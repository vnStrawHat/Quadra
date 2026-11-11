//! The app on the headless platform, the null renderer and the fake text backend: frames are
//! drawn when something changed and not otherwise, a window follows its size, closing the last
//! window ends the loop, and a lost device is made again.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_app::{App, AppError};
use quadra_platform::{HeadlessPlatform, Platform, PlatformEvent, WindowId, WindowOptions};
use quadra_prim::Size;
use quadra_reactive::{Signal, signal};
use quadra_render::{NullRenderer, Renderer};
use quadra_text::FakeBackend;
use quadra_ui::{column, text};

fn app(platform: &HeadlessPlatform) -> App<NullRenderer> {
    App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()))
}

fn label(of: Signal<String>) -> impl FnOnce() -> quadra_ui::Div {
    move || column().child(text(of))
}

/// A wake is an entry point: the app looks at what changed since the last one.
fn turn(platform: &HeadlessPlatform, app: &mut App<NullRenderer>) -> bool {
    platform.waker().wake();
    platform.step(app)
}

fn open(app: &mut App<NullRenderer>, of: Signal<String>) -> WindowId {
    app.open_window(WindowOptions::new("test").size(400.0, 300.0), label(of)).unwrap()
}

#[test]
fn a_frame_is_drawn_when_something_changed_and_not_otherwise() {
    let platform = HeadlessPlatform::new();
    let mut app = app(&platform);
    let name = signal("one".to_string());
    let window = open(&mut app, name);
    assert!(platform.step(&mut app)); // the first frame
    assert_eq!(app.renderer.stats().frames, 1);
    assert!(!platform.step(&mut app)); // nothing changed: nothing to do
    turn(&platform, &mut app);
    assert_eq!(app.renderer.stats().frames, 1); // an entry point alone draws nothing

    name.set("three".to_string());
    turn(&platform, &mut app);
    let stats = app.renderer.stats();
    assert_eq!((stats.frames, stats.instances_uploaded), (2, 5)); // the five glyphs, once
    assert!(!app.tree(window).unwrap().needs_frame());
}

#[test]
fn a_window_is_laid_out_again_at_its_new_size() {
    let platform = HeadlessPlatform::new();
    let mut app = app(&platform);
    let window = open(&mut app, signal("text".to_string()));
    platform.step(&mut app);
    let root = |app: &App<NullRenderer>| {
        let tree = app.tree(window).unwrap();
        tree.rect(tree.root().unwrap()).size()
    };
    assert_eq!(root(&app), Size::new(400.0, 300.0));
    platform.resize(window, Size::new(200.0, 100.0), 2.0);
    platform.step(&mut app);
    assert_eq!(root(&app), Size::new(200.0, 100.0));
    assert_eq!(app.renderer.stats().frames, 2);

    platform.resize(window, Size::new(0.0, 0.0), 2.0); // minimized: no frame
    platform.step(&mut app);
    assert_eq!(app.renderer.stats().frames, 2);
}

#[test]
fn windows_share_signals_and_closing_the_last_one_ends_the_loop() {
    let platform = HeadlessPlatform::new();
    let mut app = app(&platform);
    let name = signal("a".to_string());
    let (first, second) = (open(&mut app, name), open(&mut app, name));
    platform.step(&mut app);
    assert_eq!(app.renderer.stats().frames, 2);
    name.set("b".to_string());
    turn(&platform, &mut app);
    assert_eq!(app.renderer.stats().frames, 4); // each window drew its own frame

    platform.push(first, PlatformEvent::CloseRequested);
    platform.step(&mut app);
    assert_eq!((app.window_count(), platform.title(first)), (1, None));
    assert_eq!(app.renderer.buffers(), 1); // what the renderer kept for it is freed
    name.set("c".to_string());
    turn(&platform, &mut app);
    assert_eq!(app.renderer.stats().frames, 5); // the signal no longer reaches the closed window

    platform.push(second, PlatformEvent::CloseRequested);
    app.run().unwrap(); // returns: the last window closed
    assert_eq!(platform.title(second), None);
}

#[test]
fn a_lost_device_is_made_again_and_everything_is_drawn_again() {
    let platform = HeadlessPlatform::new();
    let errors: Rc<RefCell<Vec<AppError>>> = Rc::default();
    let seen = errors.clone();
    let mut app = app(&platform).on_error(move |e| seen.borrow_mut().push(e.clone()));
    let name = signal("lost".to_string());
    let window = open(&mut app, name);
    platform.step(&mut app);
    let tiles = app.renderer.tiles_uploaded().0;

    app.renderer.lose_device();
    name.set("found".to_string());
    turn(&platform, &mut app); // the draw fails, the device is made again, a frame is asked for
    platform.step(&mut app);
    let tree = app.tree(window).unwrap();
    let scene = tree.scene();
    let root_layer = scene.roots()[0];
    assert_eq!(app.renderer.uploaded(scene.id(), root_layer), Some(scene.layer(root_layer).instances()));
    assert_eq!(scene.layer(root_layer).instances().len(), 5);
    assert_eq!(app.renderer.tiles_uploaded().0, tiles + 5); // every glyph again, the "o" too
    assert!(errors.borrow().is_empty()); // recovered: nothing to report
}

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

#[test]
fn pointer_events_of_a_window_reach_the_handlers_of_its_view() {
    use quadra_platform::{Mods, MouseButton, PointerKind};
    use quadra_prim::Point;
    use quadra_ui::div;

    let platform = HeadlessPlatform::new();
    let mut app = app(&platform);
    let count = signal(0);
    let view = move || {
        let button = div().size(100.0, 40.0).on_click(move |_| count.update(|n| *n += 1));
        column().child(button).child(text(move || count.get().to_string()))
    };
    let window = app.open_window(WindowOptions::new("clicks").size(400.0, 300.0), view).unwrap();
    platform.step(&mut app);
    let frames = app.renderer.stats().frames;
    let at = |kind| PlatformEvent::Pointer {
        kind,
        button: MouseButton::Left,
        at: Point::new(50.0, 20.0),
        mods: Mods::NONE,
    };
    for kind in [PointerKind::Move, PointerKind::Down, PointerKind::Up] {
        platform.push(window, at(kind));
    }
    platform.step(&mut app); // the events, then the frame their handler made necessary
    assert_eq!((count.get_untracked(), app.renderer.stats().frames), (1, frames + 1));
    platform.push(window, at(PointerKind::Move));
    assert!(platform.step(&mut app)); // delivered,
    assert_eq!(app.renderer.stats().frames, frames + 1); // and nothing to draw
}

#[test]
fn the_apps_keymap_turns_keys_into_actions_in_every_window() {
    use quadra_platform::{Key, Mods};
    use quadra_tree::{Action, Keymap};
    use quadra_ui::div;

    struct Add(i32);
    impl Action for Add {
        const NAME: &'static str = "count::add";
    }

    let platform = HeadlessPlatform::new();
    let mut keymap = Keymap::new();
    keymap.bind("ctrl-equal", Add(1)).unwrap();
    keymap.bind("ctrl-k ctrl-minus", Add(-10)).unwrap();
    let mut app = app(&platform).keymap(keymap);
    let count = signal(0);
    let view = move || div().on_action::<Add>(move |add| count.update(|n| *n += add.0)).child(text("keys"));
    let (first, second) = (
        app.open_window(WindowOptions::new("a"), view).unwrap(),
        app.open_window(WindowOptions::new("b"), view).unwrap(),
    );
    let key = |key, down| PlatformEvent::Key { key, code: 0, mods: Mods::CTRL, down, repeat: false };
    for (window, keys) in [(first, vec![Key::Equal]), (second, vec![Key::K, Key::Minus])] {
        for k in keys {
            platform.push(window, key(k, true));
            platform.push(window, key(k, false));
        }
    }
    platform.step(&mut app);
    assert_eq!(count.get_untracked(), -9);
}

#[test]
fn a_window_is_told_where_text_is_typed_and_when_it_is_not() {
    use quadra_platform::{Key, Mods};
    use quadra_ui::{div, text_input};

    let platform = HeadlessPlatform::new();
    let mut app = app(&platform);
    let name = signal("abc".to_string());
    let view = move || column().child(div().size(120.0, 20.0).child(text_input(name))).child(text("label"));
    let window = app.open_window(WindowOptions::new("ime").size(400.0, 300.0), view).unwrap();
    platform.step(&mut app);
    assert_eq!(platform.text_input(window), None); // no focus: input methods are off

    let key = |down| PlatformEvent::Key { key: Key::Tab, code: 0, mods: Mods::NONE, down, repeat: false };
    platform.push(window, key(true));
    platform.push(window, key(false));
    platform.step(&mut app);
    platform.step(&mut app);
    let caret = platform.text_input(window).expect("the input has the focus");
    assert!(caret.x > 0.0 && caret.height == 20.0, "{caret:?}"); // after "abc", as high as the field

    platform.push(window, PlatformEvent::Composition { text: "xy".into(), caret: 2 });
    platform.step(&mut app);
    platform.step(&mut app);
    assert!(platform.text_input(window).unwrap().x > caret.x); // the caret moved with what is composed
    platform.push(window, PlatformEvent::Text("xy".into()));
    platform.push(window, PlatformEvent::Composition { text: String::new(), caret: 0 });
    platform.step(&mut app);
    assert_eq!(name.get_untracked(), "abcxy");
}

#[test]
fn a_window_tells_assistive_technology_its_tree_once_asked_and_does_what_it_asks() {
    use quadra_tree::{Role, SemanticAction};
    use quadra_ui::div;

    let platform = HeadlessPlatform::new();
    let mut app = app(&platform);
    let count = signal(0);
    let view = move || {
        let add = div().role(Role::Button).test_id("add").on_click(move |_| count.update(|n| *n += 1));
        column().child(add.child(text("Add"))).child(text(move || count.get().to_string()))
    };
    let window = app.open_window(WindowOptions::new("a11y").size(400.0, 300.0), view).unwrap();
    platform.step(&mut app);
    count.set(5);
    turn(&platform, &mut app);
    assert!(platform.take_semantics(window).is_empty()); // nobody asked: nothing is built

    platform.push(window, PlatformEvent::SemanticsWanted);
    platform.step(&mut app); // no frame is due, and the tree is sent all the same
    let sent = platform.take_semantics(window);
    assert_eq!((sent.len(), sent[0].nodes.len()), (1, 3)); // the window, the button, the count
    let button = sent[0].nodes.iter().find(|n| n.semantics.role == Role::Button).unwrap().id;

    platform.push(window, PlatformEvent::Semantic { node: button, action: SemanticAction::Click });
    platform.step(&mut app);
    let sent = platform.take_semantics(window);
    assert_eq!(count.get_untracked(), 6);
    // One update, after the frame the click made necessary: the text that changed.
    let names: Vec<&str> = sent.iter().flat_map(|u| &u.nodes).map(|n| n.semantics.name.as_str()).collect();
    assert_eq!((sent.len(), names), (1, vec!["6"]));
    // A node that is gone is asked nothing.
    platform.push(window, PlatformEvent::Semantic { node: 999 << 32, action: SemanticAction::Click });
    platform.step(&mut app);
    assert_eq!(count.get_untracked(), 6);
}

//! What an app's windows are told and tell, on the headless platform: assistive technology
//! asks for the tree and acts on it, the cursor follows what the pointer is over, and every
//! window has the app's theme.

use std::rc::Rc;

use quadra_app::App;
use quadra_platform::{HeadlessPlatform, Platform, PlatformEvent, WindowOptions};
use quadra_prim::Color;
use quadra_reactive::{Signal, signal};
use quadra_render::{NullRenderer, Renderer};
use quadra_text::FakeBackend;
use quadra_ui::{column, text};

fn app(platform: &HeadlessPlatform) -> App<NullRenderer> {
    App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()))
}

/// A wake is an entry point: the app looks at what changed since the last one.
fn turn(platform: &HeadlessPlatform, app: &mut App<NullRenderer>) -> bool {
    platform.waker().wake();
    platform.step(app)
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

#[test]
fn a_window_is_given_the_cursor_of_what_the_pointer_is_over() {
    use quadra_platform::{Cursor, Mods, MouseButton, PointerKind};
    use quadra_prim::Point;
    use quadra_ui::{div, text_input};

    let platform = HeadlessPlatform::new();
    let mut app = app(&platform);
    let view = || {
        let link = div().size(100.0, 40.0).cursor(Cursor::Pointer);
        column().child(link).child(div().size(100.0, 20.0).child(text_input(signal(String::new()))))
    };
    let window = app.open_window(WindowOptions::new("cursor").size(400.0, 300.0), view).unwrap();
    platform.step(&mut app);
    let mut over = |y: f32| {
        let (kind, at) = (PointerKind::Move, Point::new(50.0, y));
        platform
            .push(window, PlatformEvent::Pointer { kind, button: MouseButton::None, at, mods: Mods::NONE });
        platform.step(&mut app);
        platform.cursor(window).unwrap()
    };
    // The link's, the one of a text input, and the arrow over the rest.
    assert_eq!([over(20.0), over(50.0), over(200.0)], [Cursor::Pointer, Cursor::Text, Cursor::Arrow]);
}

#[test]
fn every_window_has_the_apps_theme_and_a_view_can_change_it() {
    use quadra_reactive::use_context;
    use quadra_tree::{Theme, Token};
    use quadra_ui::div;

    const SURFACE: Token = Token(0);
    let platform = HeadlessPlatform::new();
    let mut app = app(&platform).theme(Theme::new(vec![Color::rgb(0x202020)]));
    let lighter = signal(false);
    let view = move || {
        // The theme is in the context of every view: this one writes it when told to.
        let theme = use_context::<Signal<Rc<Theme>>>().expect("an app gives its views the theme");
        let next = Rc::new(Theme::new(vec![Color::rgb(0xF0F0F0)]));
        quadra_reactive::effect(move || {
            if lighter.get() {
                theme.set(next.clone());
            }
        });
        div().size(100.0, 40.0).bg(SURFACE)
    };
    let (first, second) = (
        app.open_window(WindowOptions::new("a"), view).unwrap(),
        app.open_window(WindowOptions::new("b"), view).unwrap(),
    );
    platform.step(&mut app);
    let background = |app: &App<NullRenderer>, window| {
        let tree = app.tree(window).unwrap();
        tree.paint_style(tree.root().unwrap()).background
    };
    assert_eq!([background(&app, first), background(&app, second)], [Color::rgb(0x202020); 2]);
    let frames = app.renderer.stats().frames;
    lighter.set(true);
    turn(&platform, &mut app);
    assert_eq!([background(&app, first), background(&app, second)], [Color::rgb(0xF0F0F0); 2]);
    assert_eq!(app.renderer.stats().frames, frames + 2); // one frame a window, and no more
    assert!(!platform.step(&mut app));
}

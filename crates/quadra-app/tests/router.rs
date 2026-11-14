//! Pages (specs/15, "Pages"): a stack of routes, going back and forth by the router, by
//! Alt-Left and Alt-Right and by the buttons of a mouse; a page is made again only when the
//! route changes.

use std::cell::Cell;
use std::rc::Rc;

use quadra_app::{App, Router, router};
use quadra_platform::{
    HeadlessPlatform, Key, Mods, MouseButton, Platform, PlatformEvent, PointerKind, WindowOptions,
};
use quadra_prim::Point;
use quadra_render::NullRenderer;
use quadra_text::FakeBackend;
use quadra_ui::text;

#[derive(Clone, Debug, PartialEq)]
enum Page {
    Home,
    File(u32),
}

#[test]
fn a_router_goes_back_and_forth_and_forgets_what_was_ahead() {
    let pages = router(Page::Home);
    assert!(!pages.can_back() && !pages.can_forward());
    pages.push(Page::File(1));
    pages.push(Page::File(1)); // the page shown: nothing
    pages.push(Page::File(2));
    pages.back();
    assert_eq!((pages.current(), pages.can_back(), pages.can_forward()), (Page::File(1), true, true));
    pages.push(Page::File(3)); // File(2) is forgotten
    assert!(!pages.can_forward());
    pages.replace(Page::File(4));
    pages.back();
    assert_eq!(pages.current(), Page::File(1)); // Home, File(1), File(4)
    pages.forward();
    pages.forward(); // past the end: nothing
    assert_eq!(pages.current(), Page::File(4));
}

#[test]
fn keys_and_the_mouse_go_back_and_forth_and_a_page_is_made_once() {
    let platform = HeadlessPlatform::new();
    let mut app = App::with(Rc::new(platform.clone()), NullRenderer::new(), Box::new(FakeBackend::new()));
    let (made, pages) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(None::<Router<Page>>)));
    let (count, keep) = (made.clone(), pages.clone());
    let window = app
        .open_window(WindowOptions::new("pages").size(300.0, 200.0), move || {
            let pages = router(Page::Home);
            keep.set(Some(pages));
            pages.view(move |page| {
                count.set(count.get() + 1);
                text(format!("{page:?}"))
            })
        })
        .unwrap();
    platform.step(&mut app);
    let pages = pages.get().unwrap();
    let said = |app: &App<NullRenderer>| {
        let tree = app.tree(window).unwrap();
        tree.semantic_tree().nodes.iter().map(|n| n.semantics.name.clone()).collect::<Vec<_>>().join("|")
    };
    assert!(said(&app).contains("Home"));
    pages.push(Page::File(7));
    platform.waker().wake();
    platform.step(&mut app);
    assert!(said(&app).contains("File(7)"), "{}", said(&app));

    let key = |key| PlatformEvent::Key { key, code: 0, mods: Mods::ALT, down: true, repeat: false };
    platform.push(window, key(Key::Left));
    platform.step(&mut app);
    assert_eq!(pages.current(), Page::Home);
    assert!(said(&app).contains("Home"), "{}", said(&app));
    platform.push(window, key(Key::Right));
    platform.step(&mut app);
    assert_eq!(pages.current(), Page::File(7));

    let at = Point::new(20.0, 20.0);
    let press = |button| PlatformEvent::Pointer { kind: PointerKind::Down, button, at, mods: Mods::NONE };
    platform.push(window, press(MouseButton::Back));
    platform.step(&mut app);
    assert_eq!(pages.current(), Page::Home);
    platform.push(window, press(MouseButton::Forward));
    platform.step(&mut app);
    assert_eq!(pages.current(), Page::File(7));
    // Home, File(7), Home, File(7), Home, File(7): a page each time the route changed.
    assert_eq!(made.get(), 6);
}

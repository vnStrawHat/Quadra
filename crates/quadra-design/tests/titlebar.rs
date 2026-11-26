//! The title bar (specs/design, "Title bar"): the window is dragged by it, what the
//! app puts in it stays the app's, and its three buttons ask the window.

use std::rc::Rc;

use quadra_design::{MenuItem, button, dark, titlebar};
use quadra_platform::{HitRegion, WindowControls};
use quadra_prim::Point;
use quadra_testing::Harness;
use quadra_tree::{Role, WindowRequest};
use quadra_view::{div, text};

fn page(small: bool) -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let bar =
            titlebar("Notes").menu(|| vec![MenuItem::new("Quit")]).control(button("Sync").test_id("sync"));
        let bar = if small { bar.title_size(16.0) } else { bar };
        quadra_design::root().child(bar).child(div().grow(1.0).child(text("content")))
    });
    h.frame();
    h
}

#[test]
fn the_bar_drags_the_window_and_its_buttons_ask_it() {
    let mut h = page(false);
    let tree = h.semantics();
    for line in [
        "Button \"Menu\"",
        "Text \"Notes\"",
        "Button \"Minimize\"",
        "Button \"Maximize\"",
        "Button \"Close\"",
    ] {
        assert!(tree.contains(line), "{line} is not in:\n{tree}");
    }
    // 32 px across the window, the three buttons 36 wide at its right end.
    let bar = h.get_by_test_id("titlebar").node();
    let (min, max) =
        (h.get_by_role(Role::Button, "Minimize").node(), h.get_by_role(Role::Button, "Maximize").node());
    let (bar, min, max) = (h.tree.window_rect(bar), h.tree.window_rect(min), h.tree.window_rect(max));
    assert_eq!((bar.width, bar.height, min.x, min.width, max.x), (800.0, 32.0, 692.0, 36.0, 728.0));

    // The title and the room left empty drag the window; the menu button and the app's
    // control stay theirs; the maximize button is the one the OS knows; under the bar is
    // the content.
    let sync = h.get_by_test_id("sync").node();
    let title = h.get_by_text("Notes").node();
    let (title, sync) = (h.tree.window_rect(title), h.tree.window_rect(sync));
    let what = |h: &Harness, x: f32, y: f32| h.tree.window_region_at(Point::new(x, y));
    assert_eq!(
        [what(&h, title.center().x, 16.0), what(&h, 500.0, 16.0), what(&h, 15.0, 16.0)],
        [HitRegion::Caption, HitRegion::Caption, HitRegion::Client]
    );
    assert_eq!(
        [
            what(&h, sync.center().x, 16.0),
            what(&h, 740.0, 16.0),
            what(&h, 780.0, 16.0),
            what(&h, 400.0, 100.0)
        ],
        [HitRegion::Client, HitRegion::Maximize, HitRegion::Client, HitRegion::Client]
    );

    for (name, asked) in [("Minimize", WindowRequest::Minimize), ("Maximize", WindowRequest::ToggleMaximize)]
    {
        h.get_by_role(Role::Button, name).click();
        assert_eq!(h.tree.take_window_requests(), [asked]);
    }
    // Maximized, the button puts the window back, and says so.
    h.tree.set_maximized(true);
    h.frame();
    assert!(h.semantics().contains("Button \"Restore\""));
    h.get_by_role(Role::Button, "Close").click();
    assert_eq!(h.tree.take_window_requests(), [WindowRequest::Close]);
}

#[test]
fn the_title_of_a_small_window_is_its_caption() {
    let mut h = page(true);
    let bar = h.get_by_test_id("titlebar").node();
    assert_eq!(h.tree.window_rect(bar).height, 34.0);
}

#[test]
fn where_the_system_draws_the_buttons_the_bar_leaves_them_their_room() {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    let controls = Rc::new(std::cell::Cell::new(None));
    let made = controls.clone();
    h.mount(move || {
        // As a window of macOS says it.
        let controls = quadra_reactive::signal(WindowControls::System { leading: 78.0 });
        quadra_reactive::provide(controls);
        made.set(Some(controls));
        let bar = titlebar("Notes").menu(|| vec![MenuItem::new("Quit")]);
        quadra_design::root().child(bar).child(div().grow(1.0).child(text("content")))
    });
    h.frame();
    let tree = h.semantics();
    for name in ["Minimize", "Maximize", "Close"] {
        assert!(!tree.contains(&format!("Button \"{name}\"")), "{name} is drawn:\n{tree}");
    }
    // The menu's button starts after the system's buttons, which are dragged by too.
    let menu = h.get_by_role(Role::Button, "Menu").node();
    assert!(h.tree.window_rect(menu).x >= 78.0);
    assert_eq!(h.tree.window_region_at(Point::new(40.0, 16.0)), HitRegion::Caption);
    assert_eq!(h.tree.window_region_at(Point::new(780.0, 16.0)), HitRegion::Caption);
    // The window fills the screen and shows no buttons: the bar has their room back.
    controls.get().unwrap().set(WindowControls::System { leading: 0.0 });
    h.frame();
    assert!(h.tree.window_rect(menu).x < 20.0);
}

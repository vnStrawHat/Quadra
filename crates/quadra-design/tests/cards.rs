//! The hover card and the drawer (specs/13, "Overlays in the kit"): a card that shows after a
//! rest and stays while the pointer is on it, and a dialog along an edge of the window.

use std::rc::Rc;
use std::time::Duration;

use quadra_design::{Edge, button, dark, drawer, hover_card};
use quadra_platform::{Key, Mods};
use quadra_prim::Rect;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role};
use quadra_view::{View, column, text};

fn mounted<V: View>(build: impl FnOnce() -> V) -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(build);
    h.frame();
    h
}

fn wait(h: &mut Harness, ms: u64) {
    h.advance(Duration::from_millis(ms));
    h.frame();
    // A few frames: what moves asks for one each frame while the clock stands still.
    for _ in 0..8 {
        if h.frame_if_needed().is_none() {
            break;
        }
    }
}

#[test]
fn a_hover_card_shows_after_a_rest_and_stays_while_the_pointer_is_on_it() {
    let opened = signal(0);
    let mut h = mounted(move || {
        let more = move || {
            let logs = button("Open logs").on_click(move || opened.update(|n| *n += 1));
            column().width(240.0).child(text("Running on node-2")).child(logs)
        };
        quadra_design::root().child(column().child(hover_card(button("api-7d9f").test_id("pod"), more)))
    });
    let pod = h.get_by_test_id("pod").node();
    let name = h.tree.window_rect(pod);
    h.pointer(PointerInput::Move, name.x + 5.0, name.y + 5.0);
    wait(&mut h, 300);
    assert!(!h.has_text("Running on node-2"));
    wait(&mut h, 100);
    assert!(h.has_text("Running on node-2"));
    // Under the name at its start, 6 px off it; the focus is where it was: nowhere.
    let card = h.tree.window_rect(h.tree.overlays().next().unwrap());
    assert_eq!((card.x, card.y, h.tree.focused()), (name.x, name.bottom() + 6.0, None));

    // The pointer crosses the gap and comes on the card: the card stays, and what it holds
    // is pressed.
    h.pointer(PointerInput::Move, card.x + 5.0, card.y + 5.0);
    wait(&mut h, 1000);
    let logs = h.get_by_role(Role::Button, "Open logs").node();
    let logs = h.tree.window_rect(logs);
    h.click(logs.center().x, logs.center().y);
    wait(&mut h, 0);
    assert_eq!((opened.get_untracked(), h.has_text("Running on node-2")), (1, true));

    // Away from both: the card goes after the grace, and a short visit shows nothing.
    h.pointer(PointerInput::Move, 600.0, 500.0);
    wait(&mut h, 100);
    assert!(h.has_text("Running on node-2"));
    wait(&mut h, 100);
    assert!(!h.has_text("Running on node-2"));
    h.pointer(PointerInput::Move, name.x + 5.0, name.y + 5.0);
    wait(&mut h, 200);
    h.pointer(PointerInput::Move, 600.0, 500.0);
    wait(&mut h, 1000);
    assert!(!h.has_text("Running on node-2"));
}

#[test]
fn a_drawer_lies_along_an_edge_of_the_window_over_all_of_its_length() {
    let (right, left, bottom) = (signal(false), signal(false), signal(false));
    let mut h = mounted(move || {
        let body = || text("Running");
        let drawers = column()
            .child(drawer(right, Edge::Right, "Pod", body))
            .child(drawer(left, Edge::Left, "Pod", body))
            .child(drawer(bottom, Edge::Bottom, "Pod", body).size(240.0));
        quadra_design::root().child(drawers)
    });
    let panel = |h: &Harness| h.tree.window_rect(h.tree.overlays().next().unwrap());

    // At the right: 360 px with its border, as high as the window. The cross gives it up.
    right.set(true);
    wait(&mut h, 0);
    assert_eq!(panel(&h), Rect::new(440.0, 0.0, 360.0, 600.0));
    assert!(h.semantics().contains("Dialog \"Pod\"") && h.has_text("Running"), "{}", h.semantics());
    // It slides in from the right: past its edge as it opens, at its place 200 ms later.
    let moved = |h: &Harness| {
        let words = h.query(|n| n.semantics.name == "Running")[0];
        let mut at = Some(words);
        let mut x = 0.0;
        while let Some(node) = at {
            x += h.tree.pose(node).translate.x;
            at = h.tree.parent(node);
        }
        x
    };
    assert!(moved(&h) > 0.0, "{}", moved(&h));
    wait(&mut h, 250);
    assert_eq!(moved(&h), 0.0);
    h.get_by_role(Role::Button, "Close").click();
    wait(&mut h, 0);
    assert_eq!((right.get_untracked(), h.tree.overlays().count()), (false, 0));

    // At the left; Escape gives it up.
    left.set(true);
    wait(&mut h, 0);
    assert_eq!(panel(&h), Rect::new(0.0, 0.0, 360.0, 600.0));
    h.key(Key::Escape, Mods::NONE);
    wait(&mut h, 0);
    assert!(!left.get_untracked());

    // At the bottom: as wide as the window, as high as it was told. A press outside gives it up.
    bottom.set(true);
    wait(&mut h, 0);
    assert_eq!(panel(&h), Rect::new(0.0, 360.0, 800.0, 240.0));
    h.click(400.0, 100.0);
    wait(&mut h, 0);
    assert!(!bottom.get_untracked());
}

#[test]
fn a_drawer_beside_the_content_leaves_it_in_use_and_takes_escape_while_it_has_the_focus() {
    let (open, shown) = (signal(false), signal("pod-a"));
    let mut h = mounted(move || {
        let pick = move |name: &'static str| {
            button(name).on_click(move || {
                shown.set(name);
                open.set(true);
            })
        };
        let list = column().grow(1.0).child(pick("pod-a")).child(pick("pod-b"));
        let head =
            move || quadra_view::row().gap(8.0).child(text(move || shown.get())).child(button("Restart"));
        let inspector = drawer(open, Edge::Right, "Pod", || text("Running")).beside().head(head);
        quadra_design::root().child(quadra_view::row().grow(1.0).child(list).child(inspector))
    });
    // Closed, it takes no room: the list is as wide as the window.
    let a = h.get_by_role(Role::Button, "pod-a").node();
    assert_eq!(h.tree.window_rect(h.tree.parent(a).unwrap()).width, 800.0);

    h.get_by_role(Role::Button, "pod-a").click();
    wait(&mut h, 250);
    // In the layout, not over it: the list gives it its room; still a dialog to a reader, its
    // head the app's row.
    assert_eq!(h.tree.overlays().count(), 0);
    let dialog = h.query(|n| n.semantics.role == Role::Dialog)[0];
    assert_eq!(h.tree.window_rect(dialog), Rect::new(440.0, 0.0, 360.0, 600.0));
    assert_eq!(h.tree.meaning(dialog).name, "Pod");
    let tree = h.semantics();
    assert!(
        tree.contains("Text \"pod-a\"") && h.has_text("Running") && !tree.contains("Text \"Pod\""),
        "{tree}"
    );
    assert_eq!(h.tree.window_rect(h.tree.parent(a).unwrap()).width, 440.0);
    // The focus went into it.
    let inside = |h: &Harness, mut at: quadra_tree::NodeId| loop {
        if at == dialog {
            break true;
        }
        match h.tree.parent(at) {
            Some(up) => at = up,
            None => break false,
        }
    };
    assert!(h.tree.focused().is_some_and(|at| inside(&h, at)));

    // A press on the next row is not a press outside: it shows that row's object.
    h.get_by_role(Role::Button, "pod-b").click();
    wait(&mut h, 0);
    assert!(open.get_untracked() && h.query(|n| n.semantics.name == "pod-b").len() == 2);
    // Escape with the focus in the list leaves it; with the focus in it, gives it up, and the
    // focus goes back to what had it.
    h.key(Key::Escape, Mods::NONE);
    wait(&mut h, 0);
    assert!(open.get_untracked());
    h.get_by_role(Role::Button, "Restart").click();
    h.key(Key::Escape, Mods::NONE);
    wait(&mut h, 0);
    assert!(!open.get_untracked());
    assert_eq!(h.tree.focused(), Some(a));
}

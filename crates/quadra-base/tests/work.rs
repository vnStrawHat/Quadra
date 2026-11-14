//! The workbench with bare parts (specs/17, "Tests"): a tab strip and a pane tree.

use std::rc::Rc;

use quadra_base::work::{self, Axis, Pane, Parts, Tab};
use quadra_platform::{Mods, MouseButton};
use quadra_prim::Point;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role};
use quadra_ui::{View, div, each, row};

fn mounted<V: View + 'static>(view: impl FnOnce() -> V) -> Harness {
    let mut h = Harness::new();
    h.mount(view);
    h.frame();
    h
}

#[test]
fn a_tab_is_chosen_moved_and_closed_by_the_middle_button() {
    let tabs = signal(vec![Tab::new("a", "Alpha"), Tab::new("b", "Beta")]);
    let active = signal("a".to_string());
    let mut h = mounted(move || {
        let strip =
            work::strip(tabs, active).on_close(move |id| tabs.update(|all| all.retain(|t| t.id != id)));
        let look = strip.clone();
        let faces =
            each(move || tabs.get(), |tab| tab.id.clone(), move |tab| look.tab(div().size(50.0, 20.0), tab));
        row().role(Role::TabList).child(faces.container(strip.lane(row())))
    });
    h.get_by_role(Role::Tab, "Beta").click();
    h.frame();
    assert!(h.semantics().contains("Tab \"Beta\" [selected]"), "{}", h.semantics());
    // Alpha dragged past the middle of Beta goes after it.
    h.pointer(PointerInput::Down, 10.0, 10.0);
    h.pointer(PointerInput::Move, 80.0, 10.0);
    h.pointer(PointerInput::Up, 80.0, 10.0);
    let ids = || tabs.with_untracked(|all| all.iter().map(|t| t.id.clone()).collect::<Vec<_>>().join(" "));
    assert_eq!(ids(), "b a");
    h.frame();
    let at = Point::new(10.0, 10.0);
    h.tree.pointer(PointerInput::Down, MouseButton::Middle, at, Mods::NONE, h.now());
    h.tree.pointer(PointerInput::Up, MouseButton::Middle, at, Mods::NONE, h.now());
    assert_eq!(ids(), "a");
}

#[test]
fn a_seam_is_dragged_and_a_double_click_makes_the_sides_even() {
    let root = signal(Pane::row(Pane::leaf("left"), Pane::leaf("right"), 0.5));
    let mut h = mounted(move || {
        let seam = Rc::new(|strip: quadra_ui::Div, _: Axis, _: bool| strip.role(Role::Group).test_id("seam"));
        let parts = Parts { grip: 6.0, seam, veil: Rc::new(|_| div()) };
        let tree = work::panes(root, |group: &str| div().label(group.to_string()), parts);
        row().size(400.0, 200.0).child(tree)
    });
    h.frame();
    let seam = h.get_by_test_id("seam").node();
    let x = h.tree.window_rect(seam).center().x;
    h.pointer(PointerInput::Down, x, 100.0);
    h.pointer(PointerInput::Move, x + 100.0, 100.0);
    h.pointer(PointerInput::Up, x + 100.0, 100.0);
    assert_eq!(root.with_untracked(|tree| tree.ratio(&[])), Some(0.75));
    h.frame();
    let x = h.tree.window_rect(seam).center().x;
    for _ in 0..2 {
        h.pointer(PointerInput::Down, x, 100.0);
        h.pointer(PointerInput::Up, x, 100.0);
    }
    assert_eq!(root.with_untracked(|tree| tree.ratio(&[])), Some(0.5));
}

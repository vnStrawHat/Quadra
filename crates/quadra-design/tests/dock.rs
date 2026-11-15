//! Docking (specs/13, "The workbench"): what a drop asks of the groups and the pane tree, and
//! tabs torn off their strip and dropped on another strip or on a pane.

use std::rc::Rc;

use quadra_design::dark;
use quadra_design::work::{Dock, DockEvent, Pane, Tab, Zone, dock, panes, tab_strip, zone_at};
use quadra_prim::{Point, Rect};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role};
use quadra_ui::{column, div};

/// "main.rs" and "app.rs" at the left, "Logs" and "Shell" at the right, in 806 by 406.
fn page() -> (Harness, Dock) {
    let tabs = |all: [(&str, &str); 2]| all.map(|(id, title)| Tab::new(id, title)).to_vec();
    let groups = [
        ("left", tabs([("main", "main.rs"), ("app", "app.rs")])),
        ("right", tabs([("logs", "Logs"), ("shell", "Shell")])),
    ];
    let work = dock(Pane::row(Pane::leaf("left"), Pane::leaf("right"), 0.5), groups);
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let leaf = move |group: &str| {
            let tabs = work.group(group).expect("a leaf has a group");
            column().grow(1.0).child(tab_strip(tabs.tabs, tabs.active).dock(work, group))
        };
        quadra_design::root().child(div().size(806.0, 406.0).child(panes(work.root, leaf).dock(work)))
    });
    settle(&mut h);
    (h, work)
}

fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {}
}

fn ids(work: Dock, group: &str) -> String {
    let Some(group) = work.group(group) else { return "none".into() };
    let ids = group.tabs.with_untracked(|all| all.iter().map(|tab| tab.id.clone()).collect::<Vec<_>>());
    format!("{} ({})", ids.join(" "), group.active.get_untracked())
}

fn event(tab: &str, from: &str, to: &str, zone: Zone, index: Option<usize>) -> DockEvent {
    DockEvent { tab: tab.into(), from: from.into(), to: to.into(), zone, index }
}

#[test]
fn a_drop_moves_a_tab_to_a_group_or_splits_a_pane_for_it() {
    let (_h, work) = page();
    // Into another group, where it is dropped; it shows there, and its neighbour here.
    work.apply(&event("main", "left", "right", Zone::Center, Some(1)));
    assert_eq!(
        (ids(work, "left"), ids(work, "right")),
        ("app (app)".into(), "logs main shell (main)".into())
    );
    // Onto an edge: a new group split off that side.
    work.apply(&event("shell", "right", "left", Zone::Bottom, None));
    let stacked = Pane::column(Pane::leaf("left"), Pane::leaf("group-1"), 0.5);
    assert_eq!(work.root.get_untracked(), Pane::row(stacked, Pane::leaf("right"), 0.5));
    assert_eq!(ids(work, "group-1"), "shell (shell)");
    // The last tab of a group takes the group and its pane with it.
    work.apply(&event("app", "left", "right", Zone::Center, None));
    assert_eq!(work.root.get_untracked(), Pane::row(Pane::leaf("group-1"), Pane::leaf("right"), 0.5));
    assert_eq!((ids(work, "left"), ids(work, "right")), ("none".into(), "logs main app (app)".into()));
    // Nothing: into the middle of its own pane, off a pane it is alone in, a tab that is not there.
    work.apply(&event("logs", "right", "right", Zone::Center, Some(2)));
    work.apply(&event("shell", "group-1", "group-1", Zone::Left, None));
    work.apply(&event("lost", "right", "group-1", Zone::Center, None));
    assert_eq!(
        (ids(work, "right"), ids(work, "group-1")),
        ("logs main app (app)".into(), "shell (shell)".into())
    );
}

#[test]
fn the_outer_quarter_of_a_pane_is_its_edges() {
    let pane = Rect::new(100.0, 100.0, 400.0, 200.0);
    let zone = |x, y| zone_at(pane, Point::new(x, y));
    assert_eq!(
        [zone(300.0, 200.0), zone(150.0, 200.0), zone(480.0, 200.0)],
        [Zone::Center, Zone::Left, Zone::Right]
    );
    assert_eq!([zone(300.0, 120.0), zone(300.0, 290.0)], [Zone::Top, Zone::Bottom]);
}

#[test]
fn a_tab_is_torn_off_its_strip_and_dropped_on_a_strip_or_a_pane() {
    let (mut h, work) = page();
    let tab = |h: &mut Harness, title: &str| {
        let node = h.get_by_role(Role::Tab, title).node();
        h.tree.window_rect(node)
    };
    // Along its strip a tab is moved by the strip, as anywhere: no drag.
    let (main, app) = (tab(&mut h, "main.rs"), tab(&mut h, "app.rs"));
    h.pointer(PointerInput::Down, main.center().x, 16.0);
    h.pointer(PointerInput::Move, app.center().x, 16.0);
    assert!(!h.tree.dragging());
    h.pointer(PointerInput::Up, app.center().x, 16.0);
    settle(&mut h);
    assert_eq!(ids(work, "left"), "app main (main)");

    // Torn off, it follows the pointer as a chip, and is dropped before "Logs".
    let (main, logs) = (tab(&mut h, "main.rs"), tab(&mut h, "Logs"));
    h.pointer(PointerInput::Down, main.center().x, 16.0);
    h.pointer(PointerInput::Move, main.center().x, 40.0);
    h.pointer(PointerInput::Move, logs.x + 4.0, 16.0);
    settle(&mut h);
    assert_eq!((h.tree.dragging(), h.tree.overlays().count()), (true, 1));
    h.pointer(PointerInput::Up, logs.x + 4.0, 16.0);
    settle(&mut h);
    assert_eq!(
        (ids(work, "left"), ids(work, "right")),
        ("app (app)".into(), "main logs shell (main)".into())
    );
    assert_eq!(h.tree.overlays().count(), 0);

    // Dropped on the bottom edge of the left pane, a tab has a pane of its own there.
    let shell = tab(&mut h, "Shell");
    h.pointer(PointerInput::Down, shell.center().x, 16.0);
    h.pointer(PointerInput::Move, shell.center().x, 40.0);
    h.pointer(PointerInput::Move, 150.0, 390.0);
    h.pointer(PointerInput::Up, 150.0, 390.0);
    settle(&mut h);
    let stacked = Pane::column(Pane::leaf("left"), Pane::leaf("group-1"), 0.5);
    assert_eq!(work.root.get_untracked(), Pane::row(stacked, Pane::leaf("right"), 0.5));
    assert_eq!((ids(work, "group-1"), tab(&mut h, "Shell").y), ("shell (shell)".into(), 209.0));

    // A click chooses a tab of a strip whose tabs can be torn off.
    let logs = tab(&mut h, "Logs");
    h.click(logs.center().x, 16.0);
    assert_eq!(ids(work, "right"), "main logs (logs)");
}

//! Handlers on views (specs/05, "Handlers"; specs/09): a click runs a handler, the handler
//! writes a signal, and the next frame runs the one binding that reads it.

use quadra_prim::Color;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{PointerInput, PointerKind, Text};
use quadra_ui::{column, div, mount_root, row, show, text};

#[test]
fn a_counter_counts_its_clicks() {
    let count = signal(0);
    let button = |label: &str, by: i32| {
        div().size(40.0, 20.0).child(text(label.to_string())).on_click(move |_| count.update(|n| *n += by))
    };
    let number = text(move || count.get().to_string());
    let mut h = Harness::new();
    mount_root(&mut h.tree, column().child(number).child(row().child(button("-", -1)).child(button("+", 1))));
    h.frame();
    let label = h.tree.children(h.tree.root().unwrap())[0];
    let shows = |h: &mut Harness| h.tree.widget_mut::<Text>(label).unwrap().text().to_string();

    // The number's line is about 20 px high; the buttons are below it, 40 px each.
    h.click(60.0, 30.0);
    h.click(60.0, 30.0);
    let (stats, _) = h.frame(); // two clicks before a frame: one binding run, not two
    assert_eq!((shows(&mut h), stats.bindings_run), ("2".to_string(), 1));
    h.click(10.0, 30.0);
    h.frame();
    assert_eq!(shows(&mut h), "1");
    h.click(300.0, 300.0); // beside the buttons
    assert!(h.frame_if_needed().is_none());
}

#[test]
fn a_pointer_over_plain_content_asks_for_no_frame_and_hover_drives_what_reads_it() {
    let over = signal(false);
    let card = div().size(100.0, 50.0).bg(Color::rgb(0x202020));
    let card = card.on_pointer(PointerKind::Enter, move |_| over.set(true));
    let card = card.on_pointer(PointerKind::Leave, move |_| over.set(false));
    let hint = show(move || over.get(), || text("over"));
    let mut h = Harness::new();
    mount_root(&mut h.tree, column().child(card).child(div().size(100.0, 50.0)).child(hint));
    h.frame();

    h.pointer(PointerInput::Move, 50.0, 75.0); // the box without handlers
    h.pointer(PointerInput::Move, 60.0, 80.0);
    assert!(h.frame_if_needed().is_none()); // specs/14: no frame

    h.pointer(PointerInput::Move, 50.0, 25.0);
    let (stats, _) = h.frame();
    assert!(over.get_untracked() && stats.nodes_painted > 0); // the hint appears
    h.pointer(PointerInput::Leave, 0.0, 0.0);
    h.frame();
    assert!(!over.get_untracked());
}

#[test]
fn a_box_shows_hover_and_press_without_running_app_code() {
    let (base, over, down) = (Color::rgb(0xE8E8E8), Color::rgb(0xD8D8D8), Color::rgb(0xC0C0C0));
    let ran = signal(0);
    let button = move || {
        ran.update(|n| *n += 1);
        div().size(80.0, 30.0).bg(base).radius(6.0).hover(|s| s.bg(over)).active(|s| s.bg(down).radius(4.0))
    };
    let mut h = Harness::new();
    mount_root(&mut h.tree, column().child(button()));
    h.frame();
    let node = h.tree.children(h.tree.root().unwrap())[0];
    h.pointer(PointerInput::Move, 40.0, 15.0);
    let (stats, _) = h.frame();
    assert_eq!((h.tree.paint_style(node).background, stats.bindings_run, stats.nodes_styled), (over, 0, 1));
    h.pointer(PointerInput::Down, 40.0, 15.0);
    h.frame();
    assert_eq!((h.tree.paint_style(node).background, h.tree.paint_style(node).radii.top_left), (down, 4.0));
    h.pointer(PointerInput::Up, 40.0, 15.0);
    h.pointer(PointerInput::Move, 400.0, 300.0);
    h.frame();
    assert_eq!((h.tree.paint_style(node).background, ran.get_untracked()), (base, 1)); // built once
}

//! Showing what is scrolled out of view (specs/06, "Scrolling"): a node asks whatever
//! scrolls around it to show it, the keyboard's focus does so by itself, and a list is told
//! which row to show whether that row is mounted or not.

use quadra_platform::{Key, Mods};
use quadra_prim::Rect;
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{Role, SemanticAction};
use quadra_view::{NodeRef, column, div, mount_root, scroll, text, uniform_list, virtual_list};

/// A page that scrolls: a 100 px header, a 200 px box that scrolls thirty rows of 20 px,
/// and 1000 px of footer. Row 25 has a handle, and can have the focus.
fn page() -> (Harness, NodeRef, Signal<f32>, Signal<f32>) {
    let (outer, inner, row) = (signal(0.0), signal(0.0), NodeRef::new());
    let handle = row.clone();
    let mut h = Harness::new();
    h.mount(move || {
        let line = move |i: usize| {
            let line = div().height(20.0).role(Role::ListItem).child(text(format!("row {i}")).size(10.0));
            if i == 25 { line.node_ref(&handle).focusable() } else { line }
        };
        let list = scroll(inner, column().children((0..30).map(line))).container(div().height(200.0));
        let page = column().child(div().height(100.0)).child(list).child(div().height(1000.0));
        scroll(outer, page)
    });
    h.frame();
    (h, row, outer, inner)
}

fn rect(h: &Harness, row: &NodeRef) -> Rect {
    h.tree.window_rect(row.get().unwrap())
}

#[test]
fn a_node_is_shown_by_scrolling_as_little_as_it_takes() {
    let (mut h, row, outer, inner) = page();
    assert_eq!(rect(&h, &row).y, 600.0); // row 25 is 500 px down a box that shows 200
    row.scroll_into_view();
    h.frame();
    // The box scrolled until the row's end was at its own: the page did not have to move.
    assert_eq!((inner.get_untracked(), outer.get_untracked(), rect(&h, &row).y), (320.0, 0.0, 280.0));

    outer.set(700.0); // the whole box is above the window now
    inner.set(580.0); // and the row above what the box shows (it stops at 400, its end)
    h.frame();
    row.scroll_into_view();
    h.frame();
    // Both scrolled back, each to where the row starts.
    assert_eq!((inner.get_untracked(), outer.get_untracked(), rect(&h, &row).y), (400.0, 200.0, 0.0));
    row.scroll_into_view(); // it shows: nothing moves
    let (stats, _) = h.frame();
    assert_eq!((stats.nodes_laid_out, stats.layers_composed, inner.get_untracked()), (0, 0, 400.0));
}

#[test]
fn the_focus_of_the_keyboard_and_a_reader_show_what_they_go_to() {
    let (mut h, row, _, inner) = page();
    h.key(Key::Tab, Mods::NONE); // the only node that can have the focus
    h.frame();
    assert_eq!((h.tree.focused(), inner.get_untracked()), (row.get(), 320.0));
    h.get_by_text("row 2").act(SemanticAction::ScrollIntoView);
    h.frame();
    assert_eq!(inner.get_untracked(), 40.0); // up, until row 2 starts where the box does
}

/// The rows of a list that are mounted: the index each says, where it is, how tall.
fn rows(h: &mut Harness) -> Vec<(usize, f32, f32)> {
    let content = h.tree.children(h.tree.root().unwrap())[0];
    let mut rows = Vec::new();
    for wrapper in h.tree.children(content).to_vec() {
        let label = h.tree.children(h.tree.children(wrapper)[0])[0];
        let said = h.tree.widget_mut::<quadra_tree::Text>(label).unwrap().text().to_string();
        let rect = h.tree.window_rect(wrapper);
        rows.push((said.trim_start_matches("row ").parse().unwrap(), rect.y, rect.height));
    }
    rows
}

#[test]
fn a_list_shows_the_row_it_is_told_to_mounted_and_measured_or_not() {
    let (want, scroll_y) = (signal(None::<usize>), signal(0.0));
    let mut h = Harness::new();
    let row = |i: Signal<usize>| {
        let label = text(move || format!("row {}", i.get())).size(8.0);
        div().height(move || 10.0 + (i.get() % 3) as f32 * 10.0).child(label)
    };
    mount_root(&mut h.tree, virtual_list(|| 10_000, 25.0, scroll_y, row).reveal(move || want.get()));
    h.frame();
    let shown = |h: &mut Harness, row: usize| rows(h).into_iter().find(|r| r.0 == row);

    want.set(Some(7000)); // far down: every row on the way is an estimate
    h.frame();
    while h.frame_if_needed().is_some() {}
    let (_, y, height) = shown(&mut h, 7000).expect("row 7000 is mounted");
    // Whole, in the window: near its end, as far from it as the rows that came into view
    // with it turned out shorter than they counted.
    assert!(height == 20.0 && y >= 300.0 && y + height <= 600.0, "{y}");

    scroll_y.update(|y| *y += 700.0); // scrolling away afterwards is not undone
    h.frame();
    assert!(shown(&mut h, 7000).is_none_or(|(_, y, _)| y < 0.0));
    want.set(Some(6990)); // above what shows: it comes to the top
    h.frame();
    while h.frame_if_needed().is_some() {}
    assert_eq!(shown(&mut h, 6990).map(|r| r.1), Some(0.0));
    want.set(Some(20_000)); // no such row: nothing moves
    assert!(h.frame_if_needed().is_none() || shown(&mut h, 6990).map(|r| r.1) == Some(0.0));
}

#[test]
fn a_uniform_list_shows_the_row_it_is_told_to() {
    let (want, scroll_y) = (signal(None::<usize>), signal(0.0));
    let mut h = Harness::new();
    let row = |i: Signal<usize>| div().child(text(move || format!("row {}", i.get())).size(10.0));
    mount_root(&mut h.tree, uniform_list(|| 100_000, 20.0, scroll_y, row).reveal(move || want.get()));
    h.frame();
    want.set(Some(50_000));
    h.frame();
    assert_eq!(scroll_y.get_untracked(), 50_001.0 * 20.0 - 600.0); // its end at the window's
    assert!(rows(&mut h).contains(&(50_000, 580.0, 20.0)));
}

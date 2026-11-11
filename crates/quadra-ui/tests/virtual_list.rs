//! The virtual list with rows of different heights (specs/05, "Virtual lists"): rows are
//! measured as they are mounted, follow one another without a gap, and what shows stays where
//! it is when a row above it turns out taller or shorter than the estimate.

use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_ui::{div, mount_root, text, virtual_list};

/// Rows are 10, 20 and 30 px tall in turn; the list counts 25 for one it has not measured.
fn height_of(row: usize) -> f32 {
    10.0 + (row % 3) as f32 * 10.0
}

const ESTIMATE: f32 = 25.0;

/// The mounted rows in order: the index each says it is, where it is in the window, and how
/// tall it is.
fn rows(h: &mut Harness) -> Vec<(usize, f32, f32)> {
    let content = h.tree.children(h.tree.root().unwrap())[0];
    let mut rows = Vec::new();
    for wrapper in h.tree.children(content).to_vec() {
        let label = h.tree.children(h.tree.children(wrapper)[0])[0];
        let said = h.tree.widget_mut::<quadra_tree::Text>(label).unwrap().text().to_string();
        let rect = h.tree.window_rect(wrapper);
        rows.push((said.trim_start_matches("row ").parse().unwrap(), rect.y, rect.height));
    }
    rows.sort_by_key(|row| row.0);
    rows
}

/// Checks the mounted rows: one after another by index, each as tall as what it holds and
/// starting where the one before ends, and together covering the window.
fn check(h: &mut Harness, count: usize) -> Vec<(usize, f32, f32)> {
    let rows = rows(h);
    for pair in rows.windows(2) {
        let ((index, y, height), next) = (pair[0], pair[1]);
        assert_eq!((next.0, next.1), (index + 1, y + height), "{rows:?}");
    }
    assert!(rows.iter().all(|&(index, _, height)| height == height_of(index)), "{rows:?}");
    let (first, last) = (rows[0], rows[rows.len() - 1]);
    assert!(first.1 <= 0.0 && (last.1 + last.2 >= 600.0 || last.0 == count - 1), "{rows:?}");
    rows
}

fn list(count: usize) -> (Harness, Signal<usize>, Signal<f32>) {
    let (count, scroll) = (signal(count), signal(0.0));
    let mut h = Harness::new();
    let row = |i: Signal<usize>| {
        let label = text(move || format!("row {}", i.get())).size(8.0);
        div().height(move || height_of(i.get())).child(label)
    };
    mount_root(&mut h.tree, virtual_list(move || count.get(), ESTIMATE, scroll, row));
    h.frame();
    (h, count, scroll)
}

#[test]
fn rows_are_as_tall_as_what_they_hold_and_follow_one_another() {
    let (mut h, ..) = list(1000);
    let rows = check(&mut h, 1000);
    // Thirty rows fill the 600 px, and two more are kept below.
    assert_eq!((rows[0].0, rows.len()), (0, 32));
    // The list is as long as what it measured, and the estimate for the rest.
    let content = h.tree.children(h.tree.root().unwrap())[0];
    let measured: f32 = (0..32).map(height_of).sum();
    assert_eq!(h.tree.rect(content).height, measured + ESTIMATE * (1000 - 32) as f32);
    // What a row built fills its box across the list, though it asked for no width.
    let row = h.tree.children(h.tree.children(content)[0])[0];
    assert_eq!(h.tree.rect(row).width, 800.0);
}

#[test]
fn a_jump_shows_rows_measured_where_it_lands_and_the_end_is_the_end() {
    let (mut h, count, scroll) = list(1000);
    scroll.set(10_000.0);
    h.frame();
    let shown = check(&mut h, 1000);
    assert!(shown[0].0 > 300, "{shown:?}"); // far from the rows measured at the start
    scroll.set(1.0e9);
    h.frame();
    let shown = check(&mut h, 1000);
    let last = shown[shown.len() - 1];
    assert_eq!((last.0, last.1 + last.2), (999, 600.0)); // the last row ends at the bottom

    count.set(4); // fewer rows than the window holds: the list is as tall as they are
    h.frame();
    assert_eq!(rows(&mut h), [(0, 0.0, 10.0), (1, 10.0, 20.0), (2, 30.0, 30.0), (3, 60.0, 10.0)]);
}

#[test]
fn a_row_that_grows_moves_the_rows_after_it() {
    let (scroll, tall) = (signal(0.0), signal(10.0f32));
    let mut h = Harness::new();
    let row = move |i: Signal<usize>| {
        let label = text(move || format!("row {}", i.get())).size(8.0);
        div().height(move || if i.get() == 1 { tall.get() } else { 20.0 }).child(label)
    };
    mount_root(&mut h.tree, virtual_list(|| 50, 20.0, scroll, row));
    h.frame();
    assert_eq!(rows(&mut h)[1..3], [(1, 20.0, 10.0), (2, 30.0, 20.0)]);
    tall.set(100.0); // nothing enters or leaves: the row's own size says it
    h.frame();
    assert_eq!(rows(&mut h)[1..3], [(1, 20.0, 100.0), (2, 120.0, 20.0)]);
    let content = h.tree.children(h.tree.root().unwrap())[0];
    assert_eq!(h.tree.rect(content).height, 49.0 * 20.0 + 100.0);
}

#[test]
fn what_shows_stays_where_it_is_when_rows_above_turn_out_another_height() {
    let (mut h, _, scroll) = list(1000);
    scroll.set(10_000.0);
    h.frame();
    // Up, a wheel line (40 px) at a time, into rows that were never measured: each is 10, 20
    // or 30 px and counted 25. A row that shows moves down by 40 px, and by nothing else.
    for step in 0..40 {
        let before = check(&mut h, 1000);
        let seen = before.iter().copied().find(|row| row.1 >= 0.0).unwrap();
        h.wheel(100.0, 300.0, 1.0);
        h.frame();
        let after = check(&mut h, 1000);
        let now = after.iter().copied().find(|row| row.0 == seen.0).unwrap();
        assert_eq!(now.1, seen.1 + 40.0, "step {step}: row {} jumped", seen.0);
    }
    // All the way up: the first row is at the top, wherever the estimates had put it.
    for _ in 0..400 {
        h.wheel(100.0, 300.0, 3.0);
        h.frame_if_needed();
    }
    let rows = check(&mut h, 1000);
    assert_eq!((rows[0].0, rows[0].1, scroll.get_untracked()), (0, 0.0, 0.0));
}

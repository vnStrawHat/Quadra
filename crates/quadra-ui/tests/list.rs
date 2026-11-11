//! The virtual list: a hundred thousand rows cost what the window shows (specs/05, "Virtual
//! lists"; the scroll rows of specs/14's structural budgets).

use quadra_prim::Size;
use quadra_reactive::{signal, stats};
use quadra_testing::Harness;
use quadra_tree::NodeId;
use quadra_ui::{mount_root, text, uniform_list};

/// The text of every text node under `id`, sorted by where it is in the window.
fn rows(h: &mut Harness) -> Vec<String> {
    fn walk(h: &mut Harness, id: NodeId, out: &mut Vec<(f32, String)>) {
        let y = h.tree.window_rect(id).y;
        if let Some(text) = h.tree.widget_mut::<quadra_tree::Text>(id) {
            out.push((y, text.text().to_string()));
        }
        for child in h.tree.children(id).to_vec() {
            walk(h, child, out);
        }
    }
    let mut out = Vec::new();
    let root = h.tree.root().unwrap();
    walk(h, root, &mut out);
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out.into_iter().map(|(_, text)| text).collect()
}

/// Where the lowest row is in the window.
fn last_row_y(h: &mut Harness) -> f32 {
    let content = h.tree.children(h.tree.root().unwrap())[0];
    let ys = h.tree.children(content).iter().map(|&row| h.tree.window_rect(row).y);
    ys.fold(f32::MIN, f32::max)
}

/// A list of `count` rows of 20 px in the 800 x 600 window: 30 rows show, and two more are
/// kept on each side.
fn list(count: usize) -> (Harness, quadra_reactive::Signal<usize>, quadra_reactive::Signal<f32>) {
    let (count, scroll) = (signal(count), signal(0.0));
    let mut h = Harness::new();
    let view = uniform_list(
        move || count.get(),
        20.0,
        scroll,
        |i| text(move || format!("row {}", i.get())).size(10.0),
    );
    mount_root(&mut h.tree, view);
    h.frame();
    (h, count, scroll)
}

#[test]
fn only_the_rows_in_the_window_are_mounted() {
    let (mut h, ..) = list(100_000);
    let shown = rows(&mut h);
    assert_eq!((shown.len(), shown[0].as_str(), shown[31].as_str()), (32, "row 0", "row 31"));
    // The viewport, the content, the thumb of the bar, and a box and a text a row.
    assert_eq!(h.tree.len(), 3 + 32 * 2);
    let content = h.tree.children(h.tree.root().unwrap())[0];
    assert_eq!(h.tree.rect(content).height, 2_000_000.0);
}

#[test]
fn scrolling_within_the_mounted_rows_paints_nothing() {
    let (mut h, _, scroll) = list(100_000);
    scroll.set(103.0); // rows 3 to 37 are mounted
    h.frame();
    scroll.set(109.0); // the same rows
    let (stats, drawn) = h.frame();
    assert_eq!((stats.nodes_painted, stats.nodes_laid_out, stats.layers_composed), (0, 0, 1));
    assert_eq!((drawn.instances_uploaded, stats.writes), (0, 2)); // the offsets: content and thumb
}

#[test]
fn a_row_that_leaves_is_given_to_the_row_that_enters() {
    let (mut h, _, scroll) = list(100_000);
    let (nodes, before) = (h.tree.len(), stats());
    scroll.set(60.0 + 8.0); // row 0 leaves, rows 32 to 35 enter: a part of a row shows at each end
    let (frame, _) = h.frame();
    let shown = rows(&mut h);
    assert_eq!((shown[0].as_str(), shown.last().unwrap().as_str()), ("row 1", "row 35"));
    assert_eq!((h.tree.len(), stats()), (nodes + 3 * 2, before.clone_with(3))); // three more rows fit
    assert!(frame.nodes_painted <= 2 * 4 + 1, "{frame:?}"); // the four rows that came in, the thumb

    for step in 1..=50 {
        scroll.set(68.0 + step as f32 * 20.0); // a row a frame: nothing is mounted any more
        h.frame();
        assert_eq!(h.tree.len(), nodes + 6, "step {step}");
    }
    assert_eq!(stats(), before.clone_with(3));
}

#[test]
fn the_list_stops_at_its_end_and_follows_its_count() {
    let (mut h, count, scroll) = list(100_000);
    scroll.set(1.0e9); // far past the end
    h.frame();
    let shown = rows(&mut h);
    assert_eq!((shown.len(), shown.last().unwrap().as_str()), (32, "row 99999"));
    assert_eq!(last_row_y(&mut h), 580.0); // the last row ends at the bottom of the window

    count.set(3); // far fewer rows than the window holds
    h.frame();
    assert_eq!(rows(&mut h), ["row 0", "row 1", "row 2"]);
    assert_eq!(h.tree.len(), 3 + 3 * 2);
    count.set(0);
    h.frame();
    assert_eq!(h.tree.len(), 3);
}

#[test]
fn a_taller_window_shows_more_rows() {
    let (mut h, ..) = list(100_000);
    h.size = Size::new(800.0, 900.0);
    h.frame();
    assert_eq!(rows(&mut h).len(), 47); // 45 show, and two more below
    h.size = Size::new(800.0, 300.0);
    h.frame();
    assert_eq!(rows(&mut h).len(), 17);
}

trait CloneWith {
    /// The same counts with `rows` more mounted rows: a scope, a signal and a binding each.
    fn clone_with(&self, rows: usize) -> Self;
}

impl CloneWith for quadra_reactive::Stats {
    fn clone_with(&self, rows: usize) -> Self {
        quadra_reactive::Stats {
            nodes: self.nodes + rows * 2,
            scopes: self.scopes + rows,
            events: self.events,
        }
    }
}

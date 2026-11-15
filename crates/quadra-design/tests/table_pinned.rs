//! A table with a pinned column (specs/behaviour/09, "Data"): what scrolls is cut to its own
//! box, so it shows neither past the table's edge nor over the pinned column; and a bar along
//! the bottom scrolls it by the pointer.

use std::rc::Rc;

use quadra_design::{Column, dark, table};
use quadra_testing::Harness;
use quadra_tree::{NodeId, PointerInput};

/// Eight columns of 150 px in a table 500 px wide, the first pinned.
fn page() -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(|| {
        let columns = (0..8).map(|c| Column::new(format!("C{c}")).width(150.0));
        let table = table(columns, || 20, |row, column| format!("r{row}c{column}"));
        quadra_view::column().size(500.0, 400.0).child(table.pinned(1))
    });
    h.frame();
    h
}

/// The box a node is painted within: that of the parent of the nearest layer around it that
/// is cut to its parent, below the root's.
fn cut_by(h: &mut Harness, name: &str) -> Option<quadra_prim::Rect> {
    let mut at: NodeId = h.get_by_text(name).node();
    while let Some(parent) = h.tree.parent(at) {
        if h.tree.is_layer(at) {
            return Some(h.tree.window_rect(parent));
        }
        at = parent;
    }
    None
}

#[test]
fn what_scrolls_is_cut_to_its_box_and_a_bar_scrolls_it() {
    let mut h = page();
    // The titles and the cells that scroll start where the pinned column ends.
    for name in ["C7", "C1", "r0c3"] {
        let cut = cut_by(&mut h, name).expect("cut to a box");
        assert!((cut.x - 150.0).abs() <= 1.0 && (cut.right() - 500.0).abs() <= 1.0, "{name}: {cut:?}");
    }

    // A drag of the thumb along the bottom moves the columns that scroll, not the pinned one.
    let x = |h: &mut Harness, name: &str| {
        let node = h.get_by_text(name).node();
        h.tree.window_rect(node).x
    };
    let (pinned, second) = (x(&mut h, "r0c0"), x(&mut h, "r0c1"));
    h.pointer(PointerInput::Move, 60.0, 393.0);
    h.pointer(PointerInput::Down, 60.0, 393.0);
    h.pointer(PointerInput::Move, 160.0, 393.0);
    h.pointer(PointerInput::Up, 160.0, 393.0);
    h.frame();
    assert_eq!(x(&mut h, "r0c0"), pinned);
    assert!(second - x(&mut h, "r0c1") > 100.0, "the columns scrolled");
}

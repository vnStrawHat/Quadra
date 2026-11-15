//! The view of a pane tree (specs/13, "The workbench"): leaves and seams placed where the
//! tree puts them, seams dragged, and leaves kept while the tree changes around them.

use std::rc::Rc;

use quadra_design::dark;
use quadra_design::work::{Axis, Pane, panes};
use quadra_prim::Rect;
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{NodeId, PointerInput, Role};
use quadra_view::{column, div};

struct Page {
    h: Harness,
    root: Signal<Pane>,
    built: Signal<u32>,
}

/// "a" beside "b" over "c", in a box of 806 by 406: a quarter for "a", halves for the others.
fn page() -> Page {
    let stacked = Pane::column(Pane::leaf("b"), Pane::leaf("c"), 0.5);
    let (root, built) = (signal(Pane::row(Pane::leaf("a"), stacked, 0.25)), signal(0));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let leaf = move |group: &str| {
            built.update(|n| *n += 1);
            column().grow(1.0).role(Role::Group).label(group.to_string())
        };
        quadra_design::root().child(div().size(806.0, 406.0).child(panes(root, leaf).header(28.0)))
    });
    let mut page = Page { h, root, built };
    page.settle();
    page
}

impl Page {
    fn settle(&mut self) {
        self.h.frame();
        while self.h.frame_if_needed().is_some() {}
    }

    fn leaf(&mut self, group: &str) -> (NodeId, Rect) {
        let node = self.h.get_by_role(Role::Group, group).node();
        (node, self.h.tree.window_rect(node))
    }
}

#[test]
fn the_parts_of_a_tree_are_placed_and_a_seam_is_dragged() {
    let mut p = page();
    let rects = |p: &mut Page| ["a", "b", "c"].map(|group| p.leaf(group).1);
    let at = |x, y, w, h| Rect::new(x, y, w, h);
    assert_eq!(
        rects(&mut p),
        [at(0.0, 0.0, 201.0, 406.0), at(207.0, 0.0, 599.0, 203.0), at(207.0, 209.0, 599.0, 197.0)]
    );

    // The seam beside "a" is held and moved 100 px: the share follows, and nothing is built.
    p.h.pointer(PointerInput::Down, 204.0, 100.0);
    p.h.pointer(PointerInput::Move, 304.0, 100.0);
    p.settle();
    let share = p.root.with_untracked(|tree| tree.ratio(&[])).unwrap();
    assert!((share - (0.25 + 100.0 / 806.0)).abs() < 1e-4, "{share}");
    assert_eq!((p.leaf("a").1.width, p.built.get_untracked()), ((share * 806.0).floor(), 3));
    // Never under 80 px for a side.
    p.h.pointer(PointerInput::Move, 2.0, 100.0);
    p.settle();
    p.h.pointer(PointerInput::Up, 2.0, 100.0);
    assert_eq!(p.leaf("a").1.width, 80.0);

    // A double click on a seam makes its sides even: the one between "b" and "c".
    p.h.click(400.0, 206.0);
    p.h.click(400.0, 206.0);
    p.settle();
    assert_eq!(p.root.with_untracked(|tree| tree.ratio(&[1])), Some(0.5));
    p.root.update(|tree| tree.set_ratio(&[1], 0.25));
    p.settle();
    assert_eq!(p.leaf("b").1.height, 101.0);
}

#[test]
fn a_leaf_is_kept_while_the_tree_changes_around_it() {
    let mut p = page();
    let before = ["a", "b", "c"].map(|group| p.leaf(group).0);
    // A new group beside "b": only its leaf is built.
    p.root.update(|tree| tree.split("b", Axis::Row, "d", false));
    p.settle();
    assert_eq!((["a", "b", "c"].map(|group| p.leaf(group).0), p.built.get_untracked()), (before, 4));
    let (b, d) = (p.leaf("b").1, p.leaf("d").1);
    assert_eq!((b.width, d.x, d.right()), (299.0, b.right() + 6.0, 806.0));

    // Folded, "c" keeps its header under a line of one pixel, and what is over it has the rest.
    p.root.update(|tree| tree.toggle("c"));
    p.settle();
    let (b, c) = (p.leaf("b").1, p.leaf("c").1);
    assert_eq!((b.height, c.y, c.height), (377.0, 378.0, 28.0));

    // Taken out, "a" goes with what it held; the others are the nodes they were.
    p.root.update(|tree| tree.remove("a"));
    p.settle();
    assert!(p.h.query(|n| n.semantics.role == Role::Group && n.semantics.name == "a").is_empty());
    assert_eq!(
        (["b", "c"].map(|group| p.leaf(group).0), p.built.get_untracked()),
        ([before[1], before[2]], 4)
    );
    assert_eq!(p.leaf("b").1.x, 0.0);
}

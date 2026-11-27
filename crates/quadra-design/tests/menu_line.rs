//! A menu's row is one line (specs/13, "Menu"; issue #49): in a menu of a width of its own a
//! long label, or a long detail, is cut, the keys are whole, and nothing of a row is outside
//! it, over the rows beside it.

use std::rc::Rc;

use quadra_design::{MenuItem, button, dark, searched_menu};
use quadra_prim::Rect;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{NodeId, Role, Side};
use quadra_view::{NodeRef, column, div};

/// A target whose user is a token of 40 letters, and when it was last used.
const TARGET: &str = "0123456789abcdef0123456789abcdef01234567@host.example.com:4422";
const WHEN: &str = "14 min ago";
const DETAIL: &str = "a detail of so many words that a menu of this width has no room for them all";

/// The rects of everything under a node.
fn inside(h: &Harness, node: NodeId, all: &mut Vec<Rect>) {
    for &child in h.tree.children(node) {
        all.push(h.tree.window_rect(child));
        inside(h, child, all);
    }
}

#[test]
fn a_row_s_long_label_and_detail_are_cut_and_nothing_of_it_is_outside_it() {
    let open = signal(false);
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let at = NodeRef::new();
        let items = move |_: &str| {
            vec![
                MenuItem::new("Local"),
                MenuItem::new(TARGET).ellipsis_start().keys(WHEN),
                MenuItem::new("Saved").detail(DETAIL).keys(WHEN),
                MenuItem::new(TARGET.to_uppercase()),
            ]
        };
        let opener = div().node_ref(&at).child(button("New").on_click(move || open.set(true)));
        let menu = searched_menu(open, &at, Side::Below, "Search", items);
        quadra_design::root().child(column().child(opener).child(menu))
    });
    h.frame();
    h.get_by_role(Role::Button, "New").click();
    h.frame();
    while h.frame_if_needed().is_some() {}
    // To a reader a row's name is its whole label, cut or not.
    for name in [TARGET.to_string(), "Saved".to_string(), TARGET.to_uppercase()] {
        let row = h.get_by_role(Role::MenuItem, &name).node();
        let (face, mut parts) = (h.tree.window_rect(row), Vec::new());
        inside(&h, row, &mut parts);
        assert_eq!(face.height, 28.0, "{name}");
        assert!(parts.len() >= 2, "{name}: {parts:?}");
        for part in parts {
            let within = part.y >= face.y && part.bottom() <= face.bottom() && part.right() <= face.right();
            assert!(within, "{name}: {part:?} is not in {face:?}");
        }
    }
    // The keys are whole, on one line: as wide in the row of the long label as in another.
    let whens = h.query(|n| n.semantics.name == WHEN);
    let widths: Vec<f32> = whens.iter().map(|&node| h.tree.window_rect(node).width).collect();
    assert!(widths.len() == 2 && widths[0] == widths[1] && widths[0] > 0.0, "{widths:?}");
}

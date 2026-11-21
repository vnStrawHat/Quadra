//! The marks of a status (specs/13, "A status"): running and pending are squares told apart
//! by their fill, and `Status::None` keeps the room of a mark.

use std::rc::Rc;

use quadra_design::{Status, dark, status_dots, status_label_sized, status_mark};
use quadra_prim::Size;
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::row;

#[test]
fn running_and_pending_are_squares_told_apart_by_their_fill_and_none_keeps_its_room() {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(|| {
        let marks = [Status::Running, Status::Pending, Status::None].map(status_mark);
        row().role(Role::Group).label("marks").children(marks)
    });
    h.frame();
    let marks = h.get_by_role(Role::Group, "marks").node();
    let marks = h.tree.children(marks).to_vec();
    let shape = |h: &mut Harness, mark| {
        let inner = h.tree.children(mark)[0];
        (h.tree.window_rect(inner).size(), h.tree.paint_style(inner))
    };
    let ((running, filled), (pending, hollow)) = (shape(&mut h, marks[0]), shape(&mut h, marks[1]));
    assert_eq!((running, pending), (Size::new(7.0, 7.0), Size::new(7.0, 7.0)));
    assert!(filled.background.a > 0.0 && filled.border_color.a == 0.0, "{filled:?}");
    assert!(hollow.background.a == 0.0 && hollow.border_color.a > 0.0, "{hollow:?}");
    // None: the 12 px box of a mark, empty.
    assert_eq!(h.tree.window_rect(marks[2]).size(), Size::new(12.0, 12.0));
    assert!(h.tree.children(marks[2]).is_empty());
}

#[test]
fn marks_come_at_other_sizes_and_in_a_row_told_as_counts() {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(|| {
        let runs = [[Status::Succeeded; 8].as_slice(), &[Status::Failed; 2]].concat();
        row().child(status_dots(runs, 8.0)).child(status_label_sized(Status::Running, "Running", 10.0))
    });
    h.frame();
    let dots = h.get_by_role(Role::Image, "8 succeeded, 2 failed").node();
    let marks = h.tree.children(dots).to_vec();
    let (first, second) = (h.tree.window_rect(marks[0]), h.tree.window_rect(marks[1]));
    assert_eq!((marks.len(), first.size(), second.x - first.right()), (10, Size::new(8.0, 8.0), 3.0));
    // A mark of 10 px before a word: its square scaled with it.
    let label = h.get_by_text("Running").node();
    let mark = h.tree.children(h.tree.parent(label).unwrap())[0];
    let square = h.tree.children(mark)[0];
    assert_eq!(h.tree.window_rect(mark).size(), Size::new(10.0, 10.0));
    assert!((h.tree.window_rect(square).width - 70.0 / 12.0).abs() < 1e-4);
}

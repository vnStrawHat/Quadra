//! A table's columns that fill the room left (specs/13, the table): by weight, never under
//! their widths, the last column at the table's edge; fixed once their edge is dragged.

use std::rc::Rc;

use quadra_design::{Column, dark, table};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role};

#[test]
fn columns_that_fill_share_the_room_left_by_weight_until_one_is_dragged() {
    let hidden = signal(Vec::<usize>::new());
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let columns = [
            Column::new("Name").width(100.0).fill(),
            Column::new("Message").width(100.0).fill_by(2.0),
            Column::new("Age").width(80.0),
        ];
        let cell = |r: usize, c: usize| format!("{r}-{c}");
        quadra_design::root().child(table(columns, || 3, cell).hidden(hidden))
    });
    h.frame();
    // A column: its title and the grip of 5 px at its edge.
    let rect = |h: &mut Harness, role: Role, name: &str| {
        let node = h.get_by_role(role, name).node();
        let at = h.tree.window_rect(node);
        if role == Role::Cell { quadra_prim::Rect::new(at.x, at.y, at.width + 5.0, at.height) } else { at }
    };
    let all = rect(&mut h, Role::Table, "");
    let (name, message, age) = (
        rect(&mut h, Role::Cell, "Name"),
        rect(&mut h, Role::Cell, "Message"),
        rect(&mut h, Role::Cell, "Age"),
    );
    // What is left past the widths, 1 to 2; Age where the table ends.
    let free = all.width - 280.0;
    assert!(
        (name.width - (100.0 + free / 3.0)).abs() < 0.5
            && (message.width - (100.0 + free * 2.0 / 3.0)).abs() < 0.5,
        "{name:?} {message:?} {age:?} in {all:?}"
    );
    assert!((age.right() - all.right()).abs() < 0.5 && age.width == 80.0, "{age:?} in {all:?}");
    // A hidden column gives them its room.
    hidden.set(vec![2]);
    h.frame();
    let wider = rect(&mut h, Role::Cell, "Message");
    assert!((wider.right() - all.right()).abs() < 0.5, "{wider:?}");
    hidden.set(Vec::new());
    h.frame();

    // Dragged, Name is fixed where it was dragged to; Message takes the change, Age stays at the edge.
    let (x, y) = (name.right() - 2.0, name.center().y);
    h.pointer(PointerInput::Move, x, y);
    h.pointer(PointerInput::Down, x, y);
    h.pointer(PointerInput::Move, x - 50.0, y);
    h.pointer(PointerInput::Up, x - 50.0, y);
    h.frame();
    let (dragged, taken, age) = (
        rect(&mut h, Role::Cell, "Name"),
        rect(&mut h, Role::Cell, "Message"),
        rect(&mut h, Role::Cell, "Age"),
    );
    assert!((dragged.width - (name.width - 50.0)).abs() < 0.5, "{dragged:?} from {name:?}");
    assert!((taken.width - (message.width + 50.0)).abs() < 0.5 && (age.right() - all.right()).abs() < 0.5);
}

#[test]
fn with_columns_pinned_a_column_that_fills_keeps_its_width() {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let columns = [Column::new("Name").width(100.0).fill(), Column::new("Message").width(100.0).fill()];
        quadra_design::root().child(table(columns, || 3, |r, c| format!("{r}-{c}")).pinned(1))
    });
    h.frame();
    // Their titles, and the grip of 5 px at the edge of each: the pinned one too.
    for name in ["Name", "Message"] {
        let title = h.get_by_role(Role::Cell, name).node();
        assert_eq!(h.tree.window_rect(title).width + 5.0, 100.0, "{name}");
    }
}

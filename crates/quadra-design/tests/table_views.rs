//! A table's cells that show views (specs/13, "What a cell shows"): built once for each row
//! that is mounted, bound to the row and what the cell says, and a button in one takes its
//! press from the row.

use std::cell::Cell;
use std::rc::Rc;

use quadra_design::{Column, button, dark, table};
use quadra_platform::Mods;
use quadra_prim::Point;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::{row, text};

#[test]
fn a_cell_shows_a_view_bound_to_its_row_and_a_button_in_it_does_not_select_the_row() {
    let (built, revealed, chosen) = (Rc::new(Cell::new(0)), signal(Vec::<usize>::new()), signal(Vec::new()));
    let names = signal((0..500).map(|i| format!("key-{i:03}")).collect::<Vec<_>>());
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    let count = built.clone();
    h.mount(move || {
        let value = Column::new("Value").view(move |at, says| {
            count.set(count.get() + 1);
            let reveal = button("Reveal")
                .on_click(move || revealed.update(|all| all.push(quadra_reactive::untrack(|| at.get()))));
            row().gap(8.0).child(text(move || format!("{} bytes", says.get().len()))).child(reveal)
        });
        let columns = [Column::new("Key").mono(), value, Column::new("Note").muted()];
        let cell = move |r: usize, c: usize| match c {
            0 | 1 => names.with(|all| all[r].clone()),
            _ => "set by the operator".to_string(),
        };
        quadra_design::root().child(table(columns, move || names.with(Vec::len), cell).selected(chosen))
    });
    h.frame();
    let first = built.get();
    assert!(first > 0 && h.has_text("7 bytes") && h.has_text("set by the operator"));

    // The button's press reveals its row's key, and does not select the row.
    let reveal = h.query(|n| n.semantics.role == Role::Button && n.semantics.name == "Reveal")[0];
    let at = h.tree.window_rect(reveal).center();
    h.click(at.x, at.y);
    h.frame();
    assert_eq!((revealed.get_untracked(), chosen.get_untracked()), (vec![0], vec![]));
    // A press elsewhere in the row selects it.
    let key = h.query(|n| n.semantics.name == "key-002")[0];
    let at = h.tree.window_rect(key).center();
    h.click(at.x, at.y);
    h.frame();
    assert_eq!(chosen.get_untracked(), vec![2]);

    // Scrolled, the rows are reused: no view is built again, and they show their new rows.
    h.tree.wheel(Point::new(0.0, -2000.0), false, Point::new(300.0, 300.0), Mods::NONE);
    h.frame();
    assert!(h.query(|n| n.semantics.name == "key-000").is_empty() && built.get() == first);
    // What a cell says changes: its view follows, and is not built again.
    names.update(|all| all.iter_mut().for_each(|name| name.push_str("-long")));
    h.frame();
    assert!(h.has_text("12 bytes") && built.get() == first, "{}", h.semantics());
}

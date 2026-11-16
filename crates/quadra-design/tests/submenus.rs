//! A submenu chosen by the pointer (specs/05, "Overlays"): a press in it is inside the menu it
//! stands beside, so the choice is made, and the focus goes back to what opened the menu.

use std::rc::Rc;

use quadra_design::{MenuItem, button, dark, searched_menu};
use quadra_platform::{Mods, MouseButton};
use quadra_prim::Point;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role, Side};
use quadra_view::{NodeRef, column, div};

fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {}
}

fn rect(h: &mut Harness, name: &str) -> quadra_prim::Rect {
    let row = h.get_by_role(Role::MenuItem, name).node();
    h.tree.window_rect(row)
}

#[test]
fn a_row_of_a_submenu_is_chosen_by_a_click() {
    let (open, done) = (signal(false), signal(Vec::<String>::new()));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let at = NodeRef::new();
        let shell = MenuItem::new("cmd").on_select(move || done.update(|all| all.push("cmd".into())));
        let items = move |_: &str| {
            let others = MenuItem::new("Other shells").submenu([shell.clone(), MenuItem::new("pwsh")]);
            vec![MenuItem::new("Local"), others, MenuItem::new("Quick")]
        };
        let opener = div().node_ref(&at).child(button("New").on_click(move || open.set(true)));
        let menu = searched_menu(open, &at, Side::Below, "Search", items);
        quadra_design::root().child(column().child(opener).child(menu))
    });
    settle(&mut h);
    h.get_by_role(Role::Button, "New").focus().click();
    settle(&mut h);
    // Over the row, then across to the first row of its menu, and a click there.
    let row = rect(&mut h, "Other shells");
    h.pointer(PointerInput::Move, row.center().x, row.center().y);
    settle(&mut h);
    let shell = rect(&mut h, "cmd");
    for k in 0..=4 {
        let x = row.center().x + (shell.center().x - row.center().x) * k as f32 / 4.0;
        h.pointer(PointerInput::Move, x, shell.center().y);
        settle(&mut h);
    }
    let at = Point::new(shell.center().x, shell.center().y);
    for input in [PointerInput::Down, PointerInput::Up] {
        h.tree.pointer(input, MouseButton::Left, at, Mods::NONE, h.now());
        settle(&mut h);
    }
    assert_eq!(done.get_untracked(), ["cmd"]);
    assert_eq!(h.query(|n| n.semantics.role == Role::Menu).len(), 0);
    let new = h.get_by_role(Role::Button, "New").node();
    assert_eq!(h.tree.focused(), Some(new), "back to what opened the menu");
}

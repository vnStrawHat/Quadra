//! More of the menu (specs/behaviour/15, "Menu"): a menu at the pointer for the other button,
//! a row that opens a menu beside itself, the names of groups, and a menu too long to show
//! whole.

use std::rc::Rc;

use quadra_design::{MenuItem, button, context_menu, dark, menu};
use quadra_platform::{Key, Mods, MouseButton};
use quadra_prim::Point;
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role};
use quadra_view::{NodeRef, column, div, text};

struct Page {
    h: Harness,
    open: Signal<bool>,
    done: Signal<Vec<String>>,
}

/// A box with a menu for the other button, a button "File" whose menu has a row that opens
/// another, and a button "Long" with a menu of forty rows under a heading.
fn page() -> Page {
    let (open, long, done) = (signal(false), signal(false), signal(Vec::new()));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let did = move |what: &str| {
            let what = what.to_string();
            move || done.update(|all| all.push(what.clone()))
        };
        let on_box = move || vec![MenuItem::new("Rename").on_select(did("rename")), MenuItem::new("Delete")];
        let area = context_menu(div().size(300.0, 100.0).focusable().child(text("a file")), on_box);
        let (file_at, long_at) = (NodeRef::new(), NodeRef::new());
        let share = move || vec![MenuItem::new("Mail").on_select(did("mail")), MenuItem::new("Link")];
        let file = move || vec![MenuItem::new("Open"), MenuItem::new("Share").submenu(share())];
        let many = move || {
            let rows = (0..40).map(|i| MenuItem::new(format!("Row {i}")).on_select(did(&format!("row {i}"))));
            std::iter::once(MenuItem::heading("Rows")).chain(rows).collect()
        };
        let buttons = column()
            .child(div().node_ref(&file_at).child(button("File").on_click(move || open.set(true))))
            .child(div().node_ref(&long_at).child(button("Long").on_click(move || long.set(true))));
        let menus = column().child(menu(open, &file_at, file)).child(menu(long, &long_at, many));
        quadra_design::root().child(area).child(buttons).child(menus)
    });
    h.frame();
    Page { h, open, done }
}

impl Page {
    fn settle(&mut self) {
        self.h.frame();
        while self.h.frame_if_needed().is_some() {}
    }

    fn other_click(&mut self, x: f32, y: f32) {
        for input in [PointerInput::Move, PointerInput::Down, PointerInput::Up] {
            self.h.tree.pointer(input, MouseButton::Right, Point::new(x, y), Mods::NONE, self.h.now());
        }
        self.settle();
    }

    fn key(&mut self, key: Key) {
        self.h.key(key, Mods::NONE);
        self.settle();
    }

    /// Where a row of a menu is in the window.
    fn rect(&mut self, name: &str) -> quadra_prim::Rect {
        let row = self.h.get_by_role(Role::MenuItem, name).node();
        self.h.tree.window_rect(row)
    }

    fn menus(&self) -> usize {
        self.h.query(|n| n.semantics.role == Role::Menu).len()
    }
}

#[test]
fn the_other_button_opens_a_menu_at_the_pointer() {
    let mut p = page();
    let nodes = p.h.tree.len();
    p.h.click(50.0, 50.0); // the main button opens nothing
    p.settle();
    assert_eq!(p.menus(), 0);
    p.other_click(50.0, 40.0);
    let list = p.h.get_by_role(Role::Menu, "").node();
    let panel = p.h.tree.window_rect(p.h.tree.overlays().next().unwrap());
    // Under the pointer by 4 px, with the focus in it.
    assert_eq!((panel.x, panel.y, p.h.tree.focused()), (50.0, 44.0, Some(list)));
    p.h.get_by_role(Role::MenuItem, "Rename").click();
    p.settle();
    assert_eq!((p.menus(), p.done.get_untracked(), p.h.tree.len()), (0, vec!["rename".to_string()], nodes));

    // Escape and a press elsewhere give it up; near the edge it opens where it fits.
    p.other_click(295.0, 95.0);
    p.key(Key::Escape);
    assert_eq!((p.menus(), p.h.tree.len()), (0, nodes));
    p.other_click(790.0, 50.0); // not over the box: no menu
    assert_eq!(p.menus(), 0);
    p.other_click(295.0, 95.0);
    p.h.click(600.0, 500.0);
    p.settle();
    assert_eq!((p.menus(), p.h.tree.len()), (0, nodes));

    // The menu key and Shift-F10 open it under what has the focus.
    p.h.click(50.0, 50.0);
    p.settle();
    p.key(Key::Menu);
    let panel = p.h.tree.window_rect(p.h.tree.overlays().next().unwrap());
    assert_eq!((p.menus(), panel.x, panel.y), (1, 0.0, 104.0));
    p.key(Key::Escape);
    p.h.key(Key::F10, Mods::SHIFT);
    p.settle();
    assert_eq!(p.menus(), 1);
}

#[test]
fn a_row_opens_a_menu_beside_itself_for_the_pointer_and_the_arrows() {
    let mut p = page();
    p.open.set(true);
    p.settle();
    assert!(p.menus() == 1 && !p.h.has_text("Mail"));
    // The pointer over the row opens its menu, to the right of the row and over it by 2 px.
    let row = p.rect("Share");
    p.h.pointer(PointerInput::Move, row.center().x, row.center().y);
    p.settle();
    let inner = p.h.tree.window_rect(p.h.tree.overlays().last().unwrap());
    assert_eq!((p.menus(), inner.x, inner.y), (2, row.right() - 2.0, row.y));
    // Over another row it closes; a choice in it ends both menus.
    let open_row = p.rect("Open").center();
    p.h.pointer(PointerInput::Move, open_row.x, open_row.y);
    p.settle();
    assert_eq!(p.menus(), 1);
    p.h.pointer(PointerInput::Move, row.center().x, row.center().y);
    p.settle();
    p.h.get_by_role(Role::MenuItem, "Mail").click();
    p.settle();
    assert_eq!(
        (p.menus(), p.open.get_untracked(), p.done.get_untracked()),
        (0, false, vec!["mail".to_string()])
    );

    // By the keys: down to the row, right into its menu, left out of it, and in again to choose.
    p.h.pointer(PointerInput::Move, 700.0, 500.0);
    p.open.set(true);
    p.settle();
    let outer = p.h.tree.focused();
    p.key(Key::Down);
    p.key(Key::Down);
    p.key(Key::Right);
    assert!(p.menus() == 2 && p.h.tree.focused() != outer);
    p.key(Key::Left);
    assert_eq!((p.menus(), p.h.tree.focused()), (1, outer));
    p.key(Key::Enter);
    p.key(Key::Down);
    p.key(Key::Enter);
    assert_eq!((p.menus(), p.done.get_untracked().len()), (0, 2));
}

#[test]
fn a_long_menu_scrolls_and_a_heading_is_not_chosen() {
    let mut p = page();
    p.h.get_by_role(quadra_tree::Role::Button, "Long").click();
    p.settle();
    assert!(p.h.has_text("Rows"));
    // 450 px of it show, in a panel with its border.
    let panel = p.h.tree.window_rect(p.h.tree.overlays().next().unwrap());
    assert_eq!(panel.height, 452.0);
    // The arrows skip the heading; up from nothing is the last row, brought into view.
    p.key(Key::Down);
    assert!(p.h.semantics().contains("MenuItem \"Row 0\" [selected]"));
    p.key(Key::Up);
    assert!(p.h.semantics().contains("MenuItem \"Row 39\" [selected]"));
    let last = p.rect("Row 39");
    assert!(last.bottom() <= panel.bottom() && last.y >= panel.y, "{last:?} in {panel:?}");
    p.key(Key::Enter);
    assert_eq!(p.done.get_untracked(), ["row 39"]);
}

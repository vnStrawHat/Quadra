//! Popups of the kit (specs/13, "Overlays in the kit"): a menu opens beside what opened it,
//! takes the focus and gives it back, and closes when a row is chosen, on Escape, and on a
//! press outside. A select is a button with such a menu.

use std::rc::Rc;

use quadra_design::{MenuItem, button, dark, menu, select};
use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::{NodeRef, column, div, text};

struct Page {
    h: Harness,
    open: Signal<bool>,
    done: Signal<Vec<&'static str>>,
    size: Signal<usize>,
}

/// A button "Edit" with a menu of three rows and a line, a select of three sizes, and
/// something else to press.
fn page() -> Page {
    let (open, done, size) = (signal(false), signal(Vec::new()), signal(1usize));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let opener = NodeRef::new();
        let did = move |what: &'static str| move || done.update(|all| all.push(what));
        let items = move || {
            vec![
                MenuItem::new("Copy").keys("Ctrl-C").on_select(did("copy")),
                MenuItem::new("Paste").disabled(true).on_select(did("paste")),
                MenuItem::separator(),
                MenuItem::new("Wrap").checked(true).on_select(did("wrap")),
            ]
        };
        let edit = button("Edit").test_id("edit").on_click(move || open.update(|open| *open = !*open));
        let edit = div().node_ref(&opener).child(edit);
        let sizes = select(["Small", "Medium", "Large"], size).test_id("size");
        let page = column().gap(8.0).child(edit).child(menu(open, &opener, items)).child(sizes);
        quadra_design::root().child(page).child(div().size(200.0, 200.0).child(text("elsewhere")))
    });
    h.frame();
    Page { h, open, done, size }
}

impl Page {
    fn settle(&mut self) {
        self.h.frame();
        while self.h.frame_if_needed().is_some() {}
    }
}

#[test]
fn a_menu_opens_beside_its_button_and_closes_when_a_row_is_chosen() {
    let mut p = page();
    assert!(!p.h.semantics().contains("Menu"));
    p.h.get_by_test_id("edit").click();
    p.settle();
    let tree = p.h.semantics();
    for line in ["Menu", "MenuItem \"Copy\"", "MenuItem \"Paste\" [disabled]", "MenuItem \"Wrap\" [checked]"]
    {
        assert!(tree.contains(line), "{line} is not in:\n{tree}");
    }
    // Below the button, 4 px off it, and at least 200 wide with its border; the focus is in the menu.
    let (edit, list) = (p.h.get_by_test_id("edit").node(), p.h.get_by_role(Role::Menu, "").node());
    let (button, shown) = (p.h.tree.window_rect(edit), p.h.tree.window_rect(list));
    assert!(shown.y >= button.bottom() + 4.0 && shown.width >= 198.0, "{shown:?}");
    assert_eq!(p.h.tree.focused(), Some(list));
    // The text of the window holds in the popup too: its face, its size, its colour.
    let copy = p.h.get_by_role(Role::MenuItem, "Copy").node();
    let at = p.h.tree.inherited(copy);
    assert_eq!((&*at.style.family, at.style.size), (quadra_design::fonts::UI, 14.0));
    assert_eq!(at.color, dark().color(quadra_design::token::TEXT));

    // A click on a row does what the row does, and closes the menu; the focus is back.
    p.h.get_by_role(Role::MenuItem, "Copy").click();
    p.settle();
    assert_eq!((p.done.get_untracked(), p.open.get_untracked()), (vec!["copy"], false));
    assert!(!p.h.semantics().contains("Menu"));
    assert_eq!(p.h.tree.focused(), Some(edit));
}

#[test]
fn the_keyboard_and_presses_elsewhere_close_a_menu() {
    let mut p = page();
    let edit = p.h.get_by_test_id("edit").node();
    p.h.tree.focus(edit);
    p.h.key(Key::Enter, Mods::NONE); // the button opens the menu
    p.settle();
    // Down, down: the disabled row and the line are passed by. Enter chooses.
    p.h.key(Key::Down, Mods::NONE);
    p.h.key(Key::Down, Mods::NONE);
    p.h.frame();
    assert!(p.h.semantics().contains("MenuItem \"Wrap\" [checked] [selected]"), "{}", p.h.semantics());
    p.h.key(Key::Enter, Mods::NONE);
    p.settle();
    assert_eq!((p.done.get_untracked(), p.open.get_untracked()), (vec!["wrap"], false));
    assert_eq!(p.h.tree.focused(), Some(edit)); // back where it was

    // Escape gives the menu up; so does a press elsewhere; a press on the button that
    // opened it closes it, and does not open it again.
    for close in 0..3 {
        p.h.get_by_test_id("edit").click();
        p.settle();
        assert!(p.open.get_untracked(), "it opens, time {close}");
        let button = p.h.tree.window_rect(edit).center();
        match close {
            0 => p.h.key(Key::Escape, Mods::NONE),
            1 => p.h.click(600.0, 500.0),
            _ => p.h.click(button.x, button.y),
        }
        p.settle();
        assert!(!p.open.get_untracked() && !p.h.semantics().contains("Menu"), "it closes, time {close}");
    }
    assert_eq!(p.done.get_untracked(), vec!["wrap"]); // nothing was chosen on the way
}

#[test]
fn a_select_shows_what_is_chosen_and_a_menu_of_the_options() {
    let mut p = page();
    assert!(p.h.semantics().contains("Button \"Medium\" [collapsed] #size"), "{}", p.h.semantics());
    p.h.get_by_test_id("size").click();
    p.settle();
    let tree = p.h.semantics();
    assert!(
        tree.contains("Button \"Medium\" [expanded]") && tree.contains("MenuItem \"Medium\" [checked]"),
        "{tree}"
    );
    p.h.get_by_role(Role::MenuItem, "Large").click();
    p.settle();
    assert_eq!(p.size.get_untracked(), 2);
    // Closed, showing the new choice, and with the focus back on it.
    assert!(p.h.semantics().contains("Button \"Large\" [collapsed] [focused] #size"), "{}", p.h.semantics());
}

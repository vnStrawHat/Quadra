//! The components by the names of their widgets (specs/17, "Widget names"), on bare parts:
//! each is the widget it is named to a reader, and does what its behaviour does.

use std::rc::Rc;

use quadra_headless as headless;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::{column, div, text};

#[test]
fn each_widget_name_is_that_widget_to_a_reader() {
    let (saved, mail, wifi, size, page, open) =
        (signal(0), signal(false), signal(false), signal(0usize), signal(0usize), signal(false));
    let mut h = Harness::new();
    h.tree.set_keymap(Rc::new(headless::keymap()));
    h.mount(move || {
        let button =
            headless::button(div().label("Save")).on_click(move || saved.update(|n| *n += 1)).build();
        let checkbox = headless::checkbox(mail, div().label("Mail"), false);
        let switch = headless::switch(wifi, div().label("Wifi"), false);
        let radios = headless::radio_group(div().label("Size"), 2, size)
            .child(headless::radio(div().label("Small"), 0, size))
            .child(headless::radio(div().label("Large"), 1, size));
        let tabs = headless::tabs(div().label("Pages"), 2, page)
            .child(headless::tab(div().label("One"), 0, page))
            .child(headless::tab(div().label("Two"), 1, page));
        let head = headless::accordion_head(div().label("More"), open);
        let body = headless::accordion_body(open, || text("Inside"));
        column().child(button).child(checkbox).child(switch).child(radios).child(tabs).child(head).child(body)
    });
    h.frame();

    h.get_by_role(Role::Button, "Save").click();
    h.get_by_role(Role::Checkbox, "Mail").click();
    h.get_by_role(Role::Switch, "Wifi").click();
    h.get_by_role(Role::Radio, "Large").click();
    h.get_by_role(Role::Tab, "Two").click();
    h.get_by_role(Role::Button, "More").click();
    h.frame();
    let state = (saved.get_untracked(), mail.get_untracked(), wifi.get_untracked());
    assert_eq!(state, (1, true, true));
    assert_eq!((size.get_untracked(), page.get_untracked(), open.get_untracked()), (1, 1, true));
    // Each is found once, or the query fails.
    h.get_by_role(Role::TabList, "Pages");
    h.get_by_role(Role::Group, "Size");
    h.get_by_text("Inside");
}

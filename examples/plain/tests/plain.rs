//! The second kit through the semantic tree (specs/17, "Tests"): each control is found by its
//! role and driven as a reader or a user would.

use std::rc::Rc;

use quadra::{Key, Mods, Role, SemanticAction, View, signal};
use quadra_plain::{Act, button, checkbox, dialog, list, radio_group, root, select, slider, switch, tabs};
use quadra_testing::Harness;

fn mounted<V: View + 'static>(view: impl FnOnce() -> V) -> Harness {
    let mut h = Harness::new();
    h.tree.set_keymap(Rc::new(quadra_base::keymap()));
    h.mount(view);
    h.frame();
    h
}

#[test]
fn a_button_and_the_toggles_do_what_they_say() {
    let (count, on, lit) = (signal(0), signal(false), signal(false));
    let mut h = mounted(move || {
        let page = root().child(button("Add", move || count.update(|n| *n += 1)));
        page.child(checkbox(on, "Lines")).child(switch(lit, "Light"))
    });
    h.get_by_role(Role::Button, "Add").click();
    h.get_by_role(Role::Checkbox, "Lines").click();
    h.get_by_role(Role::Switch, "Light").act(SemanticAction::Click);
    h.frame();
    assert_eq!((count.get_untracked(), on.get_untracked(), lit.get_untracked()), (1, true, true));
    assert!(h.semantics().contains("Checkbox \"Lines\" [checked]"), "{}", h.semantics());
}

#[test]
fn radios_and_tabs_choose_by_a_click_and_the_arrows() {
    let (size, page) = (signal(0), signal(0));
    let mut h =
        mounted(move || root().child(radio_group(&["S", "M", "L"], size)).child(tabs(&["One", "Two"], page)));
    h.get_by_role(Role::Radio, "L").click();
    h.get_by_role(Role::Tab, "Two").click();
    h.frame();
    assert_eq!((size.get_untracked(), page.get_untracked()), (2, 1));
    let group = h.get_by_role(Role::Group, "").node();
    h.tree.focus(group);
    h.key(Key::Down, Mods::NONE); // round the end
    h.frame();
    assert!(h.semantics().contains("Radio \"S\" [checked]"), "{}", h.semantics());
}

#[test]
fn a_slider_steps_by_its_keys() {
    let value = signal(50.0f32);
    let mut h = mounted(move || root().child(slider(value, (0.0, 100.0), 10.0)));
    let face = h.get_by_role(Role::Slider, "").node();
    h.tree.focus(face);
    h.key(Key::Right, Mods::NONE);
    h.key(Key::End, Mods::NONE);
    h.key(Key::Left, Mods::NONE);
    assert_eq!(value.get_untracked(), 90.0);
}

#[test]
fn a_select_opens_its_menu_and_a_choice_closes_it() {
    let fruit = signal(0);
    let mut h = mounted(move || root().child(select(&["Apple", "Pear", "Fig"], fruit)));
    h.get_by_role(Role::Button, "Apple").click();
    h.frame();
    assert!(h.semantics().contains("MenuItem \"Apple\" [checked]"), "{}", h.semantics());
    h.get_by_role(Role::MenuItem, "Fig").click();
    h.frame();
    assert_eq!(fruit.get_untracked(), 2);
    assert!(!h.semantics().contains("MenuItem"), "{}", h.semantics());
}

#[test]
fn a_dialog_is_answered_or_given_up() {
    let (open, said) = (signal(true), signal(""));
    let mut h = mounted(move || {
        let yes: Act = Rc::new(move || said.set("yes"));
        root().child(dialog(open, "Save?", "Or lose them.", vec![("Yes", yes)]))
    });
    h.get_by_role(Role::Dialog, "Save?");
    h.get_by_role(Role::Button, "Yes").click();
    h.frame();
    assert_eq!((said.get_untracked(), open.get_untracked()), ("yes", false));
    open.set(true);
    h.frame();
    h.get_by_role(Role::Button, "Close").click();
    h.frame();
    assert!(!open.get_untracked() && !h.semantics().contains("Dialog"));
}

#[test]
fn a_list_selects_by_the_arrows_and_a_click() {
    let picked = signal(None);
    let mut h =
        mounted(move || root().child(list(["a", "b", "c"].map(String::from).to_vec(), picked, 120.0)));
    let frame = h.get_by_role(Role::List, "").node();
    h.tree.focus(frame);
    h.key(Key::Down, Mods::NONE);
    h.key(Key::Down, Mods::NONE);
    assert_eq!(picked.get_untracked(), Some(1));
    h.frame();
    h.get_by_role(Role::ListItem, "c").click();
    assert_eq!(picked.get_untracked(), Some(2));
}

//! Smaller things of the components (specs/13, "Components"): a button that is busy, a
//! checkbox that is neither, an input that is emptied, a second line in a tooltip, a hint of
//! keys, a question with two answers, and menus on buttons.

use std::rc::Rc;
use std::time::Duration;

use quadra_design::{
    MenuItem, button, checkbox, confirm, dark, input, key_hint, menu_button, menubar, tooltip,
};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{NodeId, PointerInput, Role};
use quadra_view::{View, column};

fn mount<V: View + 'static>(build: impl FnOnce() -> V + 'static) -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || quadra_design::root().child(build()));
    h.frame();
    h
}

/// Whether a drawing (an icon) is under a node.
fn draws(h: &mut Harness, id: NodeId) -> bool {
    let here = h.tree.widget_mut::<quadra_tree::Drawing>(id).is_some();
    here || h.tree.children(id).to_vec().into_iter().any(|child| draws(h, child))
}

#[test]
fn a_button_that_is_busy_does_nothing_and_a_checkbox_can_be_neither() {
    let (busy, saved, all, some) = (signal(true), signal(0), signal(false), signal(true));
    let mut h = mount(move || {
        let save = button("Save").loading(move || busy.get()).on_click(move || saved.update(|n| *n += 1));
        column().child(save).child(checkbox(all).label("All").indeterminate(move || some.get()))
    });
    // Busy: a spinner before the label, and a click does nothing.
    let tree = h.semantics();
    assert!(tree.contains("Button \"Save\"") && tree.contains("ProgressBar \"Loading\""), "{tree}");
    h.get_by_role(Role::Button, "Save").click();
    assert_eq!(saved.get_untracked(), 0);
    busy.set(false);
    h.frame();
    h.get_by_role(Role::Button, "Save").click();
    assert!(saved.get_untracked() == 1 && !h.semantics().contains("Loading"));

    // Neither checked nor not: a dash in the box, until it is one or the other.
    let all_box = h.get_by_role(Role::Checkbox, "All").node();
    assert!(draws(&mut h, all_box));
    some.set(false);
    h.frame();
    assert!(!draws(&mut h, all_box));
    all.set(true);
    h.frame();
    assert!(draws(&mut h, all_box));
}

#[test]
fn an_input_is_emptied_by_its_cross_and_a_tooltip_says_more() {
    let query = signal("notes".to_string());
    let mut h = mount(move || {
        let hint = key_hint("ctrl-k", "to search");
        let field =
            tooltip("Find a note", input(query).placeholder("Search").clearable()).meta("In every folder");
        column().child(field).child(hint)
    });
    assert!(h.has_text("Ctrl-K") && h.has_text("to search"));
    h.get_by_role(Role::Button, "Clear").click();
    h.frame();
    assert!(query.get_untracked().is_empty() && !h.semantics().contains("Button \"Clear\""));
    // The second line of the tooltip is under the first.
    h.pointer(PointerInput::Move, 100.0, 15.0);
    h.advance(Duration::from_millis(500));
    h.frame();
    assert!(h.has_text("Find a note") && h.has_text("In every folder"));
}

#[test]
fn a_question_has_two_answers_and_a_bar_has_menus() {
    let (asking, answered, picked) = (signal(false), signal(0), signal(0));
    let mut h = mount(move || {
        let file = menu_button("File", move || vec![MenuItem::new("New").on_select(move || picked.set(1))]);
        let edit = menu_button("Edit", || vec![MenuItem::new("Undo")]);
        let ask = confirm(asking, "Discard changes?", "They are not saved.", "Discard", move || {
            answered.update(|n| *n += 1)
        });
        column().child(menubar([file, edit])).child(ask)
    });
    // The bar is 32 px; a button of it opens its menu under itself.
    let file = h.get_by_role(Role::Button, "File").node();
    assert_eq!(
        h.tree
            .window_rect(h.tree.parent(h.tree.parent(h.tree.parent(file).unwrap()).unwrap()).unwrap())
            .height,
        32.0
    );
    h.get_by_role(Role::Button, "File").click();
    h.frame();
    assert!(h.semantics().contains("MenuItem \"New\""));
    h.get_by_role(Role::MenuItem, "New").click();
    h.frame();
    assert_eq!((picked.get_untracked(), h.tree.overlays().count()), (1, 0));

    asking.set(true);
    h.frame();
    assert!(h.has_text("They are not saved."));
    h.get_by_role(Role::Button, "Cancel").click();
    h.frame();
    assert_eq!((asking.get_untracked(), answered.get_untracked()), (false, 0));
    asking.set(true);
    h.frame();
    h.get_by_role(Role::Button, "Discard").click();
    h.frame();
    assert_eq!((asking.get_untracked(), answered.get_untracked()), (false, 1));
}

#[test]
fn a_value_of_a_description_list_is_copied_by_its_button() {
    let mut h = mount(|| {
        let node = quadra_view::AnyView::new(quadra_view::text("n1"));
        quadra_design::description_list([("Pod IP", quadra_design::copyable("10.42.0.17")), ("Node", node)])
    });
    h.get_by_role(Role::Button, "Copy").click();
    h.frame();
    // The harness's clipboard is in memory: the real one is never touched by a test.
    assert_eq!(h.clipboard.read_text().as_deref(), Some("10.42.0.17"));
    // It says so for a moment, then it is the button that copies again.
    h.get_by_role(Role::Button, "Copied");
    h.advance(Duration::from_millis(1600));
    h.frame();
    h.get_by_role(Role::Button, "Copy");
}

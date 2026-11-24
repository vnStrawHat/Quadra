//! The arrangements (specs/design/layout.md): a form's fields are as wide as their columns,
//! a field outside one keeps its own width, and a reader finds a field by its label. The site's
//! guide quotes the first test (`site/guide/views.md`).

use std::rc::Rc;

use quadra_design::{TimeOfDay, number_input};
use quadra_design::{button, button_row, columns, dark, field, form, input, segmented, time_input};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::column;

// #test
#[test]
fn a_form_lays_its_fields_out_and_they_are_found_by_name() {
    let (name, host, user, saved) =
        (signal(String::new()), signal(String::new()), signal(String::new()), signal(false));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let pair = columns(2).child(field("Host", input(host))).child(field("User", input(user)));
        let save = button("Save").on_click(move || saved.set(true));
        let all = form().child(field("Name", input(name).label("Name"))).child(pair);
        column().width(412.0).child(all.child(button_row().child(save)))
    });
    h.frame();
    // Found as a person finds them: by what they are and what they are called.
    h.get_by_role(Role::TextInput, "Name").type_text("db-01");
    h.get_by_role(Role::Button, "Save").click();
    h.frame();
    assert_eq!((name.get_untracked(), saved.get_untracked()), ("db-01".to_string(), true));
    // No field was given a width: each is as wide as its column, the two of a row 12 apart.
    let wide = |h: &mut Harness, name: &str| {
        let node = h.get_by_role(Role::Group, name).node();
        h.tree.window_rect(node).width
    };
    assert_eq!((wide(&mut h, "Name"), wide(&mut h, "Host"), wide(&mut h, "User")), (412.0, 200.0, 200.0));
}
// #end

#[test]
fn a_control_fills_its_column_in_a_form_and_keeps_its_own_width_outside() {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(|| {
        let (a, b, n, t, s) = (
            signal(String::new()),
            signal(String::new()),
            signal(22.0),
            signal(TimeOfDay::new(2, 0)),
            signal(0),
        );
        let inside = form()
            .child(input(a).label("In"))
            .child(number_input(n).width(96.0).label("Port"))
            .child(time_input(t).label("At"))
            .child(segmented(["Hourly", "Daily"], s).label("Every"))
            .child(quadra_design::select(["SSH", "Serial"], s).test_id("kind"));
        column().width(412.0).child(inside).child(input(b).label("Out")).child(time_input(t).label("Alone"))
    });
    h.frame();
    let wide = |h: &mut Harness, role: Role, name: &str| {
        let node = h.get_by_role(role, name).node();
        // What is typed in is inside its frame, which is the field.
        let node = if role == Role::TextInput { h.tree.parent(node).expect("its frame") } else { node };
        h.tree.window_rect(node).width
    };
    let field = |h: &mut Harness, name: &str| wide(h, Role::TextInput, name);
    // Told a width, a field keeps it in a form too.
    assert_eq!((field(&mut h, "In"), field(&mut h, "Out"), field(&mut h, "Port")), (412.0, 240.0, 96.0));
    // A time and a segmented group take their column's width in a form: 412; 120 alone.
    assert_eq!((wide(&mut h, Role::Group, "At"), wide(&mut h, Role::Group, "Alone")), (412.0, 120.0));
    assert_eq!(wide(&mut h, Role::Group, "Every"), 412.0);
    // A select too, with what holds it and its menu.
    let kind = h.get_by_test_id("kind").node();
    assert_eq!(h.tree.window_rect(kind).width, 412.0);
}

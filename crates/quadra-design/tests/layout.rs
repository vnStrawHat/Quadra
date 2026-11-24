//! The arrangements (specs/design/layout.md): a form's fields are as wide as their columns,
//! a field outside one keeps its own width, and a reader finds a field by its label. The site's
//! guide quotes the first test (`site/guide/views.md`).

use std::rc::Rc;

use quadra_design::{TimeOfDay, date_picker, number_input, select};
use quadra_design::{
    button, button_row, columns, dark, field, form, hstack, input, segmented, spacer, time_input,
};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::{column, text};

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

#[test]
fn what_fills_its_column_gives_way_in_a_row_and_a_bound_text_in_a_form_follows() {
    let (name, kind, day) =
        (signal(String::new()), signal(0usize), signal(quadra_design::Date::new(2026, 11, 24)));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let said = text(move || format!("Hello, {}", name.get()));
        let with_button = hstack().child(input(name).label("Name")).child(button("Go"));
        let beside = hstack()
            .child(text("Every"))
            .child(spacer())
            .child(segmented(["Hour", "Day"], kind).label("Seg"));
        let picks = hstack()
            .child(select(["One", "Two"], kind).test_id("pick"))
            .child(date_picker(day).test_id("day"));
        column().width(412.0).child(form().child(said).child(with_button).child(beside).child(picks))
    });
    h.frame();
    let ends = |h: &mut Harness, node| {
        let rect = h.tree.window_rect(node);
        rect.x + rect.width
    };
    // In a row a field takes the room its neighbours leave; nothing goes past the column.
    let go = h.get_by_role(Role::Button, "Go").node();
    let seg = h.get_by_role(Role::Group, "Seg").node();
    let (pick, day) = (h.get_by_test_id("pick").node(), h.get_by_test_id("day").node());
    assert_eq!((ends(&mut h, go), ends(&mut h, seg)), (412.0, 412.0));
    assert!(ends(&mut h, pick) < ends(&mut h, day) && ends(&mut h, day) <= 412.0, "{}", ends(&mut h, day));
    assert!(h.tree.window_rect(go).x > 300.0, "the field took the room the button left");
    // A text bound to a signal follows it inside a form as anywhere.
    h.get_by_role(Role::TextInput, "Name").type_text("Lan");
    h.frame();
    assert!(h.has_text("Hello, Lan"));
}

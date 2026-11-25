//! The kit's text input (specs/design): a field in a frame whose border says where the
//! focus is and whether the value is accepted; a placeholder stands in it while it is empty.

use std::rc::Rc;

use quadra_design::{color_input, combobox, dark, icons, input, number_input, textarea, token};
use quadra_platform::{Key, Mods};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{NodeId, Role};
use quadra_view::column;

#[test]
fn the_frame_shows_the_focus_and_what_is_not_accepted() {
    let (name, secret, wrong, off) =
        (signal(String::new()), signal(String::new()), signal(false), signal(true));
    let mut h = Harness::new();
    let theme = dark();
    h.tree.set_theme(Rc::new(theme.theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let search = input(name).placeholder("Search").icon(icons::SEARCH).invalid(wrong).test_id("search");
        let password = input(secret).placeholder("Password").password().disabled(off).test_id("secret");
        quadra_design::root().child(column().gap(8.0).child(search).child(password))
    });
    h.frame();
    // A reader hears the placeholder as the name of a field that has no other.
    let tree = h.semantics();
    assert!(tree.contains("TextInput \"Search\" = \"\" #search"), "{tree}");
    let frame_of = |h: &mut Harness, id: &str| -> NodeId {
        let field = h.get_by_test_id(id).node();
        h.tree.parent(field).unwrap()
    };
    let (search, secret_frame) = (frame_of(&mut h, "search"), frame_of(&mut h, "secret"));
    // 240 wide and the md control height, as a button and a select are.
    assert_eq!(h.tree.rect(search).size(), quadra_prim::Size::new(240.0, 28.0));
    let look = |h: &Harness, frame: NodeId| {
        let paint = h.tree.paint_style(frame);
        (paint.background, paint.border_color)
    };
    assert_eq!(look(&h, search), (theme.color(token::INPUT_BG), theme.color(token::LINE_STRONG)));
    assert_eq!(look(&h, secret_frame), (theme.color(token::PANEL), theme.color(token::LINE_STRONG))); // disabled

    // The border follows the focus, and Tab passes the disabled field by.
    h.get_by_role(Role::TextInput, "Search").type_text("abc");
    h.frame();
    assert_eq!(
        (name.get_untracked().as_str(), look(&h, search).1),
        ("abc", theme.color(token::BORDER_FOCUSED))
    );
    h.key(Key::Tab, Mods::NONE);
    let field = h.get_by_test_id("search").node();
    assert_eq!(h.tree.focused(), Some(field));
    let at = h.tree.window_rect(secret_frame).center();
    h.click(at.x, at.y); // the disabled field: it does not take the focus, and nothing is typed
    h.type_text("x");
    h.frame();
    assert_eq!((secret.get_untracked().as_str(), h.tree.focused()), ("", None));

    // Not accepted: the border is the colour of errors, with the focus or without.
    wrong.set(true);
    h.frame();
    assert_eq!(look(&h, search).1, theme.color(token::ERROR));
    h.get_by_test_id("search").focus();
    h.frame();
    assert_eq!(look(&h, search).1, theme.color(token::ERROR));
    off.set(false);
    h.frame();
    h.get_by_test_id("secret").type_text("pw");
    h.frame();
    assert!(secret.get_untracked() == "pw" && h.has_text("\u{2022}\u{2022}"));
}

#[test]
fn underlined_the_frame_is_a_line_under_text_that_stands_where_a_box_s_does() {
    let (name, boxed, wrong, off) =
        (signal(String::new()), signal(String::new()), signal(false), signal(false));
    let mut h = Harness::new();
    let theme = dark();
    h.tree.set_theme(Rc::new(theme.theme()));
    h.mount(move || {
        let line = input(name).underlined().placeholder("Name").invalid(wrong).disabled(off).test_id("line");
        let boxed = input(boxed).placeholder("Boxed").test_id("boxed");
        quadra_design::root().child(quadra_view::row().child(line).child(boxed))
    });
    h.frame();
    let (field, other) = (h.get_by_test_id("line").node(), h.get_by_test_id("boxed").node());
    let frame = h.tree.parent(field).unwrap();
    // The field's size; a line under it alone, and nothing inside left and right.
    assert_eq!(h.tree.rect(frame).size(), quadra_prim::Size::new(240.0, 28.0));
    let border = h.tree.layout_style(frame).border;
    assert_eq!((border.top, border.left, border.right, border.bottom), (0.0, 0.0, 0.0, 1.0));
    assert_eq!((h.tree.rect(field).x, h.tree.rect(field).width), (0.0, 240.0));
    // Beside a boxed field its text is on the same line.
    assert_eq!(h.tree.window_rect(field).y, h.tree.window_rect(other).y);
    let look = |h: &Harness| (h.tree.paint_style(frame).background, h.tree.paint_style(frame).border_color);
    let none = quadra_prim::Color::TRANSPARENT;
    assert_eq!(look(&h), (none, theme.color(token::LINE_STRONG)));
    // The line says the focus and what is not accepted; disabled, it is a step back, with no fill.
    h.get_by_test_id("line").focus();
    h.frame();
    assert_eq!(look(&h), (none, theme.color(token::BORDER_FOCUSED)));
    wrong.set(true);
    h.frame();
    assert_eq!(look(&h), (none, theme.color(token::ERROR)));
    wrong.set(false);
    off.set(true);
    h.frame();
    assert_eq!(look(&h), (none, theme.color(token::LINE)));
}

#[test]
fn a_colour_input_shows_its_colour_or_the_fallback_and_refuses_what_is_no_colour() {
    let accent = signal(String::new());
    let mut h = Harness::new();
    let theme = dark();
    h.tree.set_theme(Rc::new(theme.theme()));
    h.mount(move || {
        quadra_design::root().child(color_input(accent).placeholder("Accent").fallback(token::ACCENT))
    });
    h.frame();
    let field = h.get_by_role(Role::TextInput, "Accent").node();
    let frame = h.tree.parent(field).unwrap();
    let swatch = h.tree.children(frame)[0];
    let look = |h: &Harness| (h.tree.paint_style(swatch).background, h.tree.paint_style(frame).border_color);
    assert_eq!(
        look(&h),
        (theme.color(token::ACCENT), theme.color(token::LINE_STRONG)),
        "empty: the fallback"
    );
    h.get_by_role(Role::TextInput, "Accent").set_value("#ff0000");
    h.frame();
    assert_eq!(accent.get_untracked(), "#ff0000");
    assert_eq!(look(&h).0, quadra_prim::Color::rgb(0xFF0000));
    h.get_by_role(Role::TextInput, "Accent").set_value("red");
    h.frame();
    assert_eq!(look(&h), (theme.color(token::ACCENT), theme.color(token::ERROR)), "no colour: not accepted");
}

#[test]
fn a_label_names_a_field_to_a_reader_and_follows_its_words() {
    let (query, notes, accent, count) =
        (signal(String::new()), signal(String::new()), signal(String::new()), signal(2.0));
    let (city, name) = (signal(String::new()), signal("Find notes".to_string()));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let fields = column()
            .child(input(query).placeholder("Search").label(name))
            .child(textarea(notes).placeholder("Write here").label("Notes"))
            .child(color_input(accent).placeholder("#rrggbb").label("Accent colour"))
            .child(number_input(count).label("Copies"))
            .child(number_input(count).stepper().label(move || format!("Copies of {}", name.get())))
            .child(combobox(["Hanoi", "Hue"], city).placeholder("City").label("Home city"))
            .child(input(notes).readonly().label("Shown notes"));
        quadra_design::root().child(fields)
    });
    h.frame();
    let named = |h: &Harness, name: &str| {
        h.query(|n| n.semantics.role == Role::TextInput && n.semantics.name == name).len()
    };
    for name in ["Find notes", "Notes", "Accent colour", "Copies", "Copies of Find notes", "Home city"] {
        assert_eq!(named(&h, name), 1, "{name}:\n{}", h.semantics());
    }
    // Read only, the frame is a group known by the label.
    assert_eq!(h.query(|n| n.semantics.role == Role::Group && n.semantics.name == "Shown notes").len(), 1);
    // The placeholder is no name while a label is given; words that change are followed.
    assert_eq!(named(&h, "Search") + named(&h, "Write here") + named(&h, "City"), 0);
    name.set("Find files".to_string());
    h.frame();
    assert_eq!(
        (named(&h, "Find files"), named(&h, "Copies of Find files"), named(&h, "Find notes")),
        (1, 1, 0)
    );
    h.get_by_role(Role::TextInput, "Copies").set_value("4");
    h.frame();
    assert_eq!(count.get_untracked(), 4.0);
}

#[test]
fn a_password_shows_while_its_eye_is_pressed_and_a_key_follows_a_button_s_label() {
    let secret = signal(String::new());
    let mut h = Harness::new();
    h.mount(move || {
        let field = input(secret).password().reveal().prefix("pw").mono().height(28.0).test_id("secret");
        let unlock = quadra_design::button("Unlock").shortcut("Enter");
        quadra_design::root().child(column().child(field).child(unlock))
    });
    h.frame();
    h.get_by_test_id("secret").type_text("ab");
    h.frame();
    assert!(h.has_text("\u{2022}\u{2022}") && !h.has_text("ab"));
    h.get_by_role(Role::Button, "Show password").click();
    h.frame();
    assert!(h.has_text("ab"), "{}", h.semantics());
    h.get_by_role(Role::Button, "Show password").click();
    h.frame();
    assert!(h.has_text("\u{2022}\u{2022}"));
    // Shown, then emptied (a password sent): what is typed next is hidden again.
    h.get_by_role(Role::Button, "Show password").click();
    secret.set(String::new());
    h.frame();
    h.get_by_test_id("secret").type_text("cd");
    h.frame();
    assert!(h.has_text("\u{2022}\u{2022}") && !h.has_text("cd"));
    // The button is called by its label alone; the key shows after it.
    assert!(h.has_text("Enter"));
    h.get_by_role(Role::Button, "Unlock");
}

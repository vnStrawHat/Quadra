//! The kit's text input (specs/behaviour/15): a field in a frame whose border says where the
//! focus is and whether the value is accepted; a placeholder stands in it while it is empty.

use std::rc::Rc;

use quadra_kit::{dark, icons, input, token};
use quadra_platform::{Key, Mods};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{NodeId, Role};
use quadra_ui::column;

#[test]
fn the_frame_shows_the_focus_and_what_is_not_accepted() {
    let (name, secret, wrong, off) =
        (signal(String::new()), signal(String::new()), signal(false), signal(true));
    let mut h = Harness::new();
    let theme = dark();
    h.tree.set_theme(Rc::new(theme.theme()));
    h.tree.set_keymap(Rc::new(quadra_kit::keymap()));
    h.mount(move || {
        let search = input(name).placeholder("Search").icon(icons::SEARCH).invalid(wrong).test_id("search");
        let password = input(secret).placeholder("Password").password().disabled(off).test_id("secret");
        quadra_kit::root().child(column().gap(8.0).child(search).child(password))
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
    // 240 wide; a line of 23 px, 6 above and below, and the border.
    assert_eq!(h.tree.rect(search).size(), quadra_prim::Size::new(240.0, 37.0));
    let look = |h: &Harness, frame: NodeId| {
        let paint = h.tree.paint_style(frame);
        (paint.background, paint.border_color)
    };
    assert_eq!(look(&h, search), (theme.color(token::INPUT_BG), theme.color(token::LINE)));
    assert_eq!(look(&h, secret_frame), (theme.color(token::PANEL), theme.color(token::LINE))); // disabled

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

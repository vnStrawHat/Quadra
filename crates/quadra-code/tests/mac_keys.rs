//! The editor's keys on a Mac (specs/09, "On a Mac"): its keymap made a Mac's.

use std::rc::Rc;

use quadra_code::{EditorWidget, bind_keys, editor};
use quadra_platform::{Key, Mods};
use quadra_testing::Harness;
use quadra_tree::{Keymap, Role};
use quadra_view::column;

#[test]
fn on_a_mac_option_goes_by_words_and_command_to_the_ends() {
    let mut h = Harness::new();
    let mut keys = Keymap::default();
    bind_keys(&mut keys);
    h.tree.set_keymap(Rc::new(keys.as_a_mac_has_them()));
    let (text, path) = ("one two\nthree".to_string(), "a.txt".to_string());
    h.mount(move || column().size(400.0, 140.0).child(editor(text).path(path).font(10.0, 14.0)));
    h.frame();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    h.frame();
    let mut head = |key, mods| {
        h.key(key, mods);
        h.frame_if_needed();
        h.tree.widget_mut::<EditorWidget>(id).unwrap().doc.primary().head
    };
    assert!(matches!(head(Key::Right, Mods::ALT), 3 | 4), "a word, not to the end of the line");
    assert_eq!(head(Key::Right, Mods::CTRL), 7, "the end of the line");
    assert_eq!(head(Key::Down, Mods::CTRL), 13, "the end of the text");
    assert_eq!(head(Key::Left, Mods::CTRL), 8, "the start of the line");
    assert_eq!(head(Key::Up, Mods::CTRL), 0, "the start of the text");
}

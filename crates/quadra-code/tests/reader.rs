//! What the editor tells a reader (specs/12, specs/13 "Code"): a small text whole, a large
//! one as the lines around the primary cursor, with the selection in that window.

use quadra_code::editor;
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_ui::column;

/// The value and the selection the editor tells, after `f` moved its cursor.
fn told(text: String, f: impl FnOnce(&mut quadra_code::EditorWidget)) -> (String, (usize, usize)) {
    let mut h = Harness::new();
    h.mount(move || column().size(400.0, 140.0).child(editor(text).font(10.0, 14.0)));
    h.frame();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    f(h.tree.widget_mut::<quadra_code::EditorWidget>(id).unwrap());
    let says = h.tree.semantic_tree();
    let node = says.nodes.iter().find(|n| n.semantics.role == Role::TextInput).unwrap();
    (node.semantics.value.clone().unwrap(), node.semantics.selection.unwrap())
}

#[test]
fn a_large_text_is_told_as_the_lines_around_the_cursor() {
    let (value, selection) = told("ab\ncd".into(), |w| w.doc.set_cursor(3, 4, false));
    assert_eq!((value.as_str(), selection), ("ab\ncd", (3, 4)));
    // 10,000 lines of 37 bytes: 370 KB. The cursor on line 5,000, after "line 05000 ".
    let text: String = (0..10_000).map(|i| format!("line {i:05} and what follows it here.\n")).collect();
    let long = text.find('\n').unwrap() + 1;
    let at = 5_000 * long + 11;
    let (value, selection) = told(text, |w| w.doc.set_cursor(at, at, false));
    let lines: Vec<&str> = value.split('\n').collect();
    assert_eq!(lines.len(), 401);
    assert!(lines[0].starts_with("line 04800") && lines[400].starts_with("line 05200"));
    assert_eq!(selection, (200 * long + 11, 200 * long + 11));
}

#[test]
fn a_reader_hears_of_the_caret_that_moves() {
    use std::rc::Rc;

    use quadra_code::bind_keys;
    use quadra_platform::{Key, Mods};
    use quadra_tree::Keymap;

    let mut h = Harness::new();
    let mut keys = Keymap::default();
    bind_keys(&mut keys);
    h.tree.set_keymap(Rc::new(keys));
    h.mount(move || column().size(400.0, 140.0).child(editor("ab\ncd").font(10.0, 14.0)));
    h.frame();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    h.tree.listen(true);
    h.frame();
    assert!(h.tree.take_semantic_update().is_some()); // the first: all of it
    h.key(Key::Down, Mods::NONE);
    h.frame_if_needed();
    let update = h.tree.take_semantic_update().expect("the caret moved");
    let input = update.nodes.iter().find(|n| n.semantics.role == Role::TextInput).unwrap();
    assert_eq!(input.semantics.selection, Some((3, 3)));
}

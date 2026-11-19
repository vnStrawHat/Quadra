//! The toolbar (specs/21, "Toolbar"): a button formats the selection as one change that one
//! undo takes back, its key does the same, the focus is back in the editor after, and the
//! toolbar shows only while the editor does.

use std::cell::RefCell;
use std::rc::Rc;

use markdown::{MdApp, Model, keymap};
use quadra::{Key, Mods, Role};
use quadra_testing::Harness;

fn app(name: &str, text: &str) -> (Harness, Model) {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/markdown-tests");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, text).unwrap();
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 700.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let built = Rc::new(RefCell::new(None));
    let tell = built.clone();
    h.mount(move || {
        let md = MdApp::new(Some(path));
        *tell.borrow_mut() = Some(md.model.clone());
        md.view()
    });
    let m: Model = built.borrow_mut().take().unwrap();
    h.settle();
    (h, m)
}

/// The editor's text, as it holds it now.
fn text(h: &mut Harness, m: &Model) -> String {
    let read = Rc::new(RefCell::new(String::new()));
    let into = read.clone();
    m.editor.read(move |w| *into.borrow_mut() = w.doc.text());
    h.settle();
    read.take()
}

fn select(h: &mut Harness, m: &Model, range: std::ops::Range<usize>) {
    m.editor.edit(move |w| w.doc.set_cursor(range.start, range.end, false));
    h.settle();
}

#[test]
fn a_button_formats_the_selection_as_one_change_and_gives_the_focus_back() {
    let (mut h, m) = app("toolbar-bold.md", "say hello now\n");
    assert!(
        h.query(|n| n.semantics.name == "Bold" && n.semantics.role == Role::Button).is_empty(),
        "Preview: none"
    );
    h.key(Key::Digit1, Mods::CTRL);
    h.settle();
    select(&mut h, &m, 4..9);
    h.get_by_role(Role::Button, "Bold").click();
    h.settle();
    assert_eq!(text(&mut h, &m), "say **hello** now\n");
    let focused = h.query(|n| n.focused && n.semantics.role == Role::TextInput);
    assert_eq!(focused.len(), 1, "the focus is back in the editor");
    // One undo takes it back.
    h.key(Key::Z, Mods::CTRL);
    h.settle();
    assert_eq!(text(&mut h, &m), "say hello now\n");
}

#[test]
fn the_keys_of_the_buttons_and_the_lines_touched() {
    let (mut h, m) = app("toolbar-keys.md", "one\ntwo\n");
    h.key(Key::Digit1, Mods::CTRL);
    h.settle();
    select(&mut h, &m, 0..5);
    h.get_by_role(Role::TextInput, "").focus();
    h.key(Key::Digit8, Mods::CTRL | Mods::SHIFT);
    h.settle();
    assert_eq!(text(&mut h, &m), "- one\n- two\n");
    h.key(Key::Digit2, Mods::CTRL | Mods::ALT);
    h.settle();
    assert_eq!(text(&mut h, &m), "## one\n## two\n", "a heading takes a list's place");
    h.get_by_role(Role::Button, "Table").click();
    h.settle();
    assert!(text(&mut h, &m).contains("| Column 1 | Column 2 | Column 3 |"));
}

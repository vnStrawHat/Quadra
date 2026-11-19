//! Find (specs/21, "Find"): Preview's bar, its count and the way to the next match, the app's
//! keys from its field; the editor's bar while the editor shows, from either pane.

use std::path::PathBuf;
use std::rc::Rc;

use markdown::{MdApp, Tabs, keymap};
use quadra::{Key, Mods, Role};
use quadra_shell::FakeShell;
use quadra_testing::Harness;

const FRUIT: &str = "# Fruit\n\nAn apple a day.\n\n```\nlet apple = 1;\n```\n\n| Name |\n|---|\n| Apple |\n";

fn file(name: &str, text: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/markdown-tests/search");
    std::fs::create_dir_all(&dir).expect("the test folder");
    let path = dir.join(name);
    std::fs::write(&path, text).expect("the test file");
    path
}

fn app(files: Vec<PathBuf>) -> (Harness, Tabs) {
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 800.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let built = Rc::new(std::cell::RefCell::new(None));
    let tell = built.clone();
    h.mount(move || {
        FakeShell::new().provide();
        let md = MdApp::with_files(files);
        *tell.borrow_mut() = Some(md.tabs.clone());
        md.view()
    });
    h.settle();
    let tabs = built.borrow_mut().take().unwrap();
    (h, tabs)
}

fn key(h: &mut Harness, key: Key, mods: Mods) {
    h.key(key, mods);
    h.settle();
}

fn bar_open(h: &Harness) -> bool {
    !h.query(|n| n.semantics.name == "Find in the document").is_empty()
}

#[test]
fn preview_finds_counts_and_steps_through_its_matches() {
    let (mut h, _) = app(vec![file("fruit.md", FRUIT)]);
    key(&mut h, Key::F, Mods::CTRL);
    assert!(bar_open(&h));
    // The focus is in the query: what is typed goes there.
    h.type_text("apple");
    h.settle();
    assert!(h.has_text("1 of 3"), "the paragraph's, the code's and the cell's");
    key(&mut h, Key::Enter, Mods::NONE);
    assert!(h.has_text("2 of 3"));
    key(&mut h, Key::Enter, Mods::SHIFT);
    assert!(h.has_text("1 of 3"));
    key(&mut h, Key::F3, Mods::NONE);
    key(&mut h, Key::F3, Mods::NONE);
    key(&mut h, Key::F3, Mods::NONE);
    assert!(h.has_text("1 of 3"), "round");
    h.get_by_role(Role::Button, "Match case").click();
    h.settle();
    assert!(h.has_text("1 of 2"));
    h.get_by_role(Role::TextInput, "Find").type_text("zz");
    h.settle();
    assert!(h.has_text("No results"));
    key(&mut h, Key::Escape, Mods::NONE);
    assert!(!bar_open(&h));
}

#[test]
fn the_apps_keys_work_from_the_query() {
    let (mut h, tabs) = app(vec![file("keys-1.md", FRUIT), file("keys-2.md", "# Two\n")]);
    key(&mut h, Key::F, Mods::CTRL);
    assert!(bar_open(&h));
    key(&mut h, Key::Tab, Mods::CTRL);
    assert_eq!(tabs.current_untracked().name(), "keys-2.md", "Ctrl+Tab from the field");
    assert!(!bar_open(&h), "each tab has its own find");
    key(&mut h, Key::Tab, Mods::CTRL);
    assert!(bar_open(&h), "the first's is open still");
    key(&mut h, Key::Digit1, Mods::CTRL);
    assert!(!bar_open(&h), "Preview's find is Preview's");
    key(&mut h, Key::W, Mods::CTRL);
    assert_eq!(tabs.current_untracked().name(), "keys-2.md", "Ctrl+W");
}

#[test]
fn while_the_editor_shows_ctrl_f_is_its_find_from_either_pane() {
    let (mut h, _) = app(vec![file("editor.md", FRUIT)]);
    key(&mut h, Key::Digit2, Mods::CTRL);
    // The focus in the preview's pane: Ctrl+F is still the editor's find.
    let rendered = h.get_by_role(Role::Group, "Rendered").node();
    let at = h.tree.window_rect(rendered);
    h.click(at.x + 40.0, at.y + at.height - 20.0);
    h.settle();
    key(&mut h, Key::F, Mods::CTRL);
    assert!(!bar_open(&h), "not Preview's");
    assert!(!h.query(|n| n.semantics.name == "Previous match").is_empty(), "the editor's bar");
    h.type_text("apple");
    h.settle();
    assert!(h.has_text("1 of 3"), "the editor finds in the source");
}

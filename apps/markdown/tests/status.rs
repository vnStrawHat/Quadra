//! The status bar (specs/21, "Window"): the counts of the text the preview shows, the
//! editor's cursor while it shows, the lines and the line ends.

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use markdown::{MdApp, keymap};
use quadra::{Key, Mods, Role};
use quadra_testing::Harness;

fn file(name: &str, text: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/markdown-tests/status");
    std::fs::create_dir_all(&dir).expect("the test folder");
    let path = dir.join(name);
    std::fs::write(&path, text).expect("the test file");
    path
}

#[test]
fn the_status_bar_counts_and_follows_the_cursor() {
    let path = file("status.md", "# Notes\r\n\r\nOne two three.\r\n");
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 800.0);
    h.tree.set_keymap(Rc::new(keymap()));
    h.mount(move || MdApp::new(Some(path)).view());
    h.settle();
    assert!(h.has_text("4 words · 21 characters · 1 min read"));
    assert!(h.has_text("3 lines") && h.has_text("CRLF"));
    assert!(!h.has_text("Ln 1, Col 1"), "no cursor in Preview");
    h.key(Key::Digit1, Mods::CTRL);
    h.settle();
    assert!(h.has_text("Ln 1, Col 1"));
    h.get_by_role(Role::TextInput, "").focus();
    for (key, mods) in [(Key::Down, Mods::NONE), (Key::Down, Mods::NONE), (Key::End, Mods::SHIFT)] {
        h.key(key, mods);
    }
    h.settle();
    assert!(h.has_text("Ln 3, Col 15 (14 selected)"));
    h.get_by_role(Role::TextInput, "").type_text("Four");
    h.settle();
    h.advance(Duration::from_millis(200));
    h.settle();
    assert!(h.has_text("2 words · 11 characters · 1 min read"), "counted once the editor stands still");
}

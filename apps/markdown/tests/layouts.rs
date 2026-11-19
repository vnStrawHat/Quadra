//! The window (specs/21, "Window" and "Layouts"): a file given opens in Preview, its name is
//! the title, and the three layouts by the bar and by their keys.

use std::path::PathBuf;
use std::rc::Rc;

use markdown::{MdApp, keymap};
use quadra::{Key, Mods, Role};
use quadra_testing::Harness;
use quadra_tree::WindowRequest;

/// A file of the test's own under `.tmp/`.
fn file(name: &str, text: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/markdown-tests");
    std::fs::create_dir_all(&dir).expect("the test folder");
    let path = dir.join(name);
    std::fs::write(&path, text).expect("the test file");
    path
}

fn app(path: Option<PathBuf>) -> Harness {
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 800.0);
    h.tree.set_keymap(Rc::new(keymap()));
    h.mount(move || MdApp::new(path).view());
    h.settle();
    h
}

fn panes(h: &Harness) -> Vec<String> {
    let named = |n: &quadra_tree::SemanticNode| n.semantics.role == Role::Group;
    let mut names: Vec<String> = h.query(named).into_iter().map(|id| h.tree.meaning(id).name).collect();
    names.retain(|name| name == "Source" || name == "Rendered");
    names
}

#[test]
fn a_file_given_opens_in_preview_and_names_the_window() {
    let path = file("opens.md", "# Release notes\n\nThe **first** one.\n");
    let mut h = app(Some(path));
    assert!(h.has_text("Release notes"), "the heading is rendered");
    assert_eq!(panes(&h), ["Rendered"]);
    assert!(h.has_text("opens.md"));
    let titles: Vec<WindowRequest> = h.tree.take_window_requests();
    assert!(titles.contains(&WindowRequest::Title("opens.md - Markdown".into())), "{titles:?}");
}

#[test]
fn a_file_that_starts_with_a_byte_order_mark_shows_its_first_heading() {
    let h = app(Some(file("bom.md", "\u{feff}# Marked\n\nText.\n")));
    assert_eq!(h.query(|n| n.semantics.role == Role::Heading && n.semantics.name == "Marked").len(), 1);
}

#[test]
fn the_layouts_by_the_bar_and_by_their_keys() {
    let mut h = app(Some(file("layouts.md", "# One\n\ntext\n")));
    h.get_by_role(Role::Radio, "Raw").click();
    h.settle();
    assert_eq!(panes(&h), ["Source"]);
    h.key(Key::Digit2, Mods::CTRL);
    h.settle();
    assert_eq!(panes(&h), ["Source", "Rendered"]);
    h.key(Key::Digit3, Mods::CTRL);
    h.settle();
    assert_eq!(panes(&h), ["Rendered"]);
}

#[test]
fn without_a_file_it_is_untitled_and_one_that_cannot_be_read_says_so() {
    let h = app(None);
    assert!(h.has_text("Untitled"));
    let missing = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/markdown-tests/none.md");
    let mut h2 = app(Some(missing));
    h2.settle();
    assert!(!h2.query(|n| n.semantics.name.starts_with("Could not open")).is_empty());
    assert!(h2.has_text("Untitled"), "the document is as it was");
}

#[test]
fn a_layout_that_hides_the_editor_takes_the_keys_away_from_it() {
    let path = file("hidden-keys.md", "# Kept\n");
    let mut h = app(Some(path));
    h.key(Key::Digit1, Mods::CTRL);
    h.settle();
    h.get_by_role(Role::TextInput, "").type_text("a");
    h.settle();
    // Preview: the editor is kept, hidden, and what is typed reaches nothing.
    h.key(Key::Digit3, Mods::CTRL);
    h.settle();
    assert_eq!(panes(&h), ["Rendered"], "the editor hidden");
    assert_eq!(h.tree.focused(), None);
    h.type_text("zz");
    h.settle();
    h.key(Key::Digit1, Mods::CTRL);
    h.settle();
    let value = h.get_by_role(Role::TextInput, "").node();
    let said = h.tree.meaning(value).value.unwrap_or_default();
    assert!(said.starts_with("a# Kept") && !said.contains('z'), "{said}");
}

//! Files (specs/21, "Files"): an edit shown in the preview, saving through a rename with the
//! line ends kept, save as for an untitled document, and the question before unsaved work is
//! lost, by opening another file and by closing the window. The OS's dialogs are a fake.

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use markdown::{MdApp, Model, keymap};
use quadra::{Key, Mods, Role};
use quadra_shell::{Answer, FakeShell};
use quadra_testing::Harness;
use quadra_tree::WindowRequest;

fn dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/markdown-tests/files");
    std::fs::create_dir_all(&dir).expect("the test folder");
    dir
}

fn file(name: &str, text: &str) -> PathBuf {
    let path = dir().join(name);
    std::fs::write(&path, text).expect("the test file");
    path
}

struct App {
    h: Harness,
    shell: Rc<FakeShell>,
    model: Model,
}

fn app(path: Option<PathBuf>) -> App {
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 800.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let (shell, built) = (FakeShell::new(), Rc::new(std::cell::RefCell::new(None)));
    let (given, tell) = (shell.clone(), built.clone());
    h.mount(move || {
        given.provide();
        let md = MdApp::new(path);
        *tell.borrow_mut() = Some(md.model.clone());
        md.view()
    });
    h.settle();
    let model = built.borrow_mut().take().unwrap();
    App { h, shell, model }
}

impl App {
    fn key(&mut self, key: Key, mods: Mods) {
        self.h.key(key, mods);
        self.h.settle();
    }

    /// Types at the start of the editor's text, then waits for the preview.
    fn type_in(&mut self, words: &str) {
        self.h.get_by_role(Role::TextInput, "").type_text(words);
        self.h.settle();
        self.h.advance(Duration::from_millis(200));
        self.h.settle();
    }

    fn unsaved(&self) -> bool {
        !self.h.query(|n| n.semantics.name == "Unsaved changes").is_empty()
    }
}

#[test]
fn an_edit_shows_in_the_preview_and_a_save_keeps_the_line_ends() {
    let path = file("crlf.md", "# Notes\r\n\r\nfirst\r\n");
    let mut a = app(Some(path.clone()));
    a.key(Key::Digit2, Mods::CTRL);
    assert!(!a.unsaved());
    a.type_in("Added ");
    assert!(a.h.has_text("Added # Notes"), "the preview follows the editor");
    assert!(a.unsaved());
    a.key(Key::S, Mods::CTRL);
    a.h.settle();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "Added # Notes\r\n\r\nfirst\r\n");
    assert!(!a.unsaved(), "saved");
    assert!(!dir().join(".crlf.md.saving").exists(), "the file beside it was renamed over it");
    // In Preview, the editor's kept document is what is saved.
    a.type_in("More ");
    a.key(Key::Digit3, Mods::CTRL);
    a.key(Key::S, Mods::CTRL);
    assert!(std::fs::read_to_string(&path).unwrap().starts_with("Added More # Notes\r\n"));
}

#[test]
fn an_untitled_document_is_saved_where_the_user_says() {
    let mut a = app(None);
    a.key(Key::Digit1, Mods::CTRL);
    a.type_in("# Fresh");
    let target = dir().join("fresh.md");
    a.shell.choose(&[&target.display().to_string()]);
    a.key(Key::S, Mods::CTRL);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "# Fresh");
    assert!(a.h.has_text("fresh.md") && !a.unsaved());
    assert_eq!(a.shell.asked(), ["save_file: Save as"]);
}

#[test]
fn opening_another_file_asks_first_about_unsaved_work() {
    let first = file("first.md", "# First\n");
    let second = file("second.md", "# Second\n");
    let mut a = app(Some(first.clone()));
    a.key(Key::Digit1, Mods::CTRL);
    a.type_in("x");
    // Cancel: nothing opens.
    a.shell.answer(Answer::Cancel);
    a.key(Key::O, Mods::CTRL);
    assert!(a.h.has_text("first.md"));
    // Don't save: the other opens, in Preview, and the first is as it was.
    a.shell.choose(&[&second.display().to_string()]);
    a.shell.answer(Answer::No);
    a.key(Key::O, Mods::CTRL);
    assert!(a.h.has_text("second.md") && a.h.has_text("Second"));
    assert_eq!(std::fs::read_to_string(&first).unwrap(), "# First\n");
    assert_eq!(a.model.layout.get_untracked(), markdown::PREVIEW);
    let asked = a.shell.asked();
    assert_eq!(asked, ["message: Markdown", "message: Markdown", "open_files: Open"]);
}

#[test]
fn closing_with_unsaved_work_saves_first_when_told_to() {
    let path = file("closing.md", "# Closing\n");
    let mut a = app(Some(path.clone()));
    a.key(Key::Digit1, Mods::CTRL);
    a.type_in("y");
    a.h.tree.take_window_requests();
    a.shell.answer(Answer::Yes);
    assert!(!a.h.tree.may_close(), "it asks first");
    a.h.settle();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "y# Closing\n");
    assert!(a.h.tree.take_window_requests().contains(&WindowRequest::CloseNow), "then closes");
}

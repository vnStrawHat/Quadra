//! Tabs (specs/21, "Tabs"): a file opens in a tab and asks nothing; one already open shows
//! its tab; closing a tab or the window asks about unsaved work; the keys; and the files a
//! later start of the app hands over. The OS's dialogs are a fake.

use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use markdown::{MdApp, Tabs, keymap};
use quadra::{Key, Mods, Role};
use quadra_shell::{Answer, FakeShell, Launch};
use quadra_testing::Harness;
use quadra_tree::WindowRequest;

fn dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/markdown-tests/tabs");
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
    tabs: Tabs,
    later: quadra::Sender<Launch>,
}

fn app(path: Option<PathBuf>) -> App {
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 800.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let (shell, built) = (FakeShell::new(), Rc::new(std::cell::RefCell::new(None)));
    let (given, tell) = (shell.clone(), built.clone());
    let launches = quadra::inbox::<Launch>();
    let later = launches.sender();
    h.mount(move || {
        given.provide();
        let md = MdApp::new(path).listen(launches);
        *tell.borrow_mut() = Some(md.tabs.clone());
        md.view()
    });
    h.settle();
    let tabs = built.borrow_mut().take().unwrap();
    App { h, shell, tabs, later }
}

impl App {
    fn key(&mut self, key: Key, mods: Mods) {
        self.h.key(key, mods);
        self.h.settle();
    }

    /// Types at the start of the active editor's text (Raw), then waits for the preview.
    fn edit(&mut self, words: &str) {
        self.key(Key::Digit1, Mods::CTRL);
        self.h.get_by_role(Role::TextInput, "").type_text(words);
        self.h.settle();
        self.h.advance(Duration::from_millis(200));
        self.h.settle();
    }

    /// The strip's tabs, by name; none while there is one tab.
    fn names(&self) -> Vec<String> {
        let tabs = self.h.query(|n| n.semantics.role == Role::Tab);
        tabs.into_iter().map(|id| self.h.tree.meaning(id).name).collect()
    }

    fn active(&self) -> String {
        self.tabs.current_untracked().name()
    }
}

#[test]
fn a_file_opens_in_a_tab_and_asks_nothing() {
    let (first, second) = (file("first.md", "# First\n"), file("second.md", "# Second\n"));
    let mut a = app(Some(first.clone()));
    assert!(a.names().is_empty(), "one tab: no strip");
    a.edit("x");
    a.shell.choose(&[&second.display().to_string()]);
    a.key(Key::O, Mods::CTRL);
    assert_eq!(a.names(), ["first.md", "second.md"]);
    assert_eq!(a.active(), "second.md");
    assert!(a.h.has_text("Second"));
    assert_eq!(a.shell.asked(), ["open_files: Open"], "nothing asked about the first's work");
    // The first keeps its work; opened again, its tab shows and no other is added.
    a.shell.choose(&[&first.display().to_string()]);
    a.key(Key::O, Mods::CTRL);
    assert_eq!((a.names().len(), a.active()), (2, "first.md".to_string()));
    assert!(a.tabs.current_untracked().changed.get_untracked());
    assert!(a.tabs.current_untracked().text.get_untracked().starts_with('x'));
}

#[test]
fn an_untitled_empty_tab_is_where_a_file_opens() {
    let path = file("into-blank.md", "# Blank no more\n");
    let mut a = app(None);
    a.shell.choose(&[&path.display().to_string()]);
    a.key(Key::O, Mods::CTRL);
    assert!(a.names().is_empty(), "still one tab");
    assert_eq!(a.active(), "into-blank.md");
}

#[test]
fn closing_a_tab_asks_about_its_work_and_the_keys_go_round() {
    let (first, second) = (file("close-1.md", "# One\n"), file("close-2.md", "# Two\n"));
    let mut a = app(Some(first));
    a.edit("y");
    a.shell.choose(&[&second.display().to_string()]);
    a.key(Key::O, Mods::CTRL);
    // Ctrl+Tab goes round: back to the first, which has unsaved work.
    a.key(Key::Tab, Mods::CTRL);
    assert_eq!(a.active(), "close-1.md");
    a.key(Key::Tab, Mods::CTRL | Mods::SHIFT);
    a.key(Key::Tab, Mods::CTRL | Mods::SHIFT);
    assert_eq!(a.active(), "close-1.md");
    a.shell.answer(Answer::Cancel);
    a.key(Key::W, Mods::CTRL);
    assert_eq!(a.names().len(), 2, "Cancel keeps it");
    a.shell.answer(Answer::No);
    a.key(Key::W, Mods::CTRL);
    assert_eq!(a.active(), "close-2.md");
    assert!(a.names().is_empty());
    // The last tab closed leaves an untitled one.
    a.key(Key::W, Mods::CTRL);
    assert_eq!(a.active(), "Untitled");
}

#[test]
fn closing_the_window_asks_for_each_tab_with_unsaved_work() {
    let (first, second) = (file("win-1.md", "# One\n"), file("win-2.md", "# Two\n"));
    let mut a = app(Some(first));
    a.edit("a");
    a.shell.choose(&[&second.display().to_string()]);
    a.key(Key::O, Mods::CTRL);
    a.edit("b");
    a.h.tree.take_window_requests();
    a.shell.answer(Answer::No);
    a.shell.answer(Answer::No);
    assert!(!a.h.tree.may_close(), "it asks first");
    a.h.settle();
    let asked: Vec<String> = a.shell.asked().into_iter().filter(|it| it.starts_with("message")).collect();
    assert_eq!(asked.len(), 2, "one question a tab");
    assert!(a.h.tree.take_window_requests().contains(&WindowRequest::CloseNow));
}

#[test]
fn a_later_start_hands_its_files_to_tabs_and_the_window_comes_to_the_front() {
    file("later-1.md", "# Later one\n");
    file("later-2.md", "# Later two\n");
    let mut a = app(None);
    a.h.tree.take_window_requests();
    let args = vec!["later-1.md".to_string(), "later-2.md".to_string()];
    a.later.push(Launch { folder: dir(), args });
    a.h.settle();
    assert_eq!(a.names(), ["later-1.md", "later-2.md"], "the blank tab took the first");
    assert_eq!(a.active(), "later-2.md");
    assert!(a.h.tree.take_window_requests().contains(&WindowRequest::Front));
}

#[test]
fn a_tab_opened_by_a_link_lives_on_when_its_tab_shows() {
    let from = file("link-a.md", "Go to the [next](link-b.md) one.\n");
    file("link-b.md", "# Linked here\n");
    let mut a = app(Some(from));
    let words = a.h.get_by_text("Go to the next one.").node();
    let at = a.h.tree.window_rect(words);
    // "next" is from the tenth letter on, 9.6 px each.
    let (x, y) = (at.x + 10.0 * 9.6 + 8.0, at.y + 12.0);
    for input in
        [quadra_tree::PointerInput::Move, quadra_tree::PointerInput::Down, quadra_tree::PointerInput::Up]
    {
        let now = a.h.now();
        a.h.tree.pointer(input, quadra::MouseButton::Left, quadra::Point::new(x, y), Mods::CTRL, now);
    }
    a.h.settle();
    assert_eq!(a.active(), "link-b.md");
    assert!(a.h.has_text("Linked here"));
    a.key(Key::Tab, Mods::CTRL);
    a.key(Key::Tab, Mods::CTRL);
    assert_eq!(a.active(), "link-b.md");
    assert!(a.h.has_text("Linked here"), "its text lives on");
    a.edit("z");
    assert!(a.tabs.current_untracked().changed.get_untracked());
}

#[test]
fn closing_the_window_saves_each_tab_when_told_to() {
    let (first, second) = (file("save-1.md", "# One\n"), file("save-2.md", "# Two\n"));
    let mut a = app(Some(first.clone()));
    a.edit("p");
    a.shell.choose(&[&second.display().to_string()]);
    a.key(Key::O, Mods::CTRL);
    a.edit("q");
    a.h.tree.take_window_requests();
    a.shell.answer(Answer::Yes);
    a.shell.answer(Answer::Yes);
    assert!(!a.h.tree.may_close());
    a.h.settle();
    assert_eq!(std::fs::read_to_string(&first).unwrap(), "p# One\n");
    assert_eq!(std::fs::read_to_string(&second).unwrap(), "q# Two\n");
    assert!(a.h.tree.take_window_requests().contains(&WindowRequest::CloseNow));
}

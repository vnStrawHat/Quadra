//! Settings (specs/19, "Settings"): the page writes them, and the app follows them: new
//! spaces, the signals, the tabs' place.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use oneterm::{Launch, NewSession, OneTerm, keymap};
use quadra::{Key, Mods, Role, SemanticAction, Settings, provide};
use quadra_term::vt::CursorShape;
use quadra_term::{Session, echo};
use quadra_testing::Harness;

struct App {
    h: Harness,
    made: Rc<RefCell<Vec<(Launch, Session)>>>,
    settings: Rc<Settings>,
}

fn app() -> App {
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 700.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let (made, settings) = (Rc::new(RefCell::new(Vec::new())), Settings::memory());
    let (kept, given) = (made.clone(), settings.clone());
    let sessions: NewSession = Rc::new(move |launch| {
        let s = echo();
        kept.borrow_mut().push((launch, s.clone()));
        s
    });
    h.mount(move || {
        provide(given.clone());
        OneTerm::new(sessions).view()
    });
    h.settle();
    App { h, made, settings }
}

impl App {
    fn feed(&mut self, n: usize, bytes: &[u8]) {
        let s = self.made.borrow()[n].1.clone();
        s.feeder().feed(bytes, Instant::now());
        self.h.settle();
    }

    fn key(&mut self, key: Key, mods: Mods) {
        self.h.key(key, mods);
        self.h.settle();
    }

    fn says(&self, words: &str) -> bool {
        self.h.has_text(words)
    }
}

#[test]
fn the_page_opens_from_the_title_bar_and_writes_what_is_chosen() {
    let mut a = app();
    a.h.get_by_role(Role::Button, "Settings").click();
    a.h.settle();
    assert!(a.says("Settings") && a.says("Appearance"));
    a.h.get_by_role(Role::Radio, "Dark").click();
    a.h.settle();
    assert_eq!(a.settings.value("appearance.theme", String::new()).get(), "Dark");

    a.h.get_by_role(Role::TreeItem, "Tabs & spaces").click();
    a.h.settle();
    assert!(a.says("Tabs & spaces"));
    let switches = a.h.query(|n| n.semantics.role == Role::Switch);
    assert_eq!(switches.len(), 1, "the page's one switch: vertical tabs");
    a.h.tree.act(switches[0], &SemanticAction::Click);
    a.h.settle();
    assert!(!a.settings.value("tabs.vertical", true).get());

    a.h.get_by_role(Role::Button, "Done").click();
    a.h.settle();
    assert!(!a.says("Tabs & spaces"));
}

#[test]
fn new_spaces_take_the_cursor_and_the_history_set() {
    let mut a = app();
    a.settings.value("terminal.cursor", String::new()).set("Bar".into());
    a.settings.value("terminal.scrollback", 0.0).set(500.0);
    a.h.settle();
    a.key(Key::T, Mods::CTRL);
    let made = a.made.borrow();
    let (launch, _) = made.last().expect("a new tab's space");
    assert_eq!((launch.cursor.shape, launch.scrollback), (CursorShape::Beam, 500));
}

#[test]
fn silent_notifications_and_ignored_bells_count_nothing() {
    let mut a = app();
    a.settings.value("signals.notifications", String::new()).set("Silent".into());
    a.settings.value("signals.bell", String::new()).set("Ignored".into());
    a.h.settle();
    a.feed(0, b"\x1b]9;tests passed\x07");
    assert!(!a.says("OSC 9"));
    assert!(a.says("0 bells and notices"));
    a.key(Key::D, Mods::CTRL);
    a.feed(0, b"\x07");
    assert!(a.says("0 bells and notices"), "a bell the settings ignore is seen as it rings");
}

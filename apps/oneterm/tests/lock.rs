//! The lock (specs/19, "Lock"): the password set on the settings page, locking at once and
//! after a time idle, what a locked window hides and refuses, the back-off, unlocking.

use std::rc::Rc;
use std::time::{Duration, Instant};

use oneterm::{NewSession, OneTerm, keymap};
use quadra::{Key, Mods, Role, Settings, provide};
use quadra_term::{Session, echo};
use quadra_testing::Harness;

struct App {
    h: Harness,
    first: Rc<std::cell::RefCell<Option<Session>>>,
    settings: Rc<Settings>,
}

fn app(password: Option<&str>) -> App {
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 700.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let settings = Settings::memory();
    if let Some(password) = password {
        let hash = quadra_shell::hash_password(password).expect("random bytes");
        settings.value("security.password", String::new()).set(hash);
    }
    let (made, given) = (Rc::new(std::cell::RefCell::new(None::<Session>)), settings.clone());
    let keeps = made.clone();
    let sessions: NewSession = Rc::new(move |_| {
        let s = echo();
        keeps.borrow_mut().get_or_insert_with(|| s.clone());
        s
    });
    h.mount(move || {
        provide(given.clone());
        OneTerm::new(sessions).view()
    });
    h.settle();
    App { h, first: made, settings }
}

impl App {
    fn key(&mut self, key: Key, mods: Mods) {
        self.h.key(key, mods);
        self.h.settle();
    }

    fn says(&self, words: &str) -> bool {
        self.h.has_text(words)
    }

    fn locked(&self) -> bool {
        self.says("1Term is locked")
    }

    fn tabs(&self) -> usize {
        self.h.query(|n| n.semantics.role == Role::Tab).len()
    }

    fn unlock_with(&mut self, password: &str) {
        self.h.get_by_test_id("unlock").set_value(password);
        self.h.settle();
        self.h.get_by_role(Role::Button, "Unlock").click();
        self.h.settle();
    }
}

#[test]
fn without_a_password_nothing_locks() {
    let mut a = app(None);
    assert!(a.h.query(|n| n.semantics.role == Role::Button && n.semantics.name == "Lock").is_empty());
    a.key(Key::L, Mods::CTRL | Mods::SHIFT);
    a.h.advance(Duration::from_secs(3600));
    a.h.settle();
    assert!(!a.locked());
}

#[test]
fn a_password_set_on_the_page_locks_and_opens_the_window() {
    let mut a = app(None);
    a.h.get_by_role(Role::Button, "Settings").click();
    a.h.settle();
    a.h.get_by_role(Role::TreeItem, "Security & lock").click();
    a.h.settle();
    a.h.get_by_test_id("password-new").set_value("open sesame");
    a.h.get_by_test_id("password-again").set_value("open sesame");
    a.h.settle();
    a.h.get_by_role(Role::Button, "Set password").click();
    a.h.settle();
    assert!(a.says("Password set"));
    let stored = a.settings.value("security.password", String::new()).get();
    assert!(stored.starts_with("pbkdf2-sha256$") && !stored.contains("open sesame"));
    a.h.get_by_role(Role::Button, "Done").click();
    a.h.settle();

    a.h.get_by_role(Role::Button, "Lock").click();
    a.h.settle();
    assert!(a.locked());
    assert!(a.says("Locked \u{B7} sessions are still running"));
    assert!(a.says("Locked \u{B7} details hidden"));
    assert!(a.says("1 sessions connected \u{B7} 0 notices"));
    assert_eq!(a.tabs(), 0, "the rail is not displayed, so readers do not get it");
    assert!(a.h.query(|n| n.semantics.name == "Terminal").is_empty());
    assert!(a.h.query(|n| n.semantics.name == "Settings").is_empty());

    a.unlock_with("open sesam");
    assert!(a.locked() && a.says("Wrong password"));
    a.unlock_with("open sesame");
    assert!(!a.locked());
    assert_eq!(a.tabs(), 1);
    let typing = a.h.query(|n| n.semantics.name == "Terminal" && n.focused);
    assert_eq!(typing.len(), 1, "the focus is back in the space");
}

#[test]
fn a_locked_window_refuses_the_app_keys_and_hides_cards() {
    let mut a = app(Some("pw"));
    a.key(Key::L, Mods::CTRL | Mods::SHIFT);
    assert!(a.locked());
    a.key(Key::T, Mods::CTRL);
    let s = a.first.borrow().clone().expect("the first space");
    s.feeder().feed(b"\x1b]9;secret result\x07", Instant::now());
    a.h.settle();
    assert!(!a.says("OSC 9"), "a card would show what a program said");
    assert!(a.says("1 sessions connected \u{B7} 1 notices"));
    a.unlock_with("pw");
    assert_eq!(a.tabs(), 1, "Ctrl+T did nothing while locked");
}

#[test]
fn it_locks_after_the_time_idle_and_activity_starts_it_again() {
    let mut a = app(Some("pw"));
    a.settings.value("security.lock_after_minutes", 0.0).set(1.0);
    a.h.advance(Duration::from_secs(50));
    a.h.settle();
    a.h.pointer(quadra_tree::PointerInput::Move, 600.0, 300.0);
    a.h.advance(Duration::from_secs(50));
    a.h.settle();
    assert!(!a.locked(), "the pointer moved 50 s ago");
    a.h.advance(Duration::from_secs(20));
    a.h.settle();
    assert!(a.locked());
    assert!(a.says("Locked after 1 min idle \u{B7} sessions are still running"));
}

#[test]
fn five_wrong_tries_start_a_back_off() {
    let mut a = app(Some("pw"));
    a.key(Key::L, Mods::CTRL | Mods::SHIFT);
    for _ in 0..5 {
        a.unlock_with("no");
    }
    assert!(a.says("Wrong password \u{B7} try again in 2 s"));
    a.unlock_with("pw");
    assert!(a.locked(), "the right password waits too");
    a.h.advance(Duration::from_secs(2));
    a.h.settle();
    a.unlock_with("pw");
    assert!(!a.locked());
}

//! Quick connect (specs/19, "M7 in detail"): the dialog makes the launch of a tab or a space,
//! a target that does not read is said, and a remote session's
//! glue reports a connection that failed. No host is ever reached: the sessions are
//! `echo()`, and the one real engine connects to a port nobody listens on.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use oneterm::{Launch, NewSession, OneTerm, Remote, keymap, remote_shell};
use oneterm_ssh::{Auth, Target};
use quadra::{Key, Mods, Role};
use quadra_term::echo;
use quadra_testing::Harness;

fn app() -> (Harness, Rc<RefCell<Vec<Launch>>>) {
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 700.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let made = Rc::new(RefCell::new(Vec::new()));
    let kept = made.clone();
    let sessions: NewSession = Rc::new(move |launch| {
        kept.borrow_mut().push(launch);
        echo()
    });
    h.mount(move || OneTerm::new(sessions).view());
    h.settle();
    (h, made)
}

fn connect(h: &mut Harness, target: &str) {
    h.key(Key::K, Mods::CTRL | Mods::SHIFT);
    h.settle();
    h.get_by_test_id("connect-target").set_value(target);
    h.settle();
    h.get_by_role(Role::Button, "Connect").click();
    h.settle();
}

fn tabs(h: &Harness) -> usize {
    h.query(|n| n.semantics.role == Role::Tab).len()
}

#[test]
fn a_target_opens_a_tab_that_connects_to_it() {
    let (mut h, made) = app();
    connect(&mut h, "ssh trung@10.20.3.41 -p 2222");
    let want = Remote::new(Target::parse("trung@10.20.3.41:2222").unwrap(), Auth::Auto);
    assert_eq!(made.borrow().last().and_then(|it| it.ssh.clone()), Some(want.clone()));
    assert_eq!(tabs(&h), 2);
}

#[test]
fn a_target_that_does_not_read_is_said_and_opens_nothing() {
    let (mut h, made) = app();
    connect(&mut h, "-x nowhere");
    assert_eq!(made.borrow().len(), 1, "the first tab's shell only");
    assert!(h.has_text("Not a target: -x nowhere"));
}

#[test]
fn in_the_current_tab_it_is_a_new_space() {
    let (mut h, made) = app();
    h.key(Key::K, Mods::CTRL | Mods::SHIFT);
    h.settle();
    h.get_by_role(Role::Checkbox, "In current tab (new space)").click();
    h.get_by_test_id("connect-target").set_value("a@h");
    h.settle();
    h.get_by_role(Role::Button, "Connect").click();
    h.settle();
    assert_eq!(tabs(&h), 1);
    assert_eq!(h.query(|n| n.semantics.name == "Terminal" && n.semantics.role == Role::TextInput).len(), 2);
    assert!(made.borrow()[1].ssh.is_some());
}

#[test]
fn a_remote_session_says_its_connection_failed() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let target = Target { user: "nobody".into(), host: "127.0.0.1".into(), port };
    let remote = Remote::new(target, Auth::Password);
    let mut h = Harness::new();
    let session = Rc::new(RefCell::new(None));
    let kept = session.clone();
    h.mount(move || {
        let s = remote_shell(Launch { ssh: Some(remote.clone()), ..Launch::default() });
        *kept.borrow_mut() = Some(s.clone());
        quadra_term::terminal(&s)
    });
    let s = session.borrow().clone().unwrap();
    assert_eq!(s.title().get_untracked(), format!("nobody@127.0.0.1:{port}"));
    let end = Instant::now() + Duration::from_secs(20);
    while s.exited().get_untracked().is_none() {
        assert!(Instant::now() < end, "the engine never ended");
        std::thread::sleep(Duration::from_millis(10));
        h.settle();
    }
    assert_eq!(s.exited().get_untracked(), Some(255));
    let said = s.screen_text().join("\n");
    assert!(said.contains("[connection closed:"), "{said}");
}

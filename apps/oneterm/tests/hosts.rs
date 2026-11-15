//! Saved hosts (specs/19, "M8 in detail"): added in the dialog, listed in the rail by group,
//! filtered, connected to on a click (recent, counted on the tab, named in the status bar),
//! saved from quick connect. The sessions are `echo()`: no host is reached.

use std::cell::RefCell;
use std::rc::Rc;

use oneterm::{Launch, NewSession, OneTerm, keymap};
use quadra::{Key, Mods, Role, Settings, provide};
use quadra_term::echo;
use quadra_testing::Harness;

struct App {
    h: Harness,
    made: Rc<RefCell<Vec<Launch>>>,
    settings: Rc<Settings>,
}

fn app() -> App {
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 800.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let (made, settings) = (Rc::new(RefCell::new(Vec::new())), Settings::memory());
    let (kept, given) = (made.clone(), settings.clone());
    let sessions: NewSession = Rc::new(move |launch| {
        kept.borrow_mut().push(launch);
        echo()
    });
    h.mount(move || {
        provide(given.clone());
        OneTerm::new(sessions).view()
    });
    h.settle();
    App { h, made, settings }
}

impl App {
    fn add(&mut self, name: &str, group: &str, target: &str) {
        self.h.get_by_role(Role::Button, "Add SSH host…").click();
        self.h.settle();
        for (id, value) in [("host-name", name), ("host-group", group), ("host-target", target)] {
            self.h.get_by_test_id(id).set_value(value);
        }
        self.h.settle();
        self.h.get_by_role(Role::Button, "Save").click();
        self.h.settle();
    }

    fn rows(&self) -> Vec<Vec<String>> {
        self.settings.value("ssh.hosts", Vec::<Vec<String>>::new()).get()
    }
}

#[test]
fn a_host_added_is_saved_listed_and_connects_on_a_click() {
    let mut a = app();
    assert!(a.h.has_text("local"), "the status bar names a local space");
    a.add("prod-api-01", "Production", "trung@10.20.1.14");
    a.add("", "Staging", "trung@10.30.2.8:2222");
    assert_eq!(a.rows()[0], ["prod-api-01", "Production", "trung@10.20.1.14", "Auto", "", "", ""]);
    assert_eq!(a.rows()[1][..3], ["10.30.2.8", "Staging", "trung@10.30.2.8:2222"]);
    assert!(a.h.has_text("Production") && a.h.has_text("Staging"));

    a.h.get_by_role(Role::Button, "prod-api-01").click();
    a.h.settle();
    let launch = a.made.borrow().last().cloned().unwrap();
    assert_eq!(launch.ssh.map(|it| it.target.label()), Some("trung@10.20.1.14".into()));
    assert!(a.h.has_text("1 ssh \u{B7} 1 space"), "the tab counts its kinds");
    assert!(a.h.has_text("prod-api-01"), "the status bar names the host");
    let recent = a.settings.value("ssh.recent", Vec::<Vec<String>>::new()).get();
    assert_eq!(recent[0][0], "trung@10.20.1.14");
}

#[test]
fn a_host_reached_through_another_carries_its_way_and_forwards() {
    let mut a = app();
    a.add("bastion", "", "ops@203.0.113.5");
    a.h.get_by_role(Role::Button, "Add SSH host…").click();
    a.h.settle();
    for (id, value) in
        [("host-name", "db"), ("host-target", "trung@10.0.0.9"), ("host-forwards", "5432:localhost:5432")]
    {
        a.h.get_by_test_id(id).set_value(value);
    }
    a.h.get_by_test_id("host-jump").click();
    a.h.settle();
    a.h.get_by_role(Role::MenuItem, "bastion").click();
    a.h.settle();
    a.h.get_by_role(Role::Button, "Save").click();
    a.h.settle();
    assert_eq!(a.rows()[1][5..], ["bastion", "5432:localhost:5432"]);

    a.h.get_by_role(Role::Button, "db").click();
    a.h.settle();
    let remote = a.made.borrow().last().cloned().unwrap().ssh.unwrap();
    let hops: Vec<String> = remote.jumps.iter().map(|it| it.target.label()).collect();
    assert_eq!(hops, ["ops@203.0.113.5"]);
    assert_eq!(remote.forwards[0].label(), "5432:localhost:5432");
}

#[test]
fn the_filter_keeps_the_hosts_that_match() {
    let mut a = app();
    a.add("prod-api-01", "Production", "trung@10.20.1.14");
    a.add("home-nas", "Personal", "admin@nas.local");
    a.h.get_by_test_id("host-filter").set_value("NAS");
    a.h.settle();
    assert!(a.h.query(|n| n.semantics.role == Role::Button && n.semantics.name == "prod-api-01").is_empty());
    assert_eq!(a.h.query(|n| n.semantics.role == Role::Button && n.semantics.name == "home-nas").len(), 1);
}

#[test]
fn quick_connect_can_save_the_host() {
    let mut a = app();
    a.h.key(Key::K, Mods::CTRL | Mods::SHIFT);
    a.h.settle();
    a.h.get_by_role(Role::Checkbox, "Save as host").click();
    a.h.get_by_test_id("connect-target").set_value("ops@10.20.0.2");
    a.h.settle();
    a.h.get_by_role(Role::Button, "Connect").click();
    a.h.settle();
    assert_eq!(a.rows()[0][..3], ["10.20.0.2", "", "ops@10.20.0.2"]);
}

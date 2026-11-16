//! Settings go back to their defaults (specs/19, "Settings"): a changed setting has its Reset,
//! and a page its Reset all.

use std::rc::Rc;

use oneterm::{NewSession, OneTerm, keymap};
use quadra::{Role, Settings, provide};
use quadra_term::echo;
use quadra_testing::Harness;

fn app() -> (Harness, Rc<Settings>) {
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 700.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let settings = Settings::memory();
    let given = settings.clone();
    let sessions: NewSession = Rc::new(|_| echo());
    h.mount(move || {
        provide(given.clone());
        OneTerm::new(sessions).view()
    });
    h.settle();
    h.get_by_role(Role::Button, "Settings").click();
    h.settle();
    (h, settings)
}

fn has_button(h: &Harness, name: &str) -> bool {
    !h.query(|n| n.semantics.role == Role::Button && n.semantics.name == name).is_empty()
}

#[test]
fn nothing_shows_a_reset_while_every_setting_is_its_default() {
    let (mut h, _) = app();
    for page in ["Appearance", "Terminal", "Tabs & spaces", "SSH & hosts", "SFTP"].into_iter().chain([
        "Notifications & signals",
        "Status bar",
        "Security & lock",
    ]) {
        h.get_by_role(Role::TreeItem, page).click();
        h.settle();
        let resets = h.query(|n| n.semantics.role == Role::Button && n.semantics.name.starts_with("Reset"));
        assert!(resets.is_empty(), "{page}: {:?}", h.semantics());
    }
}

#[test]
fn a_changed_setting_has_its_reset_and_a_page_its_reset_all() {
    let (mut h, s) = app();
    assert!(!has_button(&h, "Reset Theme") && !has_button(&h, "Reset all"));
    h.get_by_role(Role::Radio, "Dark").click();
    h.settle();
    assert!(has_button(&h, "Reset Theme") && has_button(&h, "Reset all"));
    h.get_by_role(Role::Button, "Reset Theme").click();
    h.settle();
    assert_eq!(s.value("appearance.theme", String::new()).get(), "System");
    assert!(!has_button(&h, "Reset Theme") && !has_button(&h, "Reset all"));

    // Reset all: the text size, and a row of two ANSI colours of which one was set.
    s.value("appearance.ui_text_percent", 0.0).set(120.0);
    s.value("terminal.colors.ansi.bright_red", String::new()).set("#400000".into());
    h.settle();
    assert!(has_button(&h, "Reset Text size") && has_button(&h, "Reset Red"));
    h.get_by_role(Role::Button, "Reset all").click();
    h.settle();
    assert_eq!(s.value("appearance.ui_text_percent", 0.0).get(), 100.0);
    assert_eq!(s.value("terminal.colors.ansi.bright_red", String::new()).get(), "");
    assert!(!has_button(&h, "Reset all"));

    // A choice by name follows its reset: the default shell is the OS's again.
    s.value("terminal.shell", String::new()).set("A program".into());
    h.get_by_role(Role::TreeItem, "Terminal").click();
    h.settle();
    assert!(!h.has_text("The OS's"), "the select shows the program");
    h.get_by_role(Role::Button, "Reset Default shell").click();
    h.settle();
    assert_eq!(s.value("terminal.shell", "x".to_string()).get(), "");
    assert!(h.has_text("The OS's") && !has_button(&h, "Reset all"));
}

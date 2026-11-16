//! Close tab (specs/19, "Rail"): the tab that shows has it at its end, in the rail and in the
//! strip; a press on it closes that tab and leaves the other one showing.

use std::rc::Rc;

use oneterm::{NewSession, OneTerm, keymap};
use quadra::{Key, Mods, Role, Settings, provide};
use quadra_term::echo;
use quadra_testing::Harness;

fn app(vertical: bool) -> Harness {
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 700.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let settings = Settings::memory();
    settings.value("tabs.vertical", true).set(vertical);
    let sessions: NewSession = Rc::new(|_| echo());
    h.mount(move || {
        provide(settings.clone());
        OneTerm::new(sessions).view()
    });
    h.settle();
    h
}

fn tabs(h: &Harness) -> Vec<(String, bool)> {
    let found = std::cell::RefCell::new(Vec::new());
    h.query(|n| {
        if n.semantics.role == Role::Tab {
            found.borrow_mut().push((n.semantics.name.clone(), n.semantics.selected));
        }
        false
    });
    found.into_inner()
}

#[test]
fn close_tab_closes_the_tab_that_shows_and_the_other_shows() {
    for vertical in [true, false] {
        let mut h = app(vertical);
        h.key(Key::T, Mods::CTRL);
        h.settle();
        assert_eq!(tabs(&h).len(), 2);
        let closes = h.query(|n| n.semantics.role == Role::Button && n.semantics.name == "Close tab");
        assert_eq!(closes.len(), 1, "only the tab that shows has it");
        // A press of the pointer, so the row under the button hears it too, unless stopped.
        let at = h.tree.window_rect(closes[0]).center();
        h.click(at.x, at.y);
        h.settle();
        assert_eq!(tabs(&h), [("Terminal".to_string(), true)], "vertical: {vertical}");
    }
}

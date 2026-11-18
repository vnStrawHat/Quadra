//! OSC 99 notifications as cards (specs/19, "Signals"): buttons, opening, a card that takes
//! the place of one with its id, and what the program asked to hear, written back to it.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use oneterm::{NewSession, OneTerm, keymap};
use quadra::{Key, Mods, Role};
use quadra_term::vt::ResizePolicy;
use quadra_term::{Link, Session};
use quadra_testing::Harness;

struct App {
    h: Harness,
    made: Rc<RefCell<Vec<Session>>>,
    wrote: Rc<RefCell<Vec<u8>>>,
}

fn app() -> App {
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 700.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let (made, wrote) = (Rc::new(RefCell::new(Vec::new())), Rc::new(RefCell::new(Vec::new())));
    let (kept, into) = (made.clone(), wrote.clone());
    let sessions: NewSession = Rc::new(move |_| {
        let s = Session::new(ResizePolicy::BottomAnchor);
        let into = into.clone();
        s.connect(Link::new(move |bytes| into.borrow_mut().extend_from_slice(bytes)));
        kept.borrow_mut().push(s.clone());
        s
    });
    h.mount(move || OneTerm::new(sessions).view());
    h.settle();
    App { h, made, wrote }
}

impl App {
    fn feed(&mut self, n: usize, bytes: &[u8]) {
        let s = self.made.borrow()[n].clone();
        s.feeder().feed(bytes, Instant::now());
        self.h.settle();
    }

    /// What was written to the programs since the last time, as text.
    fn told(&self) -> String {
        String::from_utf8_lossy(&std::mem::take(&mut *self.wrote.borrow_mut())).into_owned()
    }

    fn cards(&self) -> usize {
        self.h.query(|n| n.semantics.role == Role::Button && n.semantics.name == "Dismiss notification").len()
    }
}

#[test]
fn a_card_has_the_programs_buttons_takes_the_place_of_its_id_and_reports_what_was_asked() {
    let mut a = app();
    a.feed(
        0,
        "\x1b]99;i=b:d=0:a=report:c=1;Build failed\x1b\\\x1b]99;i=b:p=buttons;Log\u{2028}Retry\x1b\\"
            .as_bytes(),
    );
    assert_eq!(a.cards(), 1);
    a.h.get_by_role(Role::Button, "Log");
    // The same id again: the card is replaced, and the program hears no close.
    a.feed(0, b"\x1b]99;i=b:a=report:c=1;Build still failing\x1b\\");
    assert_eq!(a.told(), "");
    assert_eq!(a.cards(), 1);
    a.h.get_by_role(Role::Group, "Build still failing");
    a.feed(
        0,
        "\x1b]99;i=b:d=0:a=report:c=1;Build failed\x1b\\\x1b]99;i=b:p=buttons;Log\u{2028}Retry\x1b\\"
            .as_bytes(),
    );
    a.h.get_by_role(Role::Button, "Retry").click();
    a.h.settle();
    assert_eq!(a.told(), "\x1b]99;i=b;2\x1b\\\x1b]99;i=b:p=close;\x1b\\", "the button, then the close");
    assert_eq!(a.cards(), 0);
}

#[test]
fn a_program_closes_its_card_and_asks_which_show_and_opening_shows_the_space() {
    let mut a = app();
    a.feed(0, b"\x1b]99;i=one:a=report;Deploy done\x1b\\\x1b]99;i=two;Tests passed\x1b\\");
    a.told();
    a.feed(0, b"\x1b]99;i=two:p=close;\x1b\\\x1b]99;i=q:p=alive;\x1b\\");
    assert_eq!(a.cards(), 1);
    assert_eq!(a.told(), "\x1b]99;i=q:p=alive;one\x1b\\");
    // From another tab, opening it shows its tab again and tells the program.
    a.h.key(Key::T, Mods::CTRL);
    a.h.settle();
    let chosen = |a: &App| {
        let tabs = a.h.query(|n| n.semantics.role == Role::Tab);
        let picked = a.h.query(|n| n.semantics.role == Role::Tab && n.semantics.selected);
        tabs.iter().position(|id| picked.contains(id))
    };
    assert_eq!(chosen(&a), Some(1));
    a.h.get_by_role(Role::Button, "Deploy done").click();
    a.h.settle();
    assert_eq!(chosen(&a), Some(0));
    assert_eq!(a.told(), "\x1b]99;i=one;\x1b\\");
}

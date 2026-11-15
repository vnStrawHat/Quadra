//! Tabs and spaces (specs/19, "Rail", "Spaces") on `echo()` sessions: made, split, focused,
//! closed by keys, chosen and closed in the rail; a program that ended closes on a key.

use std::cell::RefCell;
use std::rc::Rc;

use oneterm::{NewSession, OneTerm, keymap};
use quadra::{Key, Mods, MouseButton, Point, Role};
use quadra_term::{Session, echo};
use quadra_testing::Harness;
use quadra_tree::PointerInput;

struct App {
    h: Harness,
    made: Rc<RefCell<Vec<Session>>>,
}

fn app() -> App {
    let mut h = Harness::new();
    h.size = quadra::Size::new(1200.0, 700.0);
    h.tree.set_keymap(Rc::new(keymap()));
    let made = Rc::new(RefCell::new(Vec::new()));
    let kept = made.clone();
    let sessions: NewSession = Rc::new(move || {
        let s = echo();
        kept.borrow_mut().push(s.clone());
        s
    });
    h.mount(move || OneTerm::new(sessions).view());
    h.settle();
    App { h, made }
}

impl App {
    fn key(&mut self, key: Key, mods: Mods) {
        self.h.key(key, mods);
        self.h.settle();
    }

    fn typed(&mut self, text: &str) {
        self.h.type_text(text);
        self.h.settle();
    }

    fn screen(&self, n: usize) -> String {
        self.made.borrow()[n].screen_text()[0].clone()
    }

    fn tabs(&self) -> usize {
        self.h.query(|n| n.semantics.role == Role::Tab).len()
    }
}

#[test]
fn keys_make_split_move_between_and_close_spaces_and_tabs() {
    let mut a = app();
    assert_eq!(a.tabs(), 1);
    a.key(Key::D, Mods::CTRL);
    assert!(a.h.has_text("Terminal \u{B7} 2 spaces"));
    a.typed("right");
    assert_eq!(a.screen(1), "right", "the new space has the focus");
    a.key(Key::Left, Mods::CTRL | Mods::ALT);
    a.typed("left");
    assert_eq!(a.screen(0), "left");

    a.key(Key::T, Mods::CTRL);
    assert_eq!(a.tabs(), 2);
    assert!(a.h.has_text("Terminal \u{B7} 1 space"));
    a.typed("third");
    assert_eq!(a.screen(2), "third");

    a.key(Key::W, Mods::CTRL); // the tab goes with its last space
    assert_eq!(a.tabs(), 1);
    assert!(a.h.has_text("Terminal \u{B7} 2 spaces"));
    a.key(Key::W, Mods::CTRL);
    assert!(a.h.has_text("Terminal \u{B7} 1 space"));
}

#[test]
fn the_rail_chooses_a_tab_and_the_middle_button_closes_one() {
    let mut a = app();
    a.key(Key::T, Mods::CTRL);
    a.made.borrow()[1].feeder().feed(b"\x1b]0;build\x07", std::time::Instant::now());
    a.h.settle();
    assert!(a.h.has_text("build \u{B7} 1 space"));
    a.h.get_by_role(Role::Tab, "Terminal").click();
    a.h.settle();
    assert!(a.h.has_text("Terminal \u{B7} 1 space"));
    a.typed("x");
    assert_eq!(a.screen(0), "x", "the tab chosen has the focus");

    let build = a.h.get_by_role(Role::Tab, "build").node();
    let at = a.h.tree.window_rect(build).center();
    for input in [PointerInput::Down, PointerInput::Up] {
        a.h.tree.pointer(input, MouseButton::Middle, Point::new(at.x, at.y), Mods::NONE, a.h.now());
    }
    a.h.settle();
    assert_eq!(a.tabs(), 1);
}

#[test]
fn a_program_that_ended_says_so_and_a_key_closes_its_space() {
    let mut a = app();
    a.key(Key::D, Mods::CTRL);
    a.made.borrow()[1].feeder().exited(2);
    a.h.settle();
    assert!(a.made.borrow()[1].screen_text().iter().any(|row| row == "[process exited 2]"));
    a.typed("q");
    assert!(a.h.has_text("Terminal \u{B7} 1 space"));
}

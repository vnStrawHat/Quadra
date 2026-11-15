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
    let sessions: NewSession = Rc::new(move |_| {
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

impl App {
    fn press(&mut self, button: MouseButton, x: f32, y: f32) {
        for input in [PointerInput::Down, PointerInput::Up] {
            self.h.tree.pointer(input, button, Point::new(x, y), Mods::NONE, self.h.now());
        }
        self.h.settle();
    }

    fn tab_at(&mut self, name: &str) -> Point {
        let node = self.h.get_by_role(Role::Tab, name).node();
        self.h.tree.window_rect(node).center()
    }

    fn names(&self) -> Vec<String> {
        let names = RefCell::new(Vec::new());
        self.h.query(|n| {
            if n.semantics.role == Role::Tab {
                names.borrow_mut().push(n.semantics.name.clone());
            }
            false
        });
        names.into_inner()
    }
}

#[test]
fn two_clicks_rename_a_tab_and_its_menu_closes_the_others() {
    let mut a = app();
    let at = a.tab_at("Terminal");
    a.press(MouseButton::Left, at.x, at.y);
    a.press(MouseButton::Left, at.x, at.y);
    a.h.get_by_role(Role::TextInput, "").set_value("logs");
    a.key(Key::Enter, Mods::NONE);
    assert!(a.h.has_text("logs \u{B7} 1 space"));

    a.key(Key::T, Mods::CTRL);
    a.key(Key::T, Mods::CTRL);
    assert_eq!(a.tabs(), 3);
    let at = a.tab_at("logs");
    a.press(MouseButton::Right, at.x, at.y);
    a.h.get_by_role(Role::MenuItem, "Close others").click();
    a.h.settle();
    assert_eq!(a.names(), ["logs"]);
}

#[test]
fn a_tab_dragged_onto_another_takes_its_place() {
    let mut a = app();
    a.key(Key::T, Mods::CTRL);
    a.made.borrow()[1].feeder().feed(b"\x1b]0;second\x07", std::time::Instant::now());
    a.h.settle();
    assert_eq!(a.names(), ["Terminal", "second"]);
    let (from, to) = (a.tab_at("second"), a.tab_at("Terminal"));
    a.h.tree.pointer(PointerInput::Down, MouseButton::Left, from, Mods::NONE, a.h.now());
    for step in 1..=4 {
        let y = from.y + (to.y - from.y) * step as f32 / 4.0;
        a.h.tree.pointer(PointerInput::Move, MouseButton::Left, Point::new(from.x, y), Mods::NONE, a.h.now());
    }
    a.h.tree.pointer(PointerInput::Up, MouseButton::Left, to, Mods::NONE, a.h.now());
    a.h.settle();
    assert_eq!(a.names(), ["second", "Terminal"]);
}

#[test]
fn the_tabs_go_into_a_strip_and_back_into_the_rail() {
    let mut a = app();
    a.h.get_by_role(Role::Button, "Horizontal tabs").click();
    a.h.settle();
    assert!(a.h.query(|n| n.semantics.name == "Horizontal tabs").is_empty());
    assert_eq!(a.tabs(), 1);
    a.typed("still");
    a.h.get_by_role(Role::Button, "Vertical").click();
    a.h.settle();
    assert_eq!(a.made.borrow().len(), 1, "the shell lives on");
    assert_eq!(a.screen(0), "still");
}

#[test]
fn the_plus_menu_makes_a_tab_or_splits_the_space() {
    let mut a = app();
    a.h.get_by_role(Role::Button, "New terminal").click();
    a.h.settle();
    a.h.get_by_role(Role::MenuItem, "Local shell").click();
    a.h.settle();
    assert_eq!(a.tabs(), 2);
    a.h.get_by_role(Role::Button, "New terminal").click();
    a.h.settle();
    a.h.get_by_role(Role::MenuItem, "Split current space").click();
    a.h.settle();
    assert!(a.h.has_text("Terminal \u{B7} 2 spaces"));
}

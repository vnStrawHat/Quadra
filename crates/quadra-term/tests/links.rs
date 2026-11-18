//! Links (specs/18, "Input"): an OSC 8 hyperlink and a URL in the text, one that wraps;
//! Ctrl+click follows one and is nothing else, a click without Ctrl selects as before, and the
//! hand and the underline while Ctrl is held. Cells are 6 x 14 px from the padding of 8 px,
//! 64 to a row.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use quadra_platform::{Cursor, Key, Mods, MouseButton};
use quadra_prim::Point;
use quadra_render::{Kind, Step};
use quadra_term::vt::ResizePolicy;
use quadra_term::{Link, Session, TermLink, terminal};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role};
use quadra_view::column;

struct Term {
    h: Harness,
    s: Session,
    wrote: Rc<RefCell<Vec<u8>>>,
    links: Rc<RefCell<Vec<TermLink>>>,
}

fn term() -> Term {
    let mut h = Harness::new();
    let slot = Rc::new(RefCell::new(None));
    let (wrote, links) = (Rc::new(RefCell::new(Vec::new())), Rc::new(RefCell::new(Vec::new())));
    let (out, into, followed) = (slot.clone(), wrote.clone(), links.clone());
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        s.connect(Link::new(move |bytes| into.borrow_mut().extend_from_slice(bytes)));
        *out.borrow_mut() = Some(s.clone());
        let view = terminal(&s).font(10.0, 14.0).on_link(move |link| followed.borrow_mut().push(link));
        column().size(400.0, 140.0).child(view)
    });
    h.settle();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    h.settle();
    let s = slot.borrow_mut().take().unwrap();
    Term { h, s, wrote, links }
}

/// The middle of a cell, in the window.
fn cell(row: usize, col: usize) -> Point {
    Point::new(8.0 + col as f32 * 6.0 + 3.0, 8.0 + row as f32 * 14.0 + 7.0)
}

const OSC8: &[u8] = b"\x1b]8;;https://example.com/a\x1b\\click here\x1b]8;;\x1b\\ or https://b.test/x.";

impl Term {
    fn feed(&mut self, bytes: &[u8]) {
        self.s.feeder().feed(bytes, Instant::now());
        self.h.settle();
    }

    fn press(&mut self, inputs: &[PointerInput], at: Point, mods: Mods) {
        for &input in inputs {
            self.h.tree.pointer(input, MouseButton::Left, at, mods, self.h.now());
        }
        self.h.settle();
    }

    fn ctrl_click(&mut self, row: usize, col: usize) {
        let all = [PointerInput::Move, PointerInput::Down, PointerInput::Up];
        self.press(&all, cell(row, col), Mods::CTRL);
    }

    fn followed(&self) -> Vec<TermLink> {
        std::mem::take(&mut *self.links.borrow_mut())
    }

    /// The lines one pixel high drawn: underlines.
    fn underlines(&mut self) -> usize {
        self.h.frame();
        let scene = self.h.tree.scene();
        let layers = self.h.renderer.steps().iter().filter_map(|step| match step {
            Step::Layer { id, .. } => Some(scene.layer(*id).instances()),
            _ => None,
        });
        layers.flatten().filter(|i| i.kind == Kind::Quad as u32 && i.bounds[3] == 1.0).count()
    }
}

fn link(url: &str, text: Option<&str>) -> TermLink {
    TermLink { url: url.into(), text: text.map(Into::into) }
}

#[test]
fn ctrl_click_follows_an_osc8_link_and_a_url_and_selects_nothing() {
    let mut t = term();
    t.feed(OSC8);
    t.ctrl_click(0, 3);
    assert_eq!(t.followed(), [link("https://example.com/a", Some("click here"))]);
    t.ctrl_click(0, 20);
    assert_eq!(t.followed(), [link("https://b.test/x", None)], "the full stop is not the URL's");
    assert!(t.s.selection_text().is_none(), "nothing selected");
    t.ctrl_click(0, 12);
    assert!(t.followed().is_empty(), "a word that is no link");
    // A link whose text is its target says no text.
    t.feed(b"\r\n\x1b]8;;https://c.test\x1b\\https://c.test\x1b]8;;\x1b\\");
    t.ctrl_click(1, 4);
    assert_eq!(t.followed(), [link("https://c.test", None)]);
    assert!(t.wrote.borrow().is_empty(), "nothing went to the program");
}

#[test]
fn a_url_that_wraps_is_one_link() {
    let mut t = term();
    let url = format!("https://example.com/{}", "a".repeat(60));
    t.feed(format!("{} {url} end", "x".repeat(40)).as_bytes());
    assert!(t.s.screen_text()[1].starts_with("aaaa"), "it wraps");
    t.ctrl_click(1, 5);
    assert_eq!(t.followed(), [link(&url, None)]);
    t.ctrl_click(0, 45);
    assert_eq!(t.followed(), [link(&url, None)], "from its first row too");
}

#[test]
fn a_program_that_reports_the_mouse_gets_neither_the_press_nor_its_release() {
    let mut t = term();
    t.feed(OSC8);
    t.feed(b"\x1b[?1000h\x1b[?1006h");
    t.ctrl_click(0, 3);
    assert_eq!(t.followed().len(), 1);
    assert!(t.wrote.borrow().is_empty(), "{:?}", t.wrote.borrow());
    // Off a link, the press is the program's as before.
    t.press(&[PointerInput::Down, PointerInput::Up], cell(0, 12), Mods::CTRL);
    assert!(!t.wrote.borrow().is_empty());
}

#[test]
fn a_click_without_ctrl_selects_as_before() {
    let mut t = term();
    t.feed(OSC8);
    let all = [PointerInput::Move, PointerInput::Down, PointerInput::Up];
    t.press(&all, cell(0, 3), Mods::NONE);
    let drag = [PointerInput::Down, PointerInput::Move, PointerInput::Up];
    let start = Point::new(cell(0, 0).x - 2.0, cell(0, 0).y);
    t.h.tree.pointer(drag[0], MouseButton::Left, start, Mods::NONE, t.h.now());
    t.press(&drag[1..], cell(0, 4), Mods::NONE);
    assert!(t.followed().is_empty());
    assert_eq!(t.s.selection_text().as_deref(), Some("click"));
}

#[test]
fn while_ctrl_is_held_the_link_is_underlined_under_a_hand() {
    let mut t = term();
    t.feed(OSC8);
    assert_eq!(t.underlines(), 0);
    t.press(&[PointerInput::Move], cell(0, 3), Mods::NONE);
    assert_eq!(t.h.tree.cursor(), Cursor::Text, "without Ctrl, nothing");
    // Ctrl pressed where the pointer is, then moved over the URL, then let go.
    t.h.tree.key(Key::Ctrl, 0, Mods::CTRL, true, false);
    t.h.settle();
    assert_eq!(t.h.tree.cursor(), Cursor::Pointer);
    assert_eq!(t.underlines(), 10, "a line under each cell of `click here`");
    t.press(&[PointerInput::Move], cell(0, 20), Mods::CTRL);
    assert_eq!(t.underlines(), 16, "of https://b.test/x");
    t.press(&[PointerInput::Move], cell(0, 12), Mods::CTRL);
    assert_eq!((t.h.tree.cursor(), t.underlines()), (Cursor::Text, 0), "off a link");
    t.press(&[PointerInput::Move], cell(0, 3), Mods::CTRL);
    t.h.tree.key(Key::Ctrl, 0, Mods::NONE, false, false);
    t.h.settle();
    assert_eq!((t.h.tree.cursor(), t.underlines()), (Cursor::Text, 0), "Ctrl let go");
}

#[test]
fn the_other_button_tells_the_link_under_it() {
    let mut t = term();
    t.feed(OSC8);
    let right = |t: &mut Term, at: Point, button: MouseButton| {
        for input in [PointerInput::Move, PointerInput::Down, PointerInput::Up] {
            t.h.tree.pointer(input, button, at, Mods::NONE, t.h.now());
        }
        t.h.settle();
    };
    right(&mut t, cell(0, 3), MouseButton::Right);
    assert_eq!(t.s.link_pressed(), Some(link("https://example.com/a", Some("click here"))));
    right(&mut t, cell(0, 3), MouseButton::Left);
    assert_eq!(t.s.link_pressed(), None, "a press of another button");
}

#[test]
fn the_pointer_takes_the_shape_the_program_names_and_a_link_still_shows_the_hand() {
    let mut t = term();
    t.feed(OSC8);
    t.press(&[PointerInput::Move], cell(0, 3), Mods::NONE);
    for (name, shape) in [("grabbing", Cursor::Move), ("default", Cursor::Arrow), ("no-such", Cursor::Text)] {
        t.feed(format!("\x1b]22;{name}\x1b\\").as_bytes());
        assert_eq!(t.h.tree.cursor(), shape, "{name}");
    }
    t.feed(b"\x1b]22;cell\x1b\\");
    t.h.tree.key(Key::Ctrl, 0, Mods::CTRL, true, false);
    t.h.settle();
    assert_eq!(t.h.tree.cursor(), Cursor::Pointer, "the link's hand over the program's");
    t.h.tree.key(Key::Ctrl, 0, Mods::NONE, false, false);
    t.h.settle();
    assert_eq!(t.h.tree.cursor(), Cursor::Crosshair);
}

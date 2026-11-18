//! Input to the program (specs/18, "Input"): keys in normal and application cursor mode, Ctrl
//! and Alt, Tab kept from the focus, paste, a drag that selects and copy (Shift over a program
//! that reports the mouse), the wheel. Cells are
//! 6 x 14 px from the box's corner and its padding of 8 px.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use quadra_platform::{Key, Mods, MouseButton};
use quadra_prim::Point;
use quadra_term::vt::ResizePolicy;
use quadra_term::{Link, Session, bind_keys, terminal};
use quadra_testing::Harness;
use quadra_tree::{Keymap, PointerInput, Role};
use quadra_view::column;

struct Term {
    h: Harness,
    s: Session,
    wrote: Rc<RefCell<Vec<u8>>>,
}

fn term() -> Term {
    let mut h = Harness::new();
    let mut keys = Keymap::default();
    bind_keys(&mut keys);
    h.tree.set_keymap(Rc::new(keys));
    let (slot, wrote) = (Rc::new(RefCell::new(None)), Rc::new(RefCell::new(Vec::new())));
    let (out, into) = (slot.clone(), wrote.clone());
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        s.connect(Link::new(move |bytes| into.borrow_mut().extend_from_slice(bytes)));
        *out.borrow_mut() = Some(s.clone());
        column().size(400.0, 140.0).child(terminal(&s).font(10.0, 14.0))
    });
    h.settle();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    h.settle();
    let s = slot.borrow_mut().take().unwrap();
    Term { h, s, wrote }
}

impl Term {
    fn feed(&mut self, bytes: &[u8]) {
        self.s.feeder().feed(bytes, Instant::now());
        self.h.settle();
    }

    /// What was written since the last call.
    fn taken(&mut self) -> Vec<u8> {
        self.h.settle();
        std::mem::take(&mut *self.wrote.borrow_mut())
    }

    fn key(&mut self, key: Key, mods: Mods) -> Vec<u8> {
        self.h.key(key, mods);
        self.taken()
    }

    /// The middle of a cell, in the window.
    fn cell(row: usize, col: usize) -> (f32, f32) {
        (8.0 + col as f32 * 6.0 + 3.0, 8.0 + row as f32 * 14.0 + 7.0)
    }
}

#[test]
fn keys_follow_the_cursor_mode_and_ctrl_and_alt_and_tab_stays() {
    let mut t = term();
    assert_eq!(t.key(Key::Up, Mods::NONE), b"\x1b[A");
    t.feed(b"\x1b[?1h");
    assert_eq!(t.key(Key::Up, Mods::NONE), b"\x1bOA");
    assert_eq!(t.key(Key::C, Mods::CTRL), b"\x03");
    assert_eq!(t.key(Key::X, Mods::ALT), b"\x1bx");
    assert_eq!(t.key(Key::Enter, Mods::NONE), b"\r");
    assert_eq!(t.key(Key::Tab, Mods::NONE), b"\t");
    assert_eq!(t.key(Key::Tab, Mods::SHIFT), b"\x1b[Z");
    // A printable key is the text it types.
    assert_eq!(t.key(Key::A, Mods::NONE), b"");
    t.h.type_text("ă");
    assert_eq!(t.taken(), "ă".as_bytes());
}

#[test]
fn a_press_focuses_the_terminal() {
    let mut t = term();
    t.h.tree.blur();
    let (x, y) = Term::cell(1, 1);
    t.h.click(x, y);
    t.h.type_text("a");
    assert_eq!(t.taken(), b"a");
}

#[test]
fn paste_is_bracketed_when_the_program_asks() {
    let mut t = term();
    t.h.clipboard.write_text("ls\n");
    assert_eq!(t.key(Key::V, Mods::CTRL | Mods::SHIFT), b"ls\r");
    t.feed(b"\x1b[?2004h");
    assert_eq!(t.key(Key::V, Mods::CTRL | Mods::SHIFT), b"\x1b[200~ls\n\x1b[201~");
}

#[test]
fn a_drag_selects_and_copy_gives_its_text() {
    let mut t = term();
    t.feed(b"hello world");
    let (x0, y) = Term::cell(0, 0);
    let (x1, _) = Term::cell(0, 4);
    t.h.pointer(PointerInput::Down, x0 - 2.0, y);
    t.h.pointer(PointerInput::Move, x1, y);
    t.h.pointer(PointerInput::Up, x1, y);
    t.h.settle(); // the pointer is handled in the update, as a frame would
    t.key(Key::C, Mods::CTRL | Mods::SHIFT);
    assert_eq!(t.h.clipboard.read_text().as_deref(), Some("hello"));
    assert!(t.taken().is_empty(), "nothing went to the program");
}

#[test]
fn the_wheel_scrolls_the_history_and_goes_to_a_program_that_reports_the_mouse() {
    let mut t = term();
    let lines: String = (0..20).map(|n| format!("line {n}\r\n")).collect();
    t.feed(lines.as_bytes());
    assert_eq!(t.s.screen_text()[0], "line 13");
    let (x, y) = Term::cell(2, 2);
    t.h.wheel(x, y, 3.0);
    t.h.settle();
    assert_eq!(t.s.screen_text()[0], "line 10");
    // Typing shows the bottom again.
    t.h.type_text("x");
    t.h.settle();
    assert_eq!(t.s.screen_text()[0], "line 13");
    t.taken();
    t.feed(b"\x1b[?1000h\x1b[?1006h");
    t.h.wheel(x, y, 1.0);
    assert_eq!(t.taken(), b"\x1b[<64;3;3M");
    t.h.tree.pointer(PointerInput::Down, MouseButton::Left, Point::new(x, y), Mods::NONE, t.h.now());
    assert_eq!(t.taken(), b"\x1b[<0;3;3M");
}

#[test]
fn shift_selects_over_a_program_that_reports_the_mouse() {
    let mut t = term();
    t.feed(b"hello world\x1b[?1000h\x1b[?1006h");
    let ((x0, y), (x1, _)) = (Term::cell(0, 0), Term::cell(0, 4));
    let press = |t: &mut Term, input, x: f32, mods| {
        let now = t.h.now();
        t.h.tree.pointer(input, MouseButton::Left, Point::new(x, y), mods, now);
        t.h.settle();
    };
    // Shift held: the terminal's selection, nothing reported.
    press(&mut t, PointerInput::Down, x0 - 2.0, Mods::SHIFT);
    press(&mut t, PointerInput::Move, x1, Mods::SHIFT);
    press(&mut t, PointerInput::Up, x1, Mods::SHIFT);
    assert!(t.taken().is_empty(), "nothing went to the program");
    t.key(Key::C, Mods::CTRL | Mods::SHIFT);
    assert_eq!(t.h.clipboard.read_text().as_deref(), Some("hello"));
    // Without it the program has the mouse again.
    press(&mut t, PointerInput::Down, x0, Mods::NONE);
    assert_eq!(t.taken(), b"\x1b[<0;1;1M");
}

#[test]
fn an_alt_drag_selects_a_block() {
    let mut t = term();
    t.feed(b"abcdef\r\nghijkl");
    let ((x0, y0), (x1, y1)) = (Term::cell(0, 1), Term::cell(1, 3));
    for (input, x, y) in
        [(PointerInput::Down, x0 - 2.0, y0), (PointerInput::Move, x1, y1), (PointerInput::Up, x1 + 2.0, y1)]
    {
        let now = t.h.now();
        t.h.tree.pointer(input, MouseButton::Left, Point::new(x, y), Mods::ALT, now);
        t.h.settle();
    }
    let text = t.s.selection_text().unwrap();
    assert_eq!(text.lines().collect::<Vec<_>>(), ["bcd", "hij"], "the same columns of each row");
}

#[test]
fn typed_text_is_a_key_a_character_and_a_capital_keeps_its_shift() {
    let mut t = term();
    t.h.type_text("Ab!");
    assert_eq!(t.taken(), b"Ab!");
    // kitty's report-all-keys and alternate keys: a key is its unshifted code.
    t.feed(b"\x1b[>12u");
    t.h.type_text("Lo?");
    assert_eq!(t.taken(), b"\x1b[108:76;2u\x1b[111u\x1b[47:63;2u");
}

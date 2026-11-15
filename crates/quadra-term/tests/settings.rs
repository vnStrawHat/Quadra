//! The view as the app's settings say (specs/18, "Input"): a selection copied on release, the
//! middle button that pastes, a right press left to the app, the wheel's speed and its arrows
//! on the alternate screen; and what a menu does on the session. Cells are 6 x 14 px.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use quadra_platform::{Mods, MouseButton};
use quadra_prim::Point;
use quadra_reactive::signal;
use quadra_term::vt::ResizePolicy;
use quadra_term::{Link, Session, TermSettings, terminal};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, PointerKind, Role};
use quadra_view::column;

struct Term {
    h: Harness,
    s: Session,
    wrote: Rc<RefCell<Vec<u8>>>,
    rights: Rc<RefCell<u32>>,
}

fn term(settings: TermSettings) -> Term {
    let mut h = Harness::new();
    let (slot, wrote, rights) =
        (Rc::new(RefCell::new(None)), Rc::new(RefCell::new(Vec::new())), Rc::default());
    let (out, into, counts) = (slot.clone(), wrote.clone(), Rc::clone(&rights));
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        s.connect(Link::new(move |bytes| into.borrow_mut().extend_from_slice(bytes)));
        *out.borrow_mut() = Some(s.clone());
        // What is around the view hears a right press the program does not take.
        let counts: Rc<RefCell<u32>> = counts;
        column()
            .size(400.0, 140.0)
            .on_pointer(PointerKind::Down, move |e| {
                if e.button == MouseButton::Right {
                    *counts.borrow_mut() += 1;
                }
            })
            .child(terminal(&s).font(10.0, 14.0).settings(signal(settings)))
    });
    h.settle();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    h.settle();
    let s = slot.borrow_mut().take().unwrap();
    Term { h, s, wrote, rights }
}

impl Term {
    fn feed(&mut self, bytes: &[u8]) {
        self.s.feeder().feed(bytes, Instant::now());
        self.h.settle();
    }

    fn taken(&mut self) -> Vec<u8> {
        self.h.settle();
        std::mem::take(&mut *self.wrote.borrow_mut())
    }

    fn press(&mut self, button: MouseButton, row: usize, col: usize) {
        let at = Point::new(8.0 + col as f32 * 6.0 + 3.0, 8.0 + row as f32 * 14.0 + 7.0);
        for input in [PointerInput::Down, PointerInput::Up] {
            self.h.tree.pointer(input, button, at, Mods::NONE, self.h.now());
        }
        self.h.settle();
    }
}

#[test]
fn a_selection_is_copied_on_release_and_the_middle_button_pastes() {
    let all = TermSettings { copy_on_select: true, middle_paste: true, ..TermSettings::default() };
    let mut t = term(all);
    t.feed(b"hello world");
    let y = 8.0 + 7.0;
    t.h.pointer(PointerInput::Down, 8.0, y);
    t.h.pointer(PointerInput::Move, 8.0 + 4.0 * 6.0 + 3.0, y);
    t.h.pointer(PointerInput::Up, 8.0 + 4.0 * 6.0 + 3.0, y);
    t.h.settle();
    assert_eq!(t.h.clipboard.read_text().as_deref(), Some("hello"));
    assert_eq!(t.s.selection_text().as_deref(), Some("hello"));
    t.press(MouseButton::Middle, 1, 1);
    assert_eq!(t.taken(), b"hello");

    // Off, the middle button does nothing; the right one is the app's either way.
    let mut t = term(TermSettings::default());
    t.h.clipboard.write_text("x");
    t.press(MouseButton::Middle, 1, 1);
    t.press(MouseButton::Right, 1, 1);
    assert!(t.taken().is_empty());
    assert_eq!(*t.rights.borrow(), 1, "the right press reached what is around the view");
}

#[test]
fn the_wheel_goes_as_fast_as_set_and_sends_arrows_on_the_alternate_screen() {
    let mut t = term(TermSettings { scroll: 2.0, alternate_scroll: true, ..TermSettings::default() });
    let lines: String = (0..30).map(|n| format!("line {n}\r\n")).collect();
    t.feed(lines.as_bytes());
    assert_eq!(t.s.screen_text()[0], "line 23");
    t.h.wheel(20.0, 20.0, 3.0);
    t.h.settle();
    assert_eq!(t.s.screen_text()[0], "line 17", "two rows a notch");
    t.feed(b"\x1b[?1049h");
    t.taken();
    t.h.wheel(20.0, 20.0, 1.0);
    assert_eq!(t.taken(), b"\x1b[A\x1b[A");
}

#[test]
fn a_menu_selects_everything_pastes_and_clears_the_history() {
    let mut t = term(TermSettings::default());
    let lines: String = (0..30).map(|n| format!("line {n}\r\n")).collect();
    t.feed(lines.as_bytes());
    t.s.select_all();
    assert!(t.s.selection_text().unwrap_or_default().starts_with("line 0"));
    t.s.paste("ls");
    assert_eq!(t.taken(), b"ls");
    t.s.clear_history();
    t.h.settle();
    t.h.wheel(20.0, 20.0, 3.0);
    t.h.settle();
    assert_eq!(t.s.screen_text()[0], "line 23", "no history left to scroll into");
}

#[test]
fn a_program_writes_the_clipboard_and_reads_it_only_when_let() {
    let mut t = term(TermSettings::default());
    t.feed(b"\x1b]52;c;aGVsbG8=\x07");
    assert_eq!(t.h.clipboard.read_text().as_deref(), Some("hello"));
    t.feed(b"\x1b]52;c;?\x07");
    assert!(t.taken().is_empty(), "no answer by default");

    let mut t = term(TermSettings { clipboard_read: true, ..TermSettings::default() });
    t.h.clipboard.write_text("hi");
    t.feed(b"\x1b]52;c;?\x07");
    assert_eq!(t.taken(), b"\x1b]52;c;aGk=\x07");
}

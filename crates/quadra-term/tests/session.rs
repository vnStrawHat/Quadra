//! A session and its view (specs/18, "Tests"): typing on `echo()`, output at its cells,
//! replies, resizing, the signals. On the fake backend at size 10 a cell is 6 px wide and the
//! lines 14 px high: a box of 400 x 140 holds 8 rows of 64 columns inside its padding.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use quadra_platform::{Key, Mods};
use quadra_term::vt::{Progress, ResizePolicy, Size};
use quadra_term::{Link, Notice, ProgramState, Session, TerminalWidget, bind_keys, echo, terminal};
use quadra_testing::Harness;
use quadra_tree::{Keymap, NodeId, Role};
use quadra_view::column;

struct Term {
    h: Harness,
    id: NodeId,
    s: Session,
    resized: Rc<RefCell<Vec<Size>>>,
}

fn term(echoes: bool) -> Term {
    let mut h = Harness::new();
    let mut keys = Keymap::default();
    bind_keys(&mut keys);
    h.tree.set_keymap(Rc::new(keys));
    let (slot, resized) = (Rc::new(RefCell::new(None)), Rc::new(RefCell::new(Vec::new())));
    let (out, told) = (slot.clone(), resized.clone());
    h.mount(move || {
        let s = if echoes { echo() } else { Session::new(ResizePolicy::BottomAnchor) };
        if !echoes {
            s.connect(Link::new(|_| {}).on_resize(move |size| told.borrow_mut().push(size)));
        }
        *out.borrow_mut() = Some(s.clone());
        column().size(400.0, 140.0).child(terminal(&s).font(10.0, 14.0))
    });
    h.settle();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    h.settle();
    let s = slot.borrow_mut().take().unwrap();
    Term { h, id, s, resized }
}

impl Term {
    fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        let replies = self.s.feeder().feed(bytes, Instant::now());
        self.h.settle();
        replies
    }
}

#[test]
fn typing_on_echo_shows_the_text() {
    let mut t = term(true);
    t.h.type_text("hi");
    t.h.key(Key::Enter, Mods::NONE);
    t.h.type_text("yo");
    t.h.settle();
    assert_eq!(&t.s.screen_text()[..2], ["hi", "yo"]);
}

#[test]
fn output_paints_a_glyph_a_cell_and_a_reader_is_told_the_rows() {
    let mut t = term(false);
    t.feed(b"abc \x1b[1mde\x1b[0m");
    let w = t.h.tree.widget_mut::<TerminalWidget>(t.id).unwrap();
    assert_eq!(w.tiles_painted(), 5); // the blank is no glyph
    assert!(t.h.has_text("abc de"));
}

#[test]
fn replies_come_back_in_order_and_the_status_query_is_answered() {
    let mut t = term(false);
    assert_eq!(t.feed(b"ab\x1b[6n\x1b[5n"), b"\x1b[1;3R\x1b[0n");
    assert_eq!(t.feed(b"\x1b]7501;?\x07"), b"\x1b]7501;?\x07");
    assert_eq!(t.feed(b"\x1b]7501;?\x1b\\"), b"\x1b]7501;?\x1b\\");
}

#[test]
fn the_box_sizes_the_engine_and_then_the_link() {
    let mut t = term(false);
    assert_eq!(t.s.size(), Size { rows: 8, cols: 64 });
    // Told the size it was connected at, then the box's.
    assert_eq!(*t.resized.borrow(), [Size { rows: 24, cols: 80 }, Size { rows: 8, cols: 64 }]);
    t.h.size = quadra_prim::Size::new(400.0, 600.0);
    t.h.settle();
    assert_eq!(t.resized.borrow().len(), 2, "the column keeps its size");
    // A transport connected after layout hears the grid it has.
    let late = Rc::new(RefCell::new(Vec::new()));
    let told = late.clone();
    t.s.connect(Link::new(|_| {}).on_resize(move |size| told.borrow_mut().push(size)));
    assert_eq!(*late.borrow(), [Size { rows: 8, cols: 64 }]);
}

#[test]
fn a_feeder_goes_to_another_thread() {
    fn sent<T: Send + 'static>(_: &T) {}
    let t = term(false);
    sent(&t.s.feeder());
}

#[test]
fn the_signals_follow_what_the_program_says() {
    let mut t = term(false);
    let notices = Rc::new(RefCell::new(Vec::new()));
    let seen = notices.clone();
    t.s.on_notification(move |n| seen.borrow_mut().push(n.clone()));
    t.feed(b"\x1b]0;build\x07\x1b]7;file://box/home/me\x07\x07\x07");
    assert_eq!(t.s.title().get_untracked(), "build");
    assert_eq!(t.s.cwd().get_untracked(), "/home/me");
    assert_eq!(t.s.bells().get_untracked(), 2);
    t.feed(b"\x1b]9;4;1;40\x07\x1b]9;done here\x07");
    assert_eq!(t.s.progress().get_untracked(), Some(Progress::Set(40)));
    assert_eq!(notices.borrow()[0], Notice { title: String::new(), body: "done here".into() });
    t.feed(b"\x1b]9;9;\"C:\\work\"\x07"); // a Windows shell's directory is no notice
    assert_eq!(t.s.cwd().get_untracked(), "C:\\work");
    assert_eq!(notices.borrow().len(), 1);
    t.feed(b"\x1b]9;4;0\x07\x1b]7501;state=blocked:id=deploy\x07");
    assert_eq!(t.s.progress().get_untracked(), None);
    assert_eq!(t.s.status().get_untracked(), ProgramState::Blocked);
    t.feed(b"\x1b]133;A\x07"); // a new prompt: what waited is over
    assert_eq!(t.s.status().get_untracked(), ProgramState::Idle);
    t.s.feeder().exited(3);
    t.h.settle();
    assert_eq!(t.s.exited().get_untracked(), Some(3));
}

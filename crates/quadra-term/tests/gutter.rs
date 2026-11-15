//! The gutter (specs/18, "The view"): the grid starts after it, the pointer's cells too, and
//! each row tells its line number and when it was written. Cells are 6 x 14 px, the room
//! around the grid 8 px; the gutter is 16 cells and 8 px, 104 px.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use quadra_platform::{Mods, MouseButton};
use quadra_prim::Point;
use quadra_reactive::{Signal, signal};
use quadra_term::vt::ResizePolicy;
use quadra_term::{Link, Session, TermLook, TerminalWidget, terminal};
use quadra_testing::Harness;
use quadra_tree::{NodeId, PointerInput, Role};
use quadra_view::column;

struct Term {
    h: Harness,
    s: Session,
    id: NodeId,
    look: Signal<TermLook>,
    wrote: Rc<RefCell<Vec<u8>>>,
}

fn term(gutter: bool) -> Term {
    let mut h = Harness::new();
    let look = signal(TermLook { size: 10.0, line_height: 1.4, gutter, ..TermLook::default() });
    let (slot, wrote) = (Rc::new(RefCell::new(None)), Rc::new(RefCell::new(Vec::new())));
    let (out, into) = (slot.clone(), wrote.clone());
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        s.connect(Link::new(move |bytes| into.borrow_mut().extend_from_slice(bytes)));
        *out.borrow_mut() = Some(s.clone());
        column().size(400.0, 140.0).child(terminal(&s).look(look))
    });
    h.settle();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    let s = slot.borrow_mut().take().unwrap();
    Term { h, s, id, look, wrote }
}

impl Term {
    fn feed(&mut self, bytes: &[u8], now: Instant) {
        self.s.feeder().feed(bytes, now);
        self.h.settle();
    }

    fn row(&mut self, r: usize) -> Option<(u64, Option<Instant>)> {
        self.h.tree.widget_mut::<TerminalWidget>(self.id).unwrap().gutter_row(r)
    }
}

#[test]
fn the_grid_is_narrower_by_the_gutter() {
    let mut t = term(false);
    assert_eq!(t.s.size().cols, 64, "(400 - 16) / 6");
    assert_eq!(t.row(0), None, "no gutter, no labels");
    t.look.update(|it| it.gutter = true);
    t.h.settle();
    assert_eq!(t.h.tree.widget_mut::<TerminalWidget>(t.id).unwrap().gutter_width(), 104.0);
    assert_eq!(t.s.size().cols, 46, "(400 - 104 - 16) / 6");
}

#[test]
fn a_click_is_the_cell_after_the_gutter() {
    let mut t = term(true);
    t.feed(b"\x1b[?1000h\x1b[?1006h", Instant::now());
    let at = Point::new(104.0 + 8.0 + 3.0 * 6.0 + 3.0, 8.0 + 14.0 + 5.0);
    t.h.tree.pointer(PointerInput::Down, MouseButton::Left, at, Mods::NONE, t.h.now());
    t.h.settle();
    assert_eq!(&*t.wrote.borrow(), b"\x1b[<0;4;2M");
}

#[test]
fn rows_tell_their_line_and_when_they_were_written() {
    let mut t = term(true);
    let t0 = Instant::now();
    let t1 = t0 + Duration::from_secs(3661);
    t.feed(b"a\r\nb", t0);
    t.feed(b"\r\nc", t1);
    assert_eq!(t.row(0), Some((1, Some(t0))));
    assert_eq!(t.row(1), Some((2, Some(t0))));
    assert_eq!(t.row(2), Some((3, Some(t1))));
    assert_eq!(t.row(3), Some((4, Some(t1))), "a row not written shows the newest");
}

#[test]
fn numbers_count_from_the_first_row_and_survive_a_column_change() {
    let mut t = term(true);
    let t0 = Instant::now();
    let t1 = t0 + Duration::from_secs(100);
    let lines: Vec<String> = (1..20).map(|n| format!("line {n}")).collect();
    t.feed(lines.join("\r\n").as_bytes(), t0);
    t.feed(b"\r\nline 20", t1);
    assert_eq!(t.s.screen_text()[0], "line 13");
    assert_eq!(t.row(0), Some((13, Some(t0))));
    assert_eq!(t.row(7), Some((20, Some(t1))));
    // More room around the grid: other columns, so every row has a new id, and a row less.
    t.look.update(|it| it.padding = 20.0);
    t.h.settle();
    assert_eq!(t.s.size().cols, 42, "(400 - 104 - 40) / 6");
    let shown = t.s.screen_text();
    let last = shown.iter().position(|it| it == "line 20").unwrap();
    assert_eq!(t.row(last), Some((20, Some(t1))));
    assert_eq!(t.row(last - 1), Some((19, Some(t0))));
    // A new line after it is the next number.
    t.feed(b"\r\nline 21", t1);
    let last = t.s.screen_text().iter().position(|it| it == "line 21").unwrap();
    assert_eq!(t.row(last).map(|it| it.0), Some(21));
}

#[test]
fn a_line_that_wraps_again_numbers_the_rows_after_it_again() {
    let mut t = term(true);
    let long = "x".repeat(44);
    t.feed(format!("a\r\n{long}\r\nb").as_bytes(), Instant::now());
    // The number of the row that shows `b`.
    let b = |t: &mut Term| {
        let at = t.s.screen_text().iter().position(|it| it == "b").unwrap();
        t.row(at).unwrap().0
    };
    assert_eq!(b(&mut t), 3);
    // 42 columns: the long line takes two rows (and a row less: `a` goes to the history).
    t.look.update(|it| it.padding = 20.0);
    t.h.settle();
    assert_eq!(b(&mut t), 4);
    t.look.update(|it| it.padding = 8.0);
    t.h.settle();
    assert_eq!(b(&mut t), 3);
}

#[test]
fn a_clear_with_the_history_numbers_from_one_again() {
    let mut t = term(true);
    let t0 = Instant::now();
    let t1 = t0 + Duration::from_secs(60);
    let lines: Vec<String> = (1..=30).map(|n| format!("line {n}")).collect();
    t.feed(lines.join("\r\n").as_bytes(), t0);
    assert_eq!(t.row(0).unwrap().0, 23);
    t.feed(b"\x1b[H\x1b[2J\x1b[3J$ ", t1);
    assert_eq!(t.row(0), Some((1, Some(t1))), "the screen's rows are stamped afresh");
}

#[test]
fn the_alternate_screen_has_no_labels() {
    let mut t = term(true);
    t.feed(b"\x1b[?1049hfull screen", Instant::now());
    assert_eq!(t.row(0), None);
    t.feed(b"\x1b[?1049l", Instant::now());
    assert_eq!(t.row(0).map(|it| it.0), Some(1));
}

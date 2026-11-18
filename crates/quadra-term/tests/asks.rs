//! What a program asks of the terminal and hears back (specs/18): the pointer's shape and its
//! query, the colour scheme and its updates, OSC 99 notifications.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use quadra_platform::{Cursor, Mods, MouseButton};
use quadra_prim::{Color, Point};
use quadra_reactive::{Signal, signal};
use quadra_term::vt;
use quadra_term::vt::ResizePolicy;
use quadra_term::{Link, Notice, Session, TermColors, TermLook, terminal};
use quadra_testing::Harness;
use quadra_tree::PointerInput;
use quadra_view::column;

struct Term {
    h: Harness,
    s: Session,
    look: Signal<TermLook>,
    wrote: Rc<RefCell<Vec<u8>>>,
}

fn term() -> Term {
    let mut h = Harness::new();
    let look = signal(TermLook { size: 10.0, line_height: 1.4, ..TermLook::default() });
    let (slot, wrote) = (Rc::new(RefCell::new(None)), Rc::new(RefCell::new(Vec::new())));
    let (out, into) = (slot.clone(), wrote.clone());
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        s.connect(Link::new(move |bytes| into.borrow_mut().extend_from_slice(bytes)));
        *out.borrow_mut() = Some(s.clone());
        column().size(400.0, 140.0).child(terminal(&s).look(look))
    });
    h.settle();
    let s = slot.borrow_mut().take().unwrap();
    Term { h, s, look, wrote }
}

impl Term {
    /// Feeds `bytes` and returns the replies.
    fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        let replies = self.s.feeder().feed(bytes, Instant::now());
        self.h.settle();
        replies
    }
}

#[test]
fn the_pointer_follows_pushes_pops_and_resets_and_a_query_is_answered_by_what_is_drawn() {
    let mut t = term();
    let now = t.h.now();
    t.h.tree.pointer(PointerInput::Move, MouseButton::Left, Point::new(20.0, 20.0), Mods::NONE, now);
    t.feed(b"\x1b]22;>pointer\x1b\\\x1b]22;>wait\x1b\\");
    assert_eq!(t.h.tree.cursor(), Cursor::Wait);
    t.feed(b"\x1b]22;<\x1b\\");
    assert_eq!(t.h.tree.cursor(), Cursor::Pointer, "back to the level below");
    t.feed(b"\x1b]22;<\x1b\\");
    assert_eq!(t.h.tree.cursor(), Cursor::Text, "an empty stack is the text cursor");
    // A program that leaves the alternate screen without tidying leaves no shape behind.
    t.feed(b"\x1b[?1049h\x1b]22;default\x1b\\");
    assert_eq!(t.h.tree.cursor(), Cursor::Arrow);
    t.feed(b"\x1b[?1049l");
    assert_eq!(t.h.tree.cursor(), Cursor::Text);
    // `zoom-in` has no cursor here: the answer says so.
    let answer = t.feed(b"\x1b]22;?pointer,zoom-in,__default__,__grabbed__\x1b\\");
    assert_eq!(answer, b"\x1b]22;1,0,text,default\x1b\\");
}

#[test]
fn the_scheme_is_dark_or_light_by_the_background_and_a_subscriber_hears_a_switch() {
    let mut t = term();
    assert_eq!(t.feed(b"\x1b[?996n"), b"\x1b[?997;1n", "the kit's dark theme");
    t.feed(b"\x1b[?2031h");
    t.wrote.borrow_mut().clear();
    let light = TermColors { background: Some(Color::rgb(0xfafafa)), ..TermColors::default() };
    t.look.update(|look| look.colors = light.clone());
    t.h.settle();
    assert_eq!(*t.wrote.borrow(), b"\x1b[?997;2n");
    // The same scheme again says nothing.
    t.look.update(|look| look.colors.foreground = Some(Color::rgb(0x202020)));
    t.h.settle();
    assert_eq!(t.wrote.borrow().len(), b"\x1b[?997;2n".len());
}

#[test]
fn an_osc99_notification_in_chunks_is_one_notice_and_a_program_asks_to_close_and_what_shows() {
    let mut t = term();
    let told = Rc::new(RefCell::new(Vec::new()));
    let (seen, closed, alive) = (told.clone(), told.clone(), told.clone());
    t.s.on_notification(move |n| seen.borrow_mut().push(format!("{n:?}")));
    t.s.on_notice_close(move |id| closed.borrow_mut().push(format!("close {id}")));
    t.s.on_notice_alive(move |id| alive.borrow_mut().push(format!("alive {id}")));
    // Without abilities declared (the app's), the query says titles and bodies.
    let answer = t.feed(b"\x1b]99;i=q:p=?;\x1b\\");
    assert!(answer.ends_with(b"p=title,body,?\x1b\\"), "{answer:?}");
    t.feed(b"\x1b]99;i=b:d=0:u=2:a=report,-focus:c=1;Build failed\x1b\\");
    assert!(told.borrow().is_empty(), "held until its last chunk");
    t.feed(b"\x1b]99;i=b:d=0:p=body:e=1;MyBlcnJvcnM=\x1b\\");
    t.feed("\x1b]99;i=b:p=buttons;Log\u{2028}Retry\x1b\\".as_bytes());
    let want = Notice {
        id: "b".into(),
        title: "Build failed".into(),
        body: "3 errors".into(),
        buttons: vec!["Log".into(), "Retry".into()],
        urgency: Some(vt::NotificationUrgency::Critical),
        focus: false,
        report_activation: true,
        report_close: true,
        ..Notice::default()
    };
    t.feed(b"\x1b]99;i=b:p=close;\x1b\\\x1b]99;i=q:p=alive;\x1b\\");
    assert_eq!(*told.borrow(), [format!("{want:?}"), "close b".into(), "alive q".into()]);
}

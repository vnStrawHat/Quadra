//! What the user sends, told to an app that repeats it elsewhere (specs/18, "Input"): keys,
//! text and pastes are told, the mouse and focus are not; a delivery writes as the other
//! session's own and tells nothing.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use quadra_platform::{Key, Mods};
use quadra_term::vt::ResizePolicy;
use quadra_term::{Input, Link, Session, bind_keys, terminal};
use quadra_testing::Harness;
use quadra_tree::{Keymap, PointerInput, Role};
use quadra_view::column;

/// What was told, owned: keys, text, a paste.
#[derive(Debug, PartialEq)]
enum Told {
    Keys(Vec<u8>),
    Text(String),
    Paste(String),
}

fn owned(input: &Input) -> Told {
    match *input {
        Input::Keys(bytes) => Told::Keys(bytes.to_vec()),
        Input::Text(text) => Told::Text(text.into()),
        Input::Paste(text) => Told::Paste(text.into()),
    }
}

/// A session that keeps what it is written.
fn recorded() -> (Session, Rc<RefCell<Vec<u8>>>) {
    let (s, wrote) = (Session::new(ResizePolicy::BottomAnchor), Rc::new(RefCell::new(Vec::new())));
    let into = wrote.clone();
    s.connect(Link::new(move |bytes| into.borrow_mut().extend_from_slice(bytes)));
    (s, wrote)
}

#[test]
fn keys_text_and_pastes_are_told_and_the_mouse_is_not() {
    let mut h = Harness::new();
    let mut keys = Keymap::default();
    bind_keys(&mut keys);
    h.tree.set_keymap(Rc::new(keys));
    let made = Rc::new(RefCell::new(None));
    let out = made.clone();
    h.mount(move || {
        let (s, wrote) = recorded();
        let view = terminal(&s).font(10.0, 14.0);
        *out.borrow_mut() = Some((s, wrote));
        column().size(400.0, 140.0).child(view)
    });
    h.settle();
    let (s, wrote) = made.borrow_mut().take().unwrap();
    let told = Rc::new(RefCell::new(Vec::new()));
    let kept = told.clone();
    s.on_input(move |input| kept.borrow_mut().push(owned(input)));
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    h.settle();

    h.key(Key::Up, Mods::NONE);
    h.type_text("ă");
    h.key(Key::Tab, Mods::NONE);
    h.clipboard.write_text("ls");
    h.key(Key::V, Mods::CTRL | Mods::SHIFT);
    h.settle(); // the view writes after the event
    s.paste("pwd");
    h.settle();
    let want = [
        Told::Keys(b"\x1b[A".to_vec()),
        Told::Text("ă".into()),
        Told::Keys(b"\t".to_vec()),
        Told::Paste("ls".into()),
        Told::Paste("pwd".into()),
    ];
    assert_eq!(*told.borrow(), want);

    // A program that reports the mouse gets a press, and nothing is told.
    s.feeder().feed(b"\x1b[?1000h\x1b[?1006h", Instant::now());
    h.settle();
    wrote.borrow_mut().clear();
    h.pointer(PointerInput::Down, 20.0, 20.0);
    h.settle();
    assert!(!wrote.borrow().is_empty(), "the press went to the program");
    assert_eq!(told.borrow().len(), want.len());
}

#[test]
fn a_delivery_is_written_as_the_other_sessions_own_and_tells_nothing() {
    let mut h = Harness::new();
    let made = Rc::new(RefCell::new(Vec::new()));
    let out = made.clone();
    h.mount(move || {
        out.borrow_mut().extend([recorded(), recorded()]);
        column()
    });
    let [(bracketed, wrote), (plain, plain_wrote)]: [_; 2] = made.take().try_into().ok().unwrap();
    let told = Rc::new(RefCell::new(0));
    let counts = told.clone();
    bracketed.on_input(move |_| *counts.borrow_mut() += 1);
    bracketed.feeder().feed(b"\x1b[?2004h", Instant::now());
    h.settle(); // the feed reaches the engine through the session's inbox
    for s in [&bracketed, &plain] {
        s.deliver(&Input::Paste("ls"));
        s.deliver(&Input::Keys(b"\x03"));
        s.deliver(&Input::Text("x"));
    }
    assert_eq!(*wrote.borrow(), b"\x1b[200~ls\x1b[201~\x03x");
    assert_eq!(*plain_wrote.borrow(), b"ls\x03x");
    assert_eq!(*told.borrow(), 0, "a delivery is not told");
}

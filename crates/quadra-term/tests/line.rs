//! The command line (specs/18, "The command line"): read from fed OSC 133 marks and from a
//! prompt's sign, the alternate screen, the caret; keys taken before the program. Cells are 6 x
//! 14 px inside the view's padding of 8 px; 64 columns.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use quadra_platform::{Key, Mods};
use quadra_term::vt::ResizePolicy;
use quadra_term::{CommandLine, Link, Session, bind_keys, terminal};
use quadra_testing::Harness;
use quadra_tree::{Keymap, Role};
use quadra_view::column;

struct Term {
    h: Harness,
    s: Session,
    wrote: Rc<RefCell<Vec<u8>>>,
    asked: Rc<RefCell<Vec<Key>>>,
}

/// A session that echoes what it is written, as a shell would, and keeps it; Up and Tab
/// without Shift are taken before the program.
fn term() -> Term {
    let mut h = Harness::new();
    let mut keys = Keymap::default();
    bind_keys(&mut keys);
    h.tree.set_keymap(Rc::new(keys));
    let (wrote, asked) = (Rc::new(RefCell::new(Vec::<u8>::new())), Rc::new(RefCell::new(Vec::<Key>::new())));
    let slot = Rc::new(RefCell::new(None));
    let (out, into, told) = (slot.clone(), Rc::clone(&wrote), Rc::clone(&asked));
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        let feeder = s.feeder();
        s.connect(Link::new(move |bytes| {
            into.borrow_mut().extend_from_slice(bytes);
            feeder.feed(bytes, Instant::now());
        }));
        *out.borrow_mut() = Some(s.clone());
        let view = terminal(&s).font(10.0, 14.0).on_key_first(move |key, mods| {
            told.borrow_mut().push(key);
            key == Key::Up || (key == Key::Tab && mods == Mods::NONE)
        });
        column().size(400.0, 140.0).child(view)
    });
    h.settle();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    h.settle();
    let s = slot.borrow_mut().take().unwrap();
    Term { h, s, wrote, asked }
}

impl Term {
    fn feed(&mut self, bytes: &[u8]) {
        self.s.feeder().feed(bytes, Instant::now());
        self.h.settle();
    }

    fn typed(&mut self, text: &str) {
        self.h.type_text(text);
        self.h.settle();
    }

    fn line(&self) -> CommandLine {
        self.s.command_line().get_untracked()
    }

    fn taken(&mut self) -> Vec<u8> {
        std::mem::take(&mut *self.wrote.borrow_mut())
    }
}

#[test]
fn the_line_runs_from_the_input_mark_to_the_cursor() {
    let mut t = term();
    t.s.command_line();
    t.feed(b"\x1b]133;A\x07me$ \x1b]133;B\x07");
    t.typed("git st");
    let line = t.line();
    assert_eq!((line.text.as_str(), line.at_prompt, line.alt_screen), ("git st", true, false));
    let caret = line.caret.expect("the focused view has a caret");
    assert_eq!((caret.x, caret.y, caret.width, caret.height), (8.0 + 10.0 * 6.0, 8.0, 6.0, 14.0));

    // The command runs: its output is no prompt.
    t.feed(b"\r\n\x1b]133;C\x07building");
    assert!(!t.line().at_prompt);
    t.feed(b"\r\n\x1b]133;D;0\x07\x1b]133;A\x07me$ \x1b]133;B\x07");
    let line = t.line();
    assert_eq!((line.text.as_str(), line.at_prompt), ("", true));
}

#[test]
fn a_line_that_wraps_is_read_from_its_first_row() {
    let mut t = term();
    t.s.command_line();
    t.feed(b"\x1b]133;A\x07$ \x1b]133;B\x07");
    let long = "echo ".to_string() + &"x".repeat(70);
    t.typed(&long);
    assert_eq!(t.line().text, long);
}

#[test]
fn without_marks_the_prompt_is_found_by_its_sign() {
    let mut t = term();
    t.s.command_line();
    t.feed(b"C:\\Users\\me>");
    t.typed("di");
    assert_eq!((t.line().text.as_str(), t.line().at_prompt), ("di", true));
    t.feed(b"\r\nsome output");
    assert_eq!((t.line().text.as_str(), t.line().at_prompt), ("some output", false));
}

#[test]
fn the_alternate_screen_has_no_line() {
    let mut t = term();
    t.s.command_line();
    t.feed(b"$ vim");
    t.feed(b"\x1b[?1049h~");
    let line = t.line();
    assert!(line.alt_screen && line.text.is_empty() && !line.at_prompt);
    t.feed(b"\x1b[?1049l");
    assert!(!t.line().alt_screen);
}

#[test]
fn keys_are_asked_first_and_typed_text_is_not() {
    let mut t = term();
    t.taken();
    t.h.key(Key::Up, Mods::NONE);
    t.h.key(Key::Tab, Mods::NONE);
    t.h.settle();
    assert!(t.taken().is_empty(), "Up and Tab were taken");
    t.h.key(Key::Down, Mods::NONE);
    t.h.key(Key::Tab, Mods::SHIFT);
    t.h.settle();
    assert_eq!(t.taken(), b"\x1b[B\x1b[Z");
    t.typed("ls");
    assert_eq!(t.taken(), b"ls");
    assert_eq!(*t.asked.borrow(), [Key::Up, Key::Tab, Key::Down, Key::Tab]);
}

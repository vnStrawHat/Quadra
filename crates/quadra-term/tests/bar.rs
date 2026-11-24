//! The history under the hand (specs/18, "Input"): the keys that go through it a page at a
//! time or to an end, and the bar down the view's right edge, whose thumb a drag moves. The
//! view is 400 by 140 px: eight rows of 14 px in its padding of 8.

use std::rc::Rc;
use std::time::Instant;

use quadra_platform::{Key, Mods};
use quadra_term::vt::ResizePolicy;
use quadra_term::{Session, bind_keys, terminal};
use quadra_testing::Harness;
use quadra_tree::{Keymap, PointerInput, Role};
use quadra_view::column;

fn term() -> (Harness, Session) {
    let mut h = Harness::new();
    let mut keys = Keymap::default();
    bind_keys(&mut keys);
    h.tree.set_keymap(Rc::new(keys));
    let slot = Rc::new(std::cell::RefCell::new(None));
    let out = slot.clone();
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        *out.borrow_mut() = Some(s.clone());
        column().size(400.0, 140.0).child(terminal(&s).font(10.0, 14.0))
    });
    h.settle();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    let s = slot.borrow_mut().take().unwrap();
    let lines: String = (0..40).map(|n| format!("line {n}\r\n")).collect();
    s.feeder().feed(lines.as_bytes(), Instant::now());
    h.settle();
    (h, s)
}

#[test]
fn the_keys_go_through_the_history_a_page_at_a_time_and_to_its_ends() {
    let (mut h, s) = term();
    let top = |s: &Session| s.screen_text()[0].clone();
    assert_eq!(top(&s), "line 33");
    let mut key = |key, mods| {
        h.key(key, mods);
        h.settle();
    };
    // A page is the rows that show less one.
    key(Key::PageUp, Mods::SHIFT);
    assert_eq!(top(&s), "line 26");
    key(Key::PageDown, Mods::SHIFT);
    assert_eq!(top(&s), "line 33");
    key(Key::Home, Mods::CTRL | Mods::SHIFT);
    assert_eq!(top(&s), "line 0");
    key(Key::PageUp, Mods::SHIFT);
    assert_eq!(top(&s), "line 0", "no further than the history goes");
    key(Key::End, Mods::CTRL | Mods::SHIFT);
    assert_eq!(top(&s), "line 33");
}

#[test]
fn the_thumb_down_the_right_edge_is_where_the_view_is_and_a_drag_moves_the_history() {
    let (mut h, s) = term();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    let around = h.tree.parent(id).expect("the box around the widget");
    let thumb = *h.tree.children(around).last().expect("the bar's thumb");
    // The pointer over the terminal: the bar learns where the view is, at the bottom of 33
    // rows of history, and its thumb is as long as the view is of it.
    h.pointer(PointerInput::Move, 100.0, 60.0);
    h.settle();
    let at = h.tree.window_rect(thumb);
    assert!((at.x, at.width) == (386.0, 14.0) && at.bottom() == 136.0, "{at:?}");
    assert!((at.height - 132.0 * 140.0 / (140.0 + 33.0 * 14.0)).abs() < 0.5, "{at:?}");

    // Dragged to the top: the first row kept shows.
    for (what, y) in [(PointerInput::Move, 120.0), (PointerInput::Down, 120.0), (PointerInput::Move, 5.0)] {
        h.pointer(what, 393.0, y);
    }
    h.pointer(PointerInput::Up, 393.0, 5.0);
    h.settle();
    assert_eq!(s.screen_text()[0], "line 0");
    assert_eq!(h.tree.focused(), Some(id), "the terminal keeps the focus");

    // A key moves the thumb with the view: at the bottom again.
    h.key(Key::End, Mods::CTRL | Mods::SHIFT);
    h.settle();
    assert_eq!(h.tree.window_rect(thumb).bottom(), 136.0);

    // The alternate screen has no history, and no thumb.
    s.feeder().feed(b"\x1b[?1049h", Instant::now());
    h.settle();
    h.pointer(PointerInput::Move, 100.0, 60.0);
    h.settle();
    assert_eq!(h.tree.window_rect(thumb).height, 0.0);
}

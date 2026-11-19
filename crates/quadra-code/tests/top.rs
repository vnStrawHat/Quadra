//! The line at the top of the editor (specs/13, "Code"): kept as the view scrolls, and the
//! view scrolled to the line it is set to, as for a preview that follows. Lines are 14 px.

use std::rc::Rc;

use quadra_code::{bind_keys, editor};
use quadra_platform::{Key, Mods};
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{Keymap, Role};
use quadra_view::column;

#[test]
fn the_line_at_the_top_follows_the_view_and_moves_it() {
    let top = signal(0usize);
    let text: String = (0..1000).map(|i| format!("line {i}\n")).collect();
    let mut h = Harness::new();
    let mut keys = Keymap::default();
    bind_keys(&mut keys);
    h.tree.set_keymap(Rc::new(keys));
    h.mount(move || column().size(400.0, 140.0).child(editor(text).font(10.0, 14.0).top_line(top)));
    h.frame();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    let settle = |h: &mut Harness| {
        h.frame();
        while h.frame_if_needed().is_some() {}
    };
    // The wheel moves the view: the line at its top is told.
    h.wheel(100.0, 70.0, -10.0);
    settle(&mut h);
    let y = h.tree.window_rect(id).y;
    assert!(y < 0.0 && top.get_untracked() == (-y / 14.0) as usize, "{y} {}", top.get_untracked());
    // Set, the line goes to the top of the view.
    top.set(500);
    settle(&mut h);
    assert_eq!((h.tree.window_rect(id).y, top.get_untracked()), (-500.0 * 14.0, 500));
    // Past where the view goes, it goes as far as it can, and says the line it shows.
    top.set(999);
    settle(&mut h);
    assert_eq!((h.tree.window_rect(id).y, top.get_untracked()), (-(1001.0 * 14.0 - 140.0), 991));
    // The keys move the view, and the line follows.
    h.tree.focus(id);
    h.key(Key::Home, Mods::CTRL);
    settle(&mut h);
    assert_eq!(top.get_untracked(), 0);
}

#[test]
fn the_position_is_a_fractional_line_its_wrapped_rows_counted() {
    let (at, caret) = (signal(0.0f64), signal(0usize));
    // Line 1 wraps to several rows.
    let text = format!("one\n{}\nthree\n{}", "word ".repeat(48), "x\n".repeat(200));
    let mut h = Harness::new();
    let mut keys = Keymap::default();
    bind_keys(&mut keys);
    h.tree.set_keymap(Rc::new(keys));
    h.mount(move || {
        let code = editor(text).font(10.0, 14.0).wrap(true).position(at).caret_line(caret);
        column().size(400.0, 140.0).child(code)
    });
    h.frame();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    let settle = |h: &mut Harness| {
        h.frame();
        while h.frame_if_needed().is_some() {}
    };
    let rows = |h: &Harness| -h.tree.window_rect(id).y / 14.0;
    // Set half way down the wrapped line: the view stands at its rows' middle.
    at.set(1.5);
    settle(&mut h);
    // Half its rows down: 1 + rows / 2, rows a whole count above one.
    let wrapped = (rows(&h) - 1.0) * 2.0;
    assert!(wrapped >= 3.0 && (wrapped - wrapped.round()).abs() < 1e-3, "{}", rows(&h));
    // The wheel moves it, and the line at the top is told with its fraction.
    h.wheel(100.0, 70.0, -1.0);
    settle(&mut h);
    let now = at.get_untracked();
    assert!(now > 1.5, "{now}");
    // The caret's line follows the keys.
    h.tree.focus(id);
    h.key(Key::Down, Mods::NONE);
    settle(&mut h);
    assert_eq!(caret.get_untracked(), 1);
}

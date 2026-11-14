//! The editor's view (specs/13, "Code"): keys, typing, composition, the clipboard of its
//! context, the pointer, scrolling. On the fake backend at size 10 a cell is 6 px wide; the
//! lines are 14 px high and the gutter 9 cells, 54 px.

use std::rc::Rc;
use std::time::Duration;

use quadra_code::{EditorWidget, bind_keys, editor};
use quadra_platform::{Key, Mods};
use quadra_testing::Harness;
use quadra_tree::{Keymap, NodeId, PointerInput, Role};
use quadra_ui::column;

struct Ed {
    h: Harness,
    id: NodeId,
}

/// An editor of 400 x 140 px (ten lines) at the window's corner, with the focus.
fn ed(text: &str, path: &str) -> Ed {
    let mut h = Harness::new();
    let mut keys = Keymap::default();
    bind_keys(&mut keys);
    h.tree.set_keymap(Rc::new(keys));
    let (text, path) = (text.to_string(), path.to_string());
    h.mount(move || column().size(400.0, 140.0).child(editor(text).path(path).font(10.0, 14.0)));
    h.frame();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    h.frame();
    Ed { h, id }
}

impl Ed {
    fn keys(&mut self, keys: &[(Key, Mods)]) {
        for &(key, mods) in keys {
            self.h.key(key, mods);
        }
        self.h.frame_if_needed();
    }

    fn w(&mut self) -> &mut EditorWidget {
        self.h.frame_if_needed();
        self.h.tree.widget_mut::<EditorWidget>(self.id).unwrap()
    }

    fn text(&mut self) -> String {
        self.w().doc.text()
    }

    fn head(&mut self) -> usize {
        self.w().doc.primary().head
    }
}

const NONE: Mods = Mods::NONE;

#[test]
fn typing_and_the_keys_edit_and_tab_indents_rather_than_moving_the_focus() {
    let mut e = ed("", "a.rs");
    e.h.type_text("fn main() {");
    e.keys(&[(Key::Enter, NONE)]);
    e.h.type_text("x");
    assert_eq!(e.text(), "fn main() {\n    x");
    e.keys(&[(Key::Home, NONE), (Key::Tab, NONE)]);
    assert_eq!(e.text(), "fn main() {\n        x");
    assert_eq!(e.h.tree.focused(), Some(e.id));
    e.keys(&[(Key::Tab, Mods::SHIFT), (Key::D, Mods::CTRL | Mods::SHIFT)]);
    assert_eq!(e.text(), "fn main() {\n    x\n    x");
    e.keys(&[(Key::Slash, Mods::CTRL)]);
    assert_eq!(e.text(), "fn main() {\n    // x\n    x"); // the cursor stays on the first
    e.keys(&[(Key::Up, Mods::ALT), (Key::K, Mods::CTRL | Mods::SHIFT)]);
    assert_eq!(e.text(), "fn main() {\n    x");
    e.keys(&[(Key::A, Mods::CTRL), (Key::Backspace, NONE)]);
    assert_eq!(e.text(), "");
}

#[test]
fn typing_after_a_pause_is_an_undo_of_its_own() {
    let mut e = ed("", "a.txt");
    e.h.type_text("one");
    e.h.type_text(" two");
    e.h.advance(Duration::from_secs(2));
    e.h.type_text(" three");
    e.keys(&[(Key::Z, Mods::CTRL)]);
    assert_eq!(e.text(), "one two");
    e.keys(&[(Key::Z, Mods::CTRL)]);
    assert_eq!(e.text(), "");
    e.keys(&[(Key::Y, Mods::CTRL)]);
    assert_eq!(e.text(), "one two");
}

#[test]
fn several_cursors_and_the_clipboard_of_the_context() {
    let mut e = ed("let a = a + a;", "a.js");
    e.keys(&[(Key::Right, Mods::CTRL), (Key::Right, Mods::CTRL)]); // after "let a"
    e.keys(&[(Key::Left, Mods::SHIFT), (Key::D, Mods::CTRL), (Key::D, Mods::CTRL)]);
    assert_eq!(e.w().doc.cursors.len(), 3);
    e.h.type_text("b");
    assert_eq!(e.text(), "let b = b + b;");
    e.keys(&[(Key::Escape, NONE)]);
    assert_eq!(e.w().doc.cursors.len(), 1);
    e.keys(&[(Key::C, Mods::CTRL)]); // nothing selected: the line
    assert_eq!(e.h.clipboard.read_text().as_deref(), Some("let b = b + b;\n"));
    e.keys(&[(Key::End, Mods::CTRL), (Key::V, Mods::CTRL)]);
    assert_eq!(e.text(), "let b = b + b;let b = b + b;\n");
}

#[test]
fn a_composition_shows_at_the_caret_and_only_what_is_typed_goes_in() {
    let mut e = ed("ab", "a.txt");
    e.keys(&[(Key::Right, NONE)]);
    e.h.compose("vie");
    assert_eq!(e.w().preedit.as_ref().map(|p| p.0.as_str()), Some("vie"));
    assert_eq!(e.text(), "ab");
    e.h.type_text("việt");
    assert_eq!((e.text().as_str(), e.w().preedit.is_none()), ("avi\u{1ec7}tb", true));
}

#[test]
fn a_click_puts_the_cursor_a_drag_selects_and_two_clicks_take_a_word() {
    let mut e = ed("alpha beta\ngamma delta\n", "a.txt");
    let (x, y) = (54.0 + 7.0 * 6.0, 7.0); // the line 1, column 7: "be|ta"
    e.h.click(x, y);
    assert_eq!(e.head(), 7);
    e.h.advance(Duration::from_secs(1)); // not a second click
    e.h.pointer(PointerInput::Down, x, y);
    e.h.pointer(PointerInput::Move, 54.0 + 2.0 * 6.0, 14.0 + 7.0);
    e.h.pointer(PointerInput::Up, 54.0 + 2.0 * 6.0, 14.0 + 7.0);
    let c = e.w().doc.primary();
    assert_eq!((c.anchor, c.head), (7, 13));
    e.h.advance(Duration::from_secs(1));
    e.h.click(54.0 + 1.0 * 6.0, 7.0);
    e.h.click(54.0 + 1.0 * 6.0, 7.0);
    assert_eq!(e.w().doc.primary().range(), 0..5);
}

#[test]
fn the_caret_is_kept_in_view_and_only_the_lines_that_show_are_painted() {
    let text: String = (0..100_000).map(|i| format!("line {i} {}\n", "x".repeat(i % 50))).collect();
    let mut e = ed(&text, "a.txt");
    let painted = e.w().tiles_painted();
    assert!(painted < 2_000, "{painted} glyphs for ten lines");
    e.keys(&[(Key::End, Mods::CTRL)]);
    e.h.frame();
    let y = e.h.tree.window_rect(e.id).y;
    assert_eq!(y, -(100_001.0 * 14.0 - 140.0)); // the last line at the bottom
    // A line far longer than the view is scrolled to its end.
    e.keys(&[(Key::Up, NONE), (Key::End, NONE)]);
    assert!(e.w().scroll_x > 0.0);
}

#[test]
fn the_find_bar_finds_goes_to_matches_replaces_and_gives_the_focus_back() {
    let mut e = ed("one two one\nthree one", "a.txt");
    e.keys(&[(Key::F, Mods::CTRL)]);
    e.h.frame();
    assert_ne!(e.h.tree.focused(), Some(e.id)); // the query has it
    e.h.type_text("one");
    e.h.frame();
    assert_eq!(e.w().matches, vec![0..3, 8..11, 18..21]);
    assert!(e.h.has_text("1 of 3"));
    e.keys(&[(Key::Enter, NONE)]);
    assert_eq!(e.w().doc.primary().range(), 8..11);
    assert!(e.h.has_text("2 of 3"));
    e.keys(&[(Key::F3, Mods::SHIFT)]);
    assert_eq!(e.w().doc.primary().range(), 0..3);
    e.h.get_by_role(Role::Button, ".*").click();
    e.h.frame();
    e.h.get_by_role(Role::TextInput, "Find").set_value("o(");
    e.h.frame();
    assert!(e.h.has_text("Bad pattern"));
    e.keys(&[(Key::Escape, NONE)]);
    e.h.frame();
    assert_eq!(e.h.tree.focused(), Some(e.id));
    assert!(!e.h.has_text("Bad pattern"));
    assert!(e.w().matches.is_empty());

    // Ctrl-H: what replaces them, all at once and undone at once.
    e.keys(&[(Key::H, Mods::CTRL)]);
    e.h.frame();
    e.h.get_by_role(Role::TextInput, "Find").set_value("one");
    e.h.get_by_role(Role::TextInput, "Replace").set_value("1");
    e.h.frame();
    e.h.get_by_role(Role::Button, "All").click();
    e.h.frame();
    assert_eq!(e.text(), "1 two 1\nthree 1");
    assert!(e.h.has_text("No results"));

    // Escape in the editor closes the bar and leaves the cursors; a second gives one.
    e.h.tree.focus(e.id);
    e.keys(&[(Key::Home, Mods::CTRL), (Key::Down, Mods::ALT | Mods::SHIFT), (Key::Escape, NONE)]);
    e.h.frame();
    assert!(!e.h.has_text("No results"));
    let cursors = e.w().doc.cursors.len();
    e.keys(&[(Key::Escape, NONE)]);
    assert_eq!((cursors > 1, e.w().doc.cursors.len()), (true, 1));
}

/// An editor ten cells wide (the gutter's 54 px and 60 more) that wraps, with the focus.
fn wrapped(text: &str) -> Ed {
    let mut h = Harness::new();
    let mut keys = Keymap::default();
    bind_keys(&mut keys);
    h.tree.set_keymap(Rc::new(keys));
    let text = text.to_string();
    h.mount(move || column().size(114.0, 140.0).child(editor(text).wrap(true).font(10.0, 14.0)));
    h.frame();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    h.frame();
    Ed { h, id }
}

#[test]
fn wrapped_lines_break_after_blanks_and_the_rows_drive_up_down_and_the_pointer() {
    // "alpha beta " | "gamma " | "delta", then a word longer than a row in pieces of ten.
    let mut e = wrapped("alpha beta gamma delta\nabcdefghijklmnopqrstuvwxy");
    assert_eq!(e.h.tree.rect(e.id).height, 6.0 * 14.0);
    let rows: Vec<_> = (0..6).map(|row| e.w().row_span(row)).collect();
    assert_eq!(rows, vec![0..11, 11..17, 17..22, 23..33, 33..43, 43..48]);
    // Down goes a row, in the column it came from; End is the line's.
    e.keys(&[(Key::Right, NONE), (Key::Right, NONE), (Key::Down, NONE)]);
    assert_eq!(e.head(), 13);
    e.keys(&[(Key::Down, NONE), (Key::Down, NONE)]);
    assert_eq!(e.head(), 25);
    e.keys(&[(Key::Up, NONE), (Key::End, NONE)]);
    assert_eq!(e.head(), 22);
    // A click on the third row, two cells in; past a row's end stays in the row.
    e.h.click(54.0 + 2.0 * 6.0, 2.0 * 14.0 + 7.0);
    assert_eq!(e.head(), 19);
    e.h.advance(Duration::from_secs(1));
    e.h.click(54.0 + 9.0 * 6.0, 1.0 * 14.0 + 7.0);
    assert_eq!(e.head(), 16); // the blank that hangs at the end of "gamma "
    // Typing makes a row more, and the editor taller.
    e.keys(&[(Key::End, Mods::CTRL)]);
    e.h.type_text("zzzzzz");
    e.h.frame();
    assert_eq!(e.h.tree.rect(e.id).height, 7.0 * 14.0);
    assert_eq!(e.w().scroll_x, 0.0);
}

#[test]
fn ctrl_and_a_press_on_a_link_opens_it_and_moves_no_cursor() {
    use quadra_platform::MouseButton;
    use quadra_prim::Point;
    let opened = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let seen = opened.clone();
    let mut h = Harness::new();
    let text = "see www.example.com/a now".to_string();
    h.mount(move || {
        let on = editor(text).font(10.0, 14.0).on_link(move |url| seen.borrow_mut().push(url.to_string()));
        column().size(400.0, 140.0).child(on)
    });
    h.frame();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    let at = Point::new(54.0 + 8.0 * 6.0, 7.0); // on "example"
    let now = h.now();
    h.tree.pointer(PointerInput::Move, MouseButton::None, at, Mods::CTRL, now);
    h.frame();
    let mut e = Ed { h, id };
    assert_eq!(e.h.tree.cursor(), quadra_platform::Cursor::Pointer); // the pointer of a link
    e.h.tree.pointer(PointerInput::Down, MouseButton::Left, at, Mods::CTRL, now);
    e.h.tree.pointer(PointerInput::Up, MouseButton::Left, at, Mods::CTRL, now);
    e.h.frame();
    assert_eq!(*opened.borrow(), vec!["https://www.example.com/a".to_string()]);
    assert_eq!((e.head(), e.w().doc.cursors.len()), (0, 1));
    // Without Ctrl it is text like any other: the press puts the cursor.
    e.h.advance(Duration::from_secs(1));
    e.h.click(at.x, at.y);
    assert_eq!(e.head(), 8);
}

#[test]
fn the_gutter_marks_what_differs_from_the_base() {
    use quadra_code::{Mark, marks};
    let base = "a\nb\nc\nd\ne";
    assert_eq!(marks(base, "a\nB\nc\nd\ne"), [Mark::None, Mark::Changed, Mark::None, Mark::None, Mark::None]);
    assert_eq!(marks(base, "a\nb\nnew\nc\nd\ne")[2], Mark::Added);
    assert_eq!(marks(base, "a\nc\nd\ne"), [Mark::None, Mark::Removed, Mark::None, Mark::None]);
    assert_eq!(marks(base, "a\nb\nc\nd"), [Mark::None, Mark::None, Mark::None, Mark::Removed]);
    // The editor finds them again as it is edited.
    let mut h = Harness::new();
    h.mount(move || column().size(400.0, 140.0).child(editor(base).base(base).font(10.0, 14.0)));
    h.frame();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    let mut e = Ed { h, id };
    assert!(e.w().marks.iter().all(|m| *m == Mark::None));
    e.h.type_text("x");
    assert_eq!(e.w().marks[0], Mark::Changed);
}

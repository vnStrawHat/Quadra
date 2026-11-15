//! The text area (behaviour/09): text on several lines. Enter starts a line, the caret moves
//! up and down keeping its place, lines are broken at the area's width, and a scroll view
//! around it keeps the caret in view. On the fake backend at size 10 a character is 6 px
//! wide and a line 14 px high.

use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{NodeId, TextField};
use quadra_view::{column, div, mount_root, scroll, text_area, text_input};

struct Area {
    h: Harness,
    notes: Signal<String>,
    field: NodeId,
}

/// A text area in a box `width` px wide, at the top left of the window, with the focus.
fn area(text: &str, width: f32) -> Area {
    let notes = signal(text.to_string());
    let mut h = Harness::new();
    mount_root(&mut h.tree, column().child(div().width(width).child(text_area(notes).size(10.0))));
    h.frame();
    let field = h.tree.children(h.tree.children(h.tree.root().unwrap())[0])[0];
    h.tree.focus(field);
    Area { h, notes, field }
}

impl Area {
    fn keys(&mut self, keys: &[(Key, Mods)]) {
        for &(key, mods) in keys {
            self.h.key(key, mods);
        }
    }

    /// The caret and the selection, once what was typed and pressed has been done.
    fn caret(&mut self) -> (usize, std::ops::Range<usize>) {
        self.h.frame_if_needed();
        let field = self.h.tree.widget_mut::<TextField>(self.field).unwrap();
        (field.buffer.head(), field.buffer.selection())
    }
}

#[test]
fn enter_starts_a_line_and_the_caret_goes_up_and_down_keeping_its_place() {
    let mut a = area("", 300.0);
    // All in one turn, no frame between: the lines are those of the text as it is.
    a.h.type_text("d\u{F2}ng m\u{1ED9}t"); // "dong mot" with its marks
    a.keys(&[(Key::Enter, Mods::NONE)]);
    a.h.type_text("hai");
    a.keys(&[(Key::Up, Mods::NONE)]);
    a.h.type_text("!");
    a.h.frame();
    assert_eq!(a.notes.get_untracked(), "d\u{F2}n!g m\u{1ED9}t\nhai"); // three characters in, as it was
    assert_eq!(a.h.tree.rect(a.field).height, 28.0); // two lines

    // Through a line too short for it, the caret comes back to where it was.
    a.notes.set("abcdef\nx\nabcdef".to_string());
    a.h.frame();
    a.keys(&[(Key::Home, Mods::CTRL), (Key::End, Mods::NONE)]);
    assert_eq!(a.caret().0, 6); // the end of the first line, not of the text
    a.keys(&[(Key::Down, Mods::NONE)]);
    assert_eq!(a.caret().0, 8);
    a.keys(&[(Key::Down, Mods::NONE)]);
    assert_eq!(a.caret().0, 15);
    a.keys(&[(Key::Down, Mods::NONE)]); // from the last line: stays at the end
    a.keys(&[(Key::Home, Mods::NONE)]);
    assert_eq!(a.caret().0, 9);
    a.keys(&[(Key::Up, Mods::SHIFT), (Key::Up, Mods::SHIFT), (Key::Up, Mods::SHIFT)]);
    assert_eq!(a.caret(), (0, 0..9)); // selected up to the start, across the lines
    a.keys(&[(Key::End, Mods::CTRL)]);
    assert_eq!(a.caret().0, 15);
}

#[test]
fn lines_are_broken_at_the_width_and_a_click_finds_its_line() {
    let mut a = area("aaaa bbbb cccc", 60.0); // nine characters and a space fit in 60 px
    assert_eq!(a.h.tree.rect(a.field).height, 28.0);
    a.h.click(8.0, 21.0); // on the second line, after its first character
    assert_eq!(a.caret().0, 11);
    a.keys(&[(Key::Home, Mods::NONE)]);
    assert_eq!(a.caret().0, 10); // the start of the line as it is shown
    a.keys(&[(Key::Up, Mods::NONE), (Key::End, Mods::NONE)]);
    assert_eq!(a.caret().0, 9); // its end: before the space the line was broken at
    a.h.type_text("\r\nz"); // a line break that is typed or pasted is one line feed
    a.h.frame();
    assert_eq!(a.notes.get_untracked(), "aaaa bbbb\nz cccc");
    a.h.advance(std::time::Duration::from_secs(2)); // not a third click of the one above
    a.h.click(2.0, 21.0);
    a.h.click(2.0, 21.0); // a double click on the second line: its word
    assert_eq!(a.caret().1, 10..11);
}

#[test]
fn a_scroll_view_keeps_the_caret_of_the_area_in_view() {
    let (notes, scroll_y) = (signal(String::new()), signal(0.0));
    let mut h = Harness::new();
    let view = scroll(scroll_y, div().child(text_area(notes).size(10.0))).container(div().height(50.0));
    mount_root(&mut h.tree, view);
    h.frame();
    h.click(10.0, 5.0);
    for line in 0..10 {
        h.type_text(&format!("line {line}"));
        h.key(Key::Enter, Mods::NONE);
        h.frame();
    }
    // Eleven lines of 14 px in a box of 50: the last one, where the caret is, ends at its end.
    assert_eq!(scroll_y.get_untracked(), 11.0 * 14.0 - 50.0);
    h.key(Key::Home, Mods::CTRL);
    h.frame();
    assert_eq!(scroll_y.get_untracked(), 0.0);
}

#[test]
fn on_one_line_the_keys_of_lines_stay_keys() {
    let (name, seen) = (signal("abc".to_string()), signal(Vec::new()));
    let mut h = Harness::new();
    let around = div().size(120.0, 20.0).on_key_down(move |e| seen.update(|s| s.push(e.key)));
    mount_root(&mut h.tree, around.child(text_input(name).size(10.0)));
    h.frame();
    h.click(10.0, 10.0);
    for key in [Key::Up, Key::Down, Key::Enter, Key::Left] {
        h.key(key, Mods::NONE);
    }
    h.frame();
    // A list around the input can use the arrows, a form Enter; left moved the caret.
    assert_eq!(
        (seen.get_untracked(), name.get_untracked().as_str()),
        (vec![Key::Up, Key::Down, Key::Enter], "abc")
    );
}

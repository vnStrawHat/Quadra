//! The text input (behaviour/09): typed text and editing keys change the value it shows, the
//! pointer places the caret and selects, and the text scrolls so the caret stays in view. On
//! the fake backend at size 10 a character is 6 px wide.

use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{NodeId, PointerInput, TextField};
use quadra_ui::{column, div, mount_root, text_input};

struct Form {
    h: Harness,
    name: Signal<String>,
    other: Signal<String>,
    field: NodeId,
}

/// Two inputs, each in a box 120 px wide and 20 high, at the top left of the window.
fn form(text: &str) -> Form {
    let (name, other) = (signal(text.to_string()), signal(String::new()));
    let boxed = |value| div().size(120.0, 20.0).child(text_input(value).size(10.0));
    let mut h = Harness::new();
    mount_root(&mut h.tree, column().child(boxed(name)).child(boxed(other)));
    h.frame();
    let first = h.tree.children(h.tree.root().unwrap())[0];
    let field = h.tree.children(first)[0];
    Form { h, name, other, field }
}

impl Form {
    fn caret(&mut self) -> (usize, std::ops::Range<usize>) {
        let field = self.h.tree.widget_mut::<TextField>(self.field).unwrap();
        (field.buffer.head(), field.buffer.selection())
    }

    fn keys(&mut self, keys: &[(Key, Mods)]) {
        for &(key, mods) in keys {
            self.h.key(key, mods);
        }
        self.h.frame_if_needed();
    }
}

#[test]
fn typing_and_the_editing_keys_change_the_value() {
    let mut f = form("");
    f.h.click(10.0, 10.0);
    f.h.type_text("hello");
    f.h.type_text(" wörld");
    let (stats, _) = f.h.frame();
    assert_eq!((f.name.get_untracked().as_str(), f.caret().0), ("hello wörld", 12));
    assert!(stats.nodes_painted >= 1);

    f.keys(&[
        (Key::Backspace, Mods::NONE),
        (Key::Left, Mods::NONE),
        (Key::Left, Mods::NONE),
        (Key::Delete, Mods::NONE),
    ]);
    assert_eq!((f.name.get_untracked().as_str(), f.caret().0), ("hello wöl", 9)); // "d", then "r"
    f.keys(&[(Key::Left, Mods::CTRL | Mods::SHIFT)]);
    assert_eq!(f.caret(), (6, 6..9));
    f.h.type_text("W"); // over the selection
    f.keys(&[(Key::Home, Mods::NONE), (Key::Right, Mods::SHIFT)]);
    f.h.type_text("H");
    f.keys(&[]);
    assert_eq!(f.name.get_untracked(), "Hello Wl");
    f.keys(&[(Key::A, Mods::CTRL), (Key::Backspace, Mods::NONE)]);
    assert_eq!(f.name.get_untracked(), "");
    f.keys(&[(Key::Z, Mods::CTRL)]);
    assert_eq!((f.name.get_untracked().as_str(), f.caret().1), ("Hello Wl", 0..8)); // and the selection
    f.keys(&[(Key::Y, Mods::CTRL)]);
    assert_eq!(f.name.get_untracked(), "");
}

#[test]
fn a_value_written_from_elsewhere_is_shown_and_tab_moves_on() {
    let mut f = form("first");
    f.h.click(10.0, 10.0);
    f.name.set("changed".to_string());
    f.h.frame();
    f.h.type_text("!");
    f.keys(&[]);
    assert_eq!(f.name.get_untracked(), "ch!anged"); // the caret stayed where the click put it
    f.keys(&[(Key::Tab, Mods::NONE)]);
    f.h.type_text("next");
    f.keys(&[]);
    assert_eq!((f.name.get_untracked().as_str(), f.other.get_untracked().as_str()), ("ch!anged", "next"));
    f.h.click(300.0, 300.0); // outside: nothing has the focus, typing goes nowhere
    f.h.type_text("lost");
    f.keys(&[]);
    assert_eq!(f.other.get_untracked(), "next");
}

#[test]
fn the_pointer_places_the_caret_and_selects() {
    let mut f = form("one two three");
    f.h.click(26.0, 10.0); // nearest the gap after "one " (24 px)
    f.h.frame();
    assert_eq!(f.caret(), (4, 4..4));
    f.h.pointer(PointerInput::Down, 26.0, 10.0);
    f.h.pointer(PointerInput::Move, 44.0, 10.0); // dragged to after "two" (42 px)
    f.h.pointer(PointerInput::Move, 300.0, 200.0); // and far outside: the end of the text
    f.h.frame();
    assert_eq!(f.caret(), (13, 4..13));
    f.h.pointer(PointerInput::Up, 300.0, 200.0);
    f.h.pointer(PointerInput::Move, 20.0, 10.0); // released: a move selects nothing
    f.h.frame_if_needed();
    assert_eq!(f.caret().1, 4..13);

    f.h.advance(std::time::Duration::from_secs(2));
    f.h.click(50.0, 10.0);
    f.h.click(50.0, 10.0); // a double click: the word under it
    f.h.frame();
    assert_eq!(f.caret().1, 8..13);
    f.h.click(50.0, 10.0); // a third: everything
    f.h.frame();
    assert_eq!(f.caret().1, 0..13);
}

#[test]
fn the_caret_and_the_selection_are_painted_and_long_text_scrolls_with_the_caret() {
    let mut f = form("abc");
    let painted = |f: &mut Form| f.h.tree.scene().layer(f.h.tree.scene().roots()[0]).instances().len();
    assert_eq!(painted(&mut f), 3); // three glyphs, no caret without the focus
    f.h.click(100.0, 10.0);
    f.h.frame();
    assert_eq!(painted(&mut f), 4); // and the caret
    f.keys(&[(Key::Left, Mods::SHIFT)]);
    assert_eq!(painted(&mut f), 5); // and what is behind the selection

    f.h.type_text("0123456789012345678901234567890"); // far wider than the 120 px box
    f.h.frame();
    let field = f.h.tree.widget_mut::<TextField>(f.field).unwrap();
    assert!(field.index_at(quadra_prim::Point::ZERO) > 10, "the start is scrolled out"); // the caret is at the end
    f.keys(&[(Key::Home, Mods::NONE)]);
    let field = f.h.tree.widget_mut::<TextField>(f.field).unwrap();
    assert_eq!(field.index_at(quadra_prim::Point::ZERO), 0); // back at the start
}

#[test]
fn enter_submits_what_the_input_holds_after_what_was_typed() {
    let (name, sent) = (signal(String::new()), signal(Vec::<String>::new()));
    let mut h = Harness::new();
    let input = text_input(name).size(10.0).on_submit(move |text| sent.update(|s| s.push(text.to_string())));
    mount_root(&mut h.tree, div().size(120.0, 20.0).child(input));
    h.frame();
    h.click(10.0, 10.0);
    h.type_text("An");
    h.key(Key::Enter, Mods::NONE); // in the same turn as the typing
    h.frame();
    assert_eq!((name.get_untracked().as_str(), sent.get_untracked()), ("An", vec!["An".to_string()]));
}

//! Copy, cut and paste in a text input go through the clipboard of its context: here one in
//! memory. No test touches the real clipboard.

use std::rc::Rc;

use quadra_platform::{Clipboard, Key, MemoryClipboard, Mods};
use quadra_reactive::{provide, signal};
use quadra_testing::Harness;
use quadra_ui::{div, mount_with, text_input};

#[test]
fn copy_cut_and_paste_go_through_the_clipboard_of_the_context() {
    let clipboard: Rc<dyn Clipboard> = Rc::new(MemoryClipboard::default());
    let (name, provided) = (signal("one two".to_string()), clipboard.clone());
    let mut h = Harness::new();
    mount_with(&mut h.tree, move || {
        provide(provided);
        div().size(200.0, 20.0).child(text_input(name).size(10.0))
    });
    h.frame();
    let press = |h: &mut Harness, key, mods| {
        h.key(key, mods);
        h.frame_if_needed();
    };
    h.click(5.0, 10.0);
    h.click(5.0, 10.0); // a double click: the first word
    press(&mut h, Key::C, Mods::CTRL);
    assert_eq!(clipboard.read_text().as_deref(), Some("one"));
    press(&mut h, Key::End, Mods::NONE);
    press(&mut h, Key::V, Mods::CTRL);
    assert_eq!(name.get_untracked(), "one twoone");
    press(&mut h, Key::Left, Mods::CTRL | Mods::SHIFT);
    press(&mut h, Key::X, Mods::CTRL);
    assert_eq!((name.get_untracked().as_str(), clipboard.read_text().as_deref()), ("one ", Some("twoone")));
    press(&mut h, Key::C, Mods::CTRL); // nothing selected: the clipboard keeps what it has
    assert_eq!(clipboard.read_text().as_deref(), Some("twoone"));
    clipboard.write_text("a\r\nb\nc"); // lines pasted into a field of one line
    press(&mut h, Key::Insert, Mods::SHIFT);
    assert_eq!(name.get_untracked(), "one a b c");
    press(&mut h, Key::Z, Mods::CTRL); // a paste is one step of undo
    assert_eq!(name.get_untracked(), "one ");
}

#[test]
fn an_input_without_a_clipboard_in_its_context_still_edits() {
    let name = signal("text".to_string());
    let mut h = Harness::new();
    mount_with(&mut h.tree, move || div().size(200.0, 20.0).child(text_input(name).size(10.0)));
    h.frame();
    h.click(5.0, 10.0);
    for (key, mods) in
        [(Key::A, Mods::CTRL), (Key::C, Mods::CTRL), (Key::V, Mods::CTRL), (Key::X, Mods::CTRL)]
    {
        h.key(key, mods);
    }
    h.frame();
    assert_eq!(name.get_untracked(), ""); // the cut still removed the selection
}

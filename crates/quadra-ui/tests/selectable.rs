//! Text that can be selected (specs/05): by dragging, a word by a double click, all of it by
//! the key for it, copied to the clipboard of its context (one in memory here), and let go of
//! with the focus. On the fake backend at size 10 a character is 6 px wide.

use std::rc::Rc;
use std::time::Duration;

use quadra_platform::{Clipboard, Key, MemoryClipboard, Mods};
use quadra_reactive::{provide, signal};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Text};
use quadra_ui::{column, div, mount_with, text};

#[test]
fn text_is_selected_by_dragging_a_double_click_and_select_all_and_copied() {
    let clipboard: Rc<dyn Clipboard> = Rc::new(MemoryClipboard::default());
    let (said, provided) = (signal("alpha beta gamma".to_string()), clipboard.clone());
    let mut h = Harness::new();
    mount_with(&mut h.tree, move || {
        provide(provided);
        let words = text(said).size(10.0).selectable();
        column().child(div().width(60.0).child(words)).child(div().size(200.0, 40.0))
    });
    h.frame();
    let copied = |h: &mut Harness| {
        h.key(Key::C, Mods::CTRL);
        h.frame_if_needed();
        clipboard.read_text().unwrap_or_default()
    };

    // The text is broken at 60 px: "alpha beta" above "gamma". A drag over the first word.
    for (input, x) in [(PointerInput::Move, 1.0), (PointerInput::Down, 1.0), (PointerInput::Move, 31.0)] {
        h.pointer(input, x, 7.0);
    }
    h.pointer(PointerInput::Up, 31.0, 7.0);
    assert_eq!(copied(&mut h), "alpha");
    // A drag that goes on to the second line takes the rest of the first with it.
    h.advance(Duration::from_secs(1));
    h.pointer(PointerInput::Down, 37.0, 7.0);
    h.pointer(PointerInput::Move, 13.0, 21.0);
    h.pointer(PointerInput::Up, 13.0, 21.0);
    assert_eq!(copied(&mut h), "beta ga");

    h.advance(Duration::from_secs(1));
    h.click(45.0, 7.0);
    h.click(45.0, 7.0); // a double click: the word under it
    assert_eq!(copied(&mut h), "beta");
    h.key(Key::A, Mods::CTRL);
    assert_eq!(copied(&mut h), "alpha beta gamma");
    let (stats, _) = h.frame();
    assert_eq!(stats.nodes_painted, 0); // copying paints nothing

    // A press elsewhere takes the focus, and the selection with it: nothing is copied.
    clipboard.write_text("");
    h.click(100.0, 50.0);
    assert_eq!(copied(&mut h), "");
    let root = h.tree.root().unwrap();
    let node = h.tree.children(h.tree.children(root)[0])[0];
    assert_eq!(h.tree.widget_mut::<Text>(node).unwrap().selected(), "");

    // New text has nothing selected, whatever was selected in the old one.
    h.click(45.0, 7.0);
    h.key(Key::A, Mods::CTRL);
    said.set("one".to_string());
    h.frame();
    assert_eq!(copied(&mut h), "");
}

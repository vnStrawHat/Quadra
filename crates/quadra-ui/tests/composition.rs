//! Input methods (specs/09, "IME"): what is being composed shows in the field at the caret,
//! underlined and not yet in the value; what is committed is typed. Vietnamese typed with
//! Telex is the case that must work.

use quadra_platform::{Key, Mods};
use quadra_prim::Rect;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::TextField;
use quadra_ui::{column, div, text_input};

#[test]
fn vietnamese_is_composed_at_the_caret_and_committed_into_the_value() {
    let name = signal("Xin  bạn".to_string());
    let mut h = Harness::new();
    h.mount(move || column().child(div().size(200.0, 20.0).child(text_input(name).size(10.0))));
    h.frame();
    let field = h.tree.children(h.tree.children(h.tree.root().unwrap())[0])[0];
    let painted = |h: &mut Harness| h.tree.scene().layer(h.tree.scene().roots()[0]).instances().len();
    assert_eq!(h.tree.text_caret(), None); // nothing that takes text has the focus

    h.click(26.0, 10.0); // between the two spaces: 4 characters, 6 px each
    h.frame();
    let before = painted(&mut h);
    assert_eq!(h.tree.text_caret(), Some(Rect::new(24.0, 0.0, 1.0, 20.0))); // where candidates open

    // Telex "chaof": each key changes what is being composed; nothing is in the value yet.
    for composing in ["c", "ch", "cha", "chao", "chào"] {
        h.compose(composing);
        h.frame();
        assert_eq!(name.get_untracked(), "Xin  bạn");
    }
    // Four glyphs more, and the line under what is being composed.
    assert_eq!(painted(&mut h), before + 5);
    assert_eq!(h.tree.text_caret().map(|c| c.x), Some(48.0)); // the caret follows the composition
    let shows = |h: &mut Harness| h.tree.widget_mut::<TextField>(field).unwrap().preedit.clone();
    assert_eq!(shows(&mut h), Some(("chào".to_string(), 5)));

    h.type_text("chào"); // committed: now it is text like any other
    h.compose("");
    h.frame();
    assert_eq!((name.get_untracked().as_str(), shows(&mut h)), ("Xin chào bạn", None));
    assert_eq!(painted(&mut h), before + 4);
    h.key(Key::Z, Mods::CTRL); // and one step of undo
    h.frame();
    assert_eq!(name.get_untracked(), "Xin  bạn");

    // A composition that is given up leaves nothing; one over a selection shows in its place.
    h.key(Key::A, Mods::CTRL);
    h.compose("tạ");
    h.frame();
    assert_eq!((painted(&mut h), name.get_untracked().as_str()), (2 + 1 + 1, "Xin  bạn"));
    h.compose("");
    h.frame();
    assert_eq!((shows(&mut h), name.get_untracked().as_str()), (None, "Xin  bạn"));
}

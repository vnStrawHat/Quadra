//! The marks of the gutter as the text is edited (specs/13, "Code"): kept by the lines each
//! change was, they are the marks found from the whole text.

use std::rc::Rc;

use quadra_code::{EditorWidget, bind_keys, editor, marks};
use quadra_platform::{Key, Mods};
use quadra_testing::Harness;
use quadra_tree::{Keymap, Role};
use quadra_view::column;

#[test]
fn marks_kept_through_typing_undoing_and_lines_are_those_of_the_whole_text() {
    let base: String = (0..60).map(|i| format!("line {}\n", i % 7)).collect();
    let mut h = Harness::new();
    let mut keys = Keymap::default();
    bind_keys(&mut keys);
    h.tree.set_keymap(Rc::new(keys));
    let (text, against) = (base.clone(), base.clone());
    h.mount(move || column().size(400.0, 140.0).child(editor(text).base(against).font(10.0, 14.0)));
    h.frame();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    let mut seed = 11u64;
    let mut next = |n: u64| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) % n
    };
    let steps: [(Key, Mods); 7] = [
        (Key::Enter, Mods::NONE),
        (Key::Backspace, Mods::NONE),
        (Key::Down, Mods::NONE),
        (Key::Down, Mods::SHIFT),
        (Key::Up, Mods::NONE),
        (Key::Z, Mods::CTRL),
        (Key::D, Mods::CTRL), // the next occurrence: cursors, edits made at once
    ];
    for step in 0..200 {
        match next(3) {
            0 => h.type_text(["x", "line 3", "y\nz"][next(3) as usize]),
            _ => {
                let (key, mods) = steps[next(steps.len() as u64) as usize];
                h.key(key, mods);
            }
        }
        h.frame_if_needed();
        let w = h.tree.widget_mut::<EditorWidget>(id).unwrap();
        assert_eq!(w.marks, marks(&base, &w.doc.text()), "{step}");
    }
}

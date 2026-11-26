//! A select's value (specs/13, "More of the fields"): one line however narrow the field, cut
//! with an ellipsis, never spilling over what stands below it.

use std::rc::Rc;

use quadra_design::{dark, select};
use quadra_reactive::signal;
use quadra_testing::Harness;

const WORDS: &str = "No keys: add one in Settings";

#[test]
fn a_select_narrower_than_its_value_keeps_it_on_one_line() {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let options = vec!["~/.ssh/id_ed25519_work_laptop".to_string()];
        quadra_design::root().child(select(options, signal(usize::MAX)).placeholder(WORDS).width(120.0))
    });
    h.frame();
    // The field is named by its words; they are the text in its first box.
    let field = h.get_by_text(WORDS).node();
    let holder = h.tree.children(field)[0];
    let node = h.tree.children(holder)[0];
    let (words, face) = (h.tree.window_rect(node), h.tree.window_rect(field));
    assert!(words.height <= face.height && words.bottom() <= face.bottom(), "{words:?} in {face:?}");
    assert!(words.right() <= face.right(), "{words:?} in {face:?}");
}

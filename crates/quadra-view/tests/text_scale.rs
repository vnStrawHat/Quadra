//! The text scale (specs/12, "Text size"): every text, with a size of its own or not, a run
//! of rich text that says its size too, is shaped so many times larger; a box at least as
//! high as a line grows with it, and one of a fixed size does not.

use quadra_prim::Size;
use quadra_testing::Harness;
use quadra_view::{column, div, rich_text, span, text};

#[test]
fn every_text_is_so_many_times_larger_and_what_holds_it_grows() {
    let mut h = Harness::new();
    h.mount(|| {
        let words = rich_text([span("ab "), span("cd").size(20.0)]);
        let field = div().min_height(20.0).child(text("field"));
        let icon = div().size(16.0, 16.0);
        column()
            .text_size(10.0)
            .child(text("plain"))
            .child(text("note").size(20.0))
            .child(words)
            .child(field)
            .child(icon)
    });
    h.frame();
    let root = h.tree.root().unwrap();
    let [plain, note, words, field, icon] = [0, 1, 2, 3, 4].map(|i| h.tree.children(root)[i]);
    let look = |h: &Harness| {
        let sizes = [plain, note].map(|id| h.tree.inherited(id).style.size);
        let heights = [plain, note, words, field].map(|id| h.tree.rect(id).height);
        (sizes, heights, h.tree.rect(icon).size())
    };
    // On the fake backend a line is 1.4 of the size high, in whole pixels; the run of 20 px
    // makes its line higher.
    assert_eq!(look(&h), ([10.0, 20.0], [14.0, 28.0, 28.0, 20.0], Size::new(16.0, 16.0)));

    h.tree.set_text_scale(1.5);
    h.frame();
    assert_eq!(look(&h), ([15.0, 30.0], [21.0, 42.0, 42.0, 21.0], Size::new(16.0, 16.0)));
    h.tree.set_text_scale(1.0);
    h.frame();
    assert_eq!(look(&h), ([10.0, 20.0], [14.0, 28.0, 28.0, 20.0], Size::new(16.0, 16.0)));
}

//! The editor wraps while a signal says so (specs/13, "Code"): an app's switch changes the
//! rows of an editor that shows, and its text stays what it was.

use quadra_code::{EditorWidget, editor};
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::column;

#[test]
fn wrapping_by_a_signal_changes_the_rows_of_an_editor_that_shows() {
    let (wraps, text) = (quadra_reactive::signal(false), format!("{}\nshort\n", "wide ".repeat(60)));
    let mut h = Harness::new();
    let wide = text.clone();
    h.mount(move || column().size(400.0, 140.0).child(editor(wide).wrap_when(wraps).font(10.0, 14.0)));
    h.frame();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    let rows = |h: &mut Harness| h.tree.widget_mut::<EditorWidget>(id).unwrap().row_count();
    assert_eq!(rows(&mut h), 3);
    wraps.set(true);
    h.frame();
    assert!(rows(&mut h) > 3, "the wide line takes rows");
    // The text is the same, and nothing scrolls sideways while it wraps.
    let w = h.tree.widget_mut::<EditorWidget>(id).unwrap();
    assert_eq!((w.doc.buffer.text(), w.scroll_x), (text, 0.0));
    wraps.set(false);
    h.frame();
    assert_eq!(rows(&mut h), 3);
}

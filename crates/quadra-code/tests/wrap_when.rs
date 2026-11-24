//! The editor wraps while a signal says so (specs/13, "Code"): an app's switch changes the
//! rows of an editor that shows, and its text stays what it was.

use quadra_code::{EditorWidget, editor};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role};
use quadra_view::column;

#[test]
fn wrapping_by_a_signal_changes_the_rows_of_an_editor_that_shows() {
    let (wraps, text) = (quadra_reactive::signal(false), format!("{}\nshort\n", "wide ".repeat(60)));
    let mut h = Harness::new();
    let wide = text.clone();
    h.mount(move || column().size(400.0, 70.0).child(editor(wide).wrap_when(wraps).font(10.0, 14.0)));
    h.frame();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    let rows = |h: &mut Harness| h.tree.widget_mut::<EditorWidget>(id).unwrap().row_count();
    assert_eq!(rows(&mut h), 3);
    wraps.set(true);
    h.frame();
    assert!(rows(&mut h) > 5, "the wide line takes rows");
    // Laid out again: as high as its rows, more than the view; and as the view without them.
    assert_eq!(h.tree.rect(id).height, rows(&mut h) as f32 * 14.0);
    // The text is the same, and nothing scrolls sideways while it wraps.
    let w = h.tree.widget_mut::<EditorWidget>(id).unwrap();
    assert_eq!((w.doc.buffer.text(), w.scroll_x), (text, 0.0));
    wraps.set(false);
    h.frame();
    assert_eq!((rows(&mut h), h.tree.rect(id).height), (3, 70.0));
}

#[test]
fn a_line_wider_than_the_editor_has_a_bar_along_the_bottom_that_moves_it() {
    let (wraps, text) = (quadra_reactive::signal(false), format!("{}\nshort\n", "wide ".repeat(60)));
    let mut h = Harness::new();
    h.mount(move || column().size(400.0, 70.0).child(editor(text).wrap_when(wraps).font(10.0, 14.0)));
    h.frame();
    // The pointer over the editor: the bar learns how far the lines painted go.
    h.pointer(PointerInput::Move, 200.0, 30.0);
    h.frame();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    let at = |h: &mut Harness| h.tree.widget_mut::<EditorWidget>(id).unwrap().scroll_x;
    // The thumb, in the 14 px along the bottom, dragged to the right.
    let drag = |h: &mut Harness, from: f32, to: f32| {
        for (what, x) in [(PointerInput::Move, from), (PointerInput::Down, from), (PointerInput::Move, to)] {
            h.pointer(what, x, 63.0);
        }
        h.pointer(PointerInput::Up, to, 63.0);
        h.frame();
    };
    drag(&mut h, 20.0, 120.0);
    let moved = at(&mut h);
    assert!(moved > 100.0, "the text went left by more than the thumb: {moved}");
    // The wheel moves the thumb with the text: it is held where the text is, past the middle.
    let (by, over) = (quadra_prim::Point::new(-300.0, 0.0), quadra_prim::Point::new(200.0, 30.0));
    h.tree.wheel(by, true, over, quadra_platform::Mods::NONE);
    h.frame();
    let wheeled = at(&mut h);
    assert!(wheeled > moved);
    drag(&mut h, 200.0, 150.0);
    assert!(at(&mut h) < wheeled && at(&mut h) > moved, "{} of {wheeled}", at(&mut h));
    // Wrapped, nothing is wider than the editor: no thumb to drag.
    wraps.set(true);
    h.frame();
    drag(&mut h, 20.0, 120.0);
    assert_eq!(at(&mut h), 0.0);
}

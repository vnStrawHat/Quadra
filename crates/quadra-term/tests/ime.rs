//! What an input method composes (specs/18, "Input"): drawn at the cursor, underlined (the
//! clause thicker), a wide character on two cells; nothing goes to the program until it is
//! committed, and an empty composition or the focus leaving ends it. Cells are 6 x 14 px from
//! the box's corner and its padding of 8 px.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_render::{Kind, Step};
use quadra_term::{Session, TerminalWidget, echo, terminal};
use quadra_testing::Harness;
use quadra_tree::{NodeId, Role};
use quadra_view::column;

/// An `echo()` terminal of 400 x 140 at size 10, focused, `ab` already typed.
fn term() -> (Harness, NodeId, Session) {
    let mut h = Harness::new();
    let slot = Rc::new(RefCell::new(None));
    let out = slot.clone();
    h.mount(move || {
        let s = echo();
        *out.borrow_mut() = Some(s.clone());
        column().size(400.0, 140.0).child(terminal(&s).font(10.0, 14.0))
    });
    h.settle();
    let id = h.query(|n| n.semantics.role == Role::TextInput)[0];
    h.tree.focus(id);
    h.type_text("ab");
    h.settle();
    let s = slot.borrow_mut().take().unwrap();
    (h, id, s)
}

/// The glyphs drawn (their left, from the left) and the quads of the first row's underline:
/// their left, width and height.
fn drawn(h: &mut Harness) -> (Vec<f32>, Vec<(f32, f32, f32)>) {
    h.frame();
    let scene = h.tree.scene();
    let layers = h.renderer.steps().iter().filter_map(|step| match step {
        Step::Layer { id, .. } => Some(scene.layer(*id).instances()),
        _ => None,
    });
    let (mut glyphs, mut lines) = (Vec::new(), Vec::new());
    for i in layers.flatten() {
        let [x, y, width, height] = i.bounds;
        if i.kind == Kind::Mono as u32 {
            glyphs.push(x);
        } else if i.kind == Kind::Quad as u32 && y == 8.0 + 14.0 - 2.0 {
            lines.push((x, width, height));
        }
    }
    glyphs.sort_by(f32::total_cmp);
    (glyphs, lines)
}

fn tiles(h: &mut Harness, id: NodeId) -> usize {
    h.frame();
    h.tree.widget_mut::<TerminalWidget>(id).unwrap().tiles_painted()
}

#[test]
fn a_composition_is_drawn_at_the_cursor_and_nothing_reaches_the_program() {
    let (mut h, id, s) = term();
    assert_eq!(tiles(&mut h, id), 2);
    // Two wide characters, the second the clause being converted.
    let text = "\u{304b}\u{306a}";
    h.compose_clause(text, 3..6);
    h.settle();
    assert_eq!(s.screen_text()[0], "ab", "nothing was sent");
    let (glyphs, lines) = drawn(&mut h);
    assert_eq!(glyphs.len(), 4);
    let cell = |col: f32| 8.0 + col * 6.0;
    assert!(glyphs[2] >= cell(2.0) && glyphs[2] < cell(4.0), "{glyphs:?}");
    assert!(glyphs[3] >= cell(4.0) && glyphs[3] < cell(6.0), "{glyphs:?}");
    assert_eq!(lines, [(cell(2.0), 12.0, 1.0), (cell(4.0), 12.0, 2.0)], "two cells each");
    // An empty composition ends it; what is committed then arrives once.
    h.compose("");
    h.settle();
    assert_eq!(tiles(&mut h, id), 2);
    assert!(drawn(&mut h).1.is_empty());
    h.compose(text);
    h.type_text(text);
    h.settle();
    assert_eq!(s.screen_text()[0], format!("ab{text}"));
    assert_eq!(tiles(&mut h, id), 4, "the text, not the composition over it");
}

#[test]
fn the_focus_leaving_ends_a_composition() {
    let (mut h, id, s) = term();
    h.compose("xy");
    h.settle();
    assert_eq!(tiles(&mut h, id), 4);
    h.tree.blur();
    h.settle();
    assert_eq!(tiles(&mut h, id), 2);
    h.tree.focus(id);
    h.settle();
    assert_eq!(tiles(&mut h, id), 2, "not back with the focus");
    assert_eq!(s.screen_text()[0], "ab");
}

//! One selection over the blocks of a document (specs/13, "Markdown"): a drag from a text to
//! another selects both ends and what is between; Ctrl-C copies it to the harness's
//! clipboard, Ctrl-A selects all, and leaving the document lets it go.

use std::rc::Rc;

use quadra_design::dark;
use quadra_md::markdown;
use quadra_platform::{Key, Mods, MouseButton};
use quadra_prim::Point;
use quadra_testing::Harness;
use quadra_tree::PointerInput;

const DOC: &str = "# Title\n\nThe first paragraph.\n\nThe second one.\n";

#[test]
fn a_drag_selects_across_blocks_and_copying_gives_their_text() {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(|| quadra_design::root().child(markdown(DOC)));
    h.frame();
    let title = h.get_by_role(quadra_tree::Role::Heading, "Title").node();
    let second = h.get_by_text("The second one.").node();
    let (title, second) = (h.tree.window_rect(title), h.tree.window_rect(second));
    // From before "Title" to the middle of the last paragraph.
    let (from, to) = (Point::new(title.x + 1.0, title.y + 4.0), Point::new(second.x + 40.0, second.y + 4.0));
    h.tree.pointer(PointerInput::Move, MouseButton::None, from, Mods::NONE, h.now());
    h.tree.pointer(PointerInput::Down, MouseButton::Left, from, Mods::NONE, h.now());
    h.tree.pointer(PointerInput::Move, MouseButton::None, to, Mods::NONE, h.now());
    h.tree.pointer(PointerInput::Up, MouseButton::Left, to, Mods::NONE, h.now());
    h.frame();
    // Each of the three texts is filled where it is selected, in the colour of a selection.
    let fill = dark().color(quadra_design::token::SELECTION).to_array();
    let filled = |h: &Harness| {
        let layers = h.renderer.steps().iter().filter_map(|step| match step {
            quadra_render::Step::Layer { id, .. } => Some(h.tree.scene().layer(*id)),
            quadra_render::Step::Run { .. } => None,
        });
        layers.flat_map(|layer| layer.instances().iter()).filter(|i| i.color == fill).count()
    };
    assert_eq!(filled(&h), 3);
    h.key(Key::C, Mods::CTRL);
    h.frame();
    let copied = h.clipboard.read_text().unwrap_or_default();
    assert!(copied.starts_with("Title\nThe first paragraph.\nThe "), "{copied:?}");
    assert!(!copied.contains("one."), "{copied:?}");

    // All of it.
    h.key(Key::A, Mods::CTRL);
    h.key(Key::C, Mods::CTRL);
    h.frame();
    assert_eq!(h.clipboard.read_text().as_deref(), Some("Title\nThe first paragraph.\nThe second one."));
}

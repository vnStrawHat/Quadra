//! The tooltip and the dialog (specs/design): a tooltip shows after the pointer has
//! rested half a second and never takes the focus; a dialog hangs under the top of the
//! window, takes the focus, and is given up by Escape or answered by one of its buttons.

use std::rc::Rc;
use std::time::Duration;

use quadra_design::{Variant, button, dark, dialog, tooltip};
use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role};
use quadra_view::{column, text};

/// The box of the open tooltip's arrow (12 px square, the last child of its bubble), and which
/// of its four pairs shows: above, below, left, right.
fn arrow_of(h: &Harness) -> (quadra_prim::Rect, usize) {
    let bubble = h.tree.overlays().next().expect("a tooltip");
    let arrow = *h.tree.children(bubble).last().expect("its arrow");
    let layers = h.tree.children(arrow).to_vec();
    let shown = (0..4).find(|k| h.tree.rect(layers[k * 2]).width > 0.0).expect("one pair shows");
    (h.tree.window_rect(arrow), shown)
}

struct Page {
    h: Harness,
    asking: Signal<bool>,
    deleted: Signal<u32>,
}

/// A button "Save" with a tooltip, and a button that asks before it deletes.
fn page() -> Page {
    let (asking, deleted) = (signal(false), signal(0));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let save = tooltip("Save the file", button("Save").test_id("save")).keys("Ctrl-S");
        let delete = button("Delete file").test_id("delete").on_click(move || asking.set(true));
        let ask = dialog(asking, "Delete the file?", || text("It cannot be brought back."))
            .action("Cancel", Variant::Ghost, || {})
            .action("Delete", Variant::Danger, move || deleted.update(|n| *n += 1));
        quadra_design::root().child(column().gap(8.0).child(save).child(delete).child(ask))
    });
    h.frame();
    Page { h, asking, deleted }
}

impl Page {
    fn settle(&mut self) {
        self.h.frame();
        while self.h.frame_if_needed().is_some() {}
    }

    fn wait(&mut self, ms: u64) {
        self.h.advance(Duration::from_millis(ms));
        self.settle();
    }
}

#[test]
fn a_tooltip_shows_after_the_pointer_has_rested_and_goes_when_it_leaves() {
    let mut p = page();
    let save = p.h.get_by_test_id("save").node();
    // To a reader its words are the description of what it is about, shown or not.
    let said =
        p.h.tree
            .semantic_tree()
            .nodes
            .iter()
            .find(|n| n.semantics.test_id.as_deref() == Some("save"))
            .map(|n| n.semantics.description.clone());
    assert_eq!(said, Some(Some("Save the file".to_string())));
    let at = p.h.tree.window_rect(save).center();
    p.h.pointer(PointerInput::Move, at.x, at.y);
    p.wait(400);
    assert!(!p.h.semantics().contains("Tooltip"));
    p.wait(100);
    let tree = p.h.semantics();
    assert!(tree.contains("Tooltip") && p.h.has_text("Save the file") && p.h.has_text("Ctrl-S"), "{tree}");
    // Below the button, which has no room above it, and centred on it; the focus is where it was:
    // nowhere.
    let (button, tip) =
        (p.h.tree.window_rect(save), p.h.tree.window_rect(p.h.tree.overlays().next().unwrap()));
    assert!(tip.y >= button.bottom() + 4.0 && (tip.center().x - button.center().x).abs() < 1.0, "{tip:?}");
    assert_eq!(p.h.tree.focused(), None);
    // Its arrow points up at the button's middle, its base on the bubble's top border, and of
    // the four the one pointing up shows.
    let (arrow, shown) = arrow_of(&p.h);
    assert!((arrow.center().x - button.center().x).abs() <= 1.0, "{arrow:?} {button:?}");
    assert_eq!((arrow.bottom(), shown), (tip.y + 1.0, 1), "{arrow:?} {tip:?}");
    // The arrow's box reaches over the button's edge: the pointer there is still the button's.
    assert!(arrow.y < button.bottom() - 2.0, "{arrow:?} {button:?}");
    p.h.pointer(PointerInput::Move, at.x, button.bottom() - 2.0);
    p.wait(100);
    assert!(p.h.semantics().contains("Tooltip"));

    p.h.pointer(PointerInput::Move, 600.0, 500.0); // away: it goes
    p.settle();
    assert!(!p.h.semantics().contains("Tooltip"));
    // Back, and gone again before the half second is over: no tooltip follows.
    p.h.pointer(PointerInput::Move, at.x, at.y);
    p.wait(300);
    p.h.pointer(PointerInput::Move, 600.0, 500.0);
    p.wait(1000);
    assert!(!p.h.semantics().contains("Tooltip"));
    // A press takes it away too.
    p.h.pointer(PointerInput::Move, at.x, at.y);
    p.wait(600);
    assert!(p.h.semantics().contains("Tooltip"));
    p.h.click(at.x, at.y);
    p.settle();
    assert!(!p.h.semantics().contains("Tooltip"));
}

#[test]
fn a_dialog_is_answered_by_its_buttons_or_given_up() {
    let mut p = page();
    let delete = p.h.get_by_test_id("delete").node();
    p.h.tree.focus(delete);
    p.h.key(Key::Enter, Mods::NONE);
    p.settle();
    let tree = p.h.semantics();
    assert!(
        tree.contains("Dialog \"Delete the file?\"") && p.h.has_text("It cannot be brought back."),
        "{tree}"
    );
    // 440 wide with its border, centred, 80 px under the top of the window.
    let panel = p.h.tree.window_rect(p.h.tree.overlays().next().unwrap());
    assert_eq!((panel.y, panel.width, panel.center().x), (80.0, 440.0, 400.0));
    // The focus is on its first button. Escape gives the dialog up, and nothing is deleted.
    assert_eq!(p.h.tree.focused(), Some(p.h.get_by_role(Role::Button, "Cancel").node()));
    p.h.key(Key::Escape, Mods::NONE);
    p.settle();
    assert_eq!(
        (p.asking.get_untracked(), p.deleted.get_untracked(), p.h.tree.focused()),
        (false, 0, Some(delete))
    );

    // Asked again, and answered: the button does what it says, and the dialog closes.
    p.h.key(Key::Enter, Mods::NONE);
    p.settle();
    p.h.get_by_role(Role::Button, "Delete").click();
    p.settle();
    assert_eq!((p.asking.get_untracked(), p.deleted.get_untracked()), (false, 1));
    assert!(!p.h.semantics().contains("Dialog"));
}

#[test]
fn a_tooltip_follows_words_that_change() {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    let name = signal("one".to_string());
    h.mount(move || quadra_design::root().child(tooltip(name, button("Tab").test_id("tab"))));
    h.frame();
    let told = |h: &Harness| {
        let nodes = h.tree.semantic_tree().nodes;
        nodes
            .iter()
            .find(|n| n.semantics.test_id.as_deref() == Some("tab"))
            .and_then(|n| n.semantics.description.clone())
    };
    assert_eq!(told(&h).as_deref(), Some("one"));
    name.set("two".into());
    h.frame();
    while h.frame_if_needed().is_some() {}
    assert_eq!(told(&h).as_deref(), Some("two"));
}

#[test]
fn a_tooltip_at_the_right_points_left_at_the_middle_of_its_view() {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        let tab = tooltip("Pages", button("P").test_id("p")).side(quadra_tree::Side::Right);
        quadra_design::root().child(column().padding_top(200.0).child(tab))
    });
    h.frame();
    let at = h.get_by_test_id("p").node();
    let view = h.tree.window_rect(at);
    h.pointer(PointerInput::Move, view.center().x, view.center().y);
    h.advance(Duration::from_millis(600));
    while h.frame_if_needed().is_some() {}
    let tip = h.tree.window_rect(h.tree.overlays().next().expect("a tooltip"));
    let (arrow, shown) = arrow_of(&h);
    assert_eq!(shown, 3, "the pair of a bubble at the right");
    assert_eq!(arrow.right(), tip.x + 1.0, "{arrow:?} {tip:?}");
    assert!((arrow.center().y - view.center().y).abs() <= 1.0, "{arrow:?} {view:?}");
}

#[test]
fn an_action_is_off_while_it_says_so_and_a_confirm_can_destroy() {
    let (open, sure, name, added, deleted) =
        (signal(true), signal(false), signal(String::new()), signal(0), signal(0));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let add = dialog(open, "Add a cluster", || text("A name first"))
            .action("Cancel", Variant::Ghost, || {})
            .action("Add", Variant::Primary, move || added.update(|n| *n += 1))
            .disabled(move || name.with(String::is_empty));
        let ask = quadra_design::confirm(sure, "Delete pod?", "It goes.", "Delete", move || deleted.set(1))
            .danger();
        quadra_design::root().child(add).child(ask)
    });
    h.frame();
    // Off: a press does nothing and the keys do not reach it.
    let tree = h.semantics();
    assert!(tree.contains("Button \"Add\" [disabled]"), "{tree}");
    let add = h.get_by_role(Role::Button, "Add").node();
    let at = h.tree.window_rect(add).center();
    h.click(at.x, at.y);
    h.frame();
    assert!(open.get_untracked() && added.get_untracked() == 0);
    h.key(Key::Tab, Mods::NONE);
    assert_ne!(h.tree.focused(), Some(add));
    // On once there is a name: it does what it says.
    name.set("prod".into());
    h.frame();
    h.click(at.x, at.y);
    h.frame();
    assert!(!open.get_untracked() && added.get_untracked() == 1);

    // A confirm that destroys still answers as one does.
    sure.set(true);
    h.frame();
    h.get_by_role(Role::Button, "Delete").click();
    h.frame();
    assert!(!sure.get_untracked() && deleted.get_untracked() == 1);
}

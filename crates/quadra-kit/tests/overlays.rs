//! The tooltip and the dialog (specs/behaviour/15): a tooltip shows after the pointer has
//! rested half a second and never takes the focus; a dialog hangs under the top of the
//! window, takes the focus, and is given up by Escape or answered by one of its buttons.

use std::rc::Rc;
use std::time::Duration;

use quadra_kit::{Variant, button, dark, dialog, tooltip};
use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role};
use quadra_ui::{column, text};

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
    h.tree.set_keymap(Rc::new(quadra_kit::keymap()));
    h.mount(move || {
        let save = tooltip("Save the file", button("Save").test_id("save")).keys("Ctrl-S");
        let delete = button("Delete file").test_id("delete").on_click(move || asking.set(true));
        let ask = dialog(asking, "Delete the file?", || text("It cannot be brought back."))
            .action("Cancel", Variant::Ghost, || {})
            .action("Delete", Variant::Danger, move || deleted.update(|n| *n += 1));
        quadra_kit::root().child(column().gap(8.0).child(save).child(delete).child(ask))
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
    let at = p.h.tree.window_rect(save).center();
    p.h.pointer(PointerInput::Move, at.x, at.y);
    p.wait(400);
    assert!(!p.h.semantics().contains("Tooltip"));
    p.wait(100);
    let tree = p.h.semantics();
    assert!(tree.contains("Tooltip") && p.h.has_text("Save the file") && p.h.has_text("Ctrl-S"), "{tree}");
    // Below the button, centred on it; the focus is where it was: nowhere.
    let (button, tip) =
        (p.h.tree.window_rect(save), p.h.tree.window_rect(p.h.tree.overlays().next().unwrap()));
    assert!(tip.y >= button.bottom() + 4.0 && (tip.center().x - button.center().x).abs() < 1.0, "{tip:?}");
    assert_eq!(p.h.tree.focused(), None);

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
    // 440 wide (and its border), centred, 80 px under the top of the window.
    let panel = p.h.tree.window_rect(p.h.tree.overlays().next().unwrap());
    assert_eq!((panel.y, panel.width, panel.center().x), (80.0, 442.0, 400.0));
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

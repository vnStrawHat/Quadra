//! Notifications and toasts (specs/behaviour/15, "Notifications"): cards stacked at the
//! bottom right that stay until closed or answered, and lines at the bottom centre that go
//! by themselves.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use quadra_design::{Notice, Notices, Tone, dark, notices};
use quadra_prim::Rect;
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_ui::{div, text};

fn page() -> (Harness, Notices) {
    let told: Rc<Cell<Option<Notices>>> = Rc::default();
    let tell = told.clone();
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || {
        // Above a status bar of 30 px.
        let all = notices().above(30.0);
        tell.set(Some(all));
        quadra_design::root().child(div().grow(1.0).child(text("content"))).child(all.host())
    });
    h.frame();
    (h, told.take().unwrap())
}

fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {}
}

fn card(h: &mut Harness, name: &str) -> Rect {
    let node = h.get_by_role(Role::Group, name).node();
    h.tree.window_rect(node)
}

#[test]
fn cards_stack_at_the_bottom_right_until_closed_or_answered() {
    let (mut h, all) = page();
    let content = h.get_by_text("content").node();
    // The host takes no room, and with nothing to show there is nothing above the window.
    assert_eq!((h.tree.window_rect(content).height, h.tree.overlays().count()), (600.0, 0));

    let retried = Rc::new(Cell::new(0));
    let again = retried.clone();
    all.notify(Notice::new("Saved").body("All changes are on disk.").tone(Tone::Success));
    let failed = Notice::new("Upload failed").tone(Tone::Danger).action("Dismiss", || {});
    all.notify(failed.primary("Retry", move || again.set(again.get() + 1)));
    settle(&mut h);
    let tree = h.semantics();
    for line in
        ["Group \"Saved\"", "Text \"All changes are on disk.\"", "Button \"Retry\"", "Button \"Close\""]
    {
        assert!(tree.contains(line), "{line} is not in:\n{tree}");
    }
    // 448 px wide, 12 from the right edge and from the status bar; the newest
    // is the lowest, 8 under the one before.
    let (saved, failed) = (card(&mut h, "Saved"), card(&mut h, "Upload failed"));
    assert_eq!((saved.width, saved.right(), failed.bottom()), (448.0, 788.0, 600.0 - 30.0 - 12.0));
    assert_eq!(failed.y - saved.bottom(), 8.0);

    // Its button does what it says and closes the card; the other closes by its own.
    h.get_by_role(Role::Button, "Retry").click();
    settle(&mut h);
    assert_eq!((retried.get(), h.semantics().contains("Upload failed")), (1, false));
    assert_eq!(card(&mut h, "Saved").bottom(), 558.0); // it takes the place at the bottom
    h.get_by_role(Role::Button, "Close").click();
    settle(&mut h);
    assert_eq!(h.tree.overlays().count(), 0);
}

#[test]
fn a_toast_and_a_card_that_lasts_go_by_themselves() {
    let (mut h, all) = page();
    all.toast(Tone::Info, "Copied");
    all.notify(Notice::new("Synced").lasts(Duration::from_secs(3)));
    let kept = all.notify(Notice::new("Kept"));
    settle(&mut h);
    // The toast is at the bottom centre; the cards in their corner.
    let toast = card(&mut h, "Copied");
    assert_eq!(((toast.center().x - 400.0).abs() < 1.0, toast.bottom()), (true, 558.0));
    h.advance(Duration::from_secs(3));
    settle(&mut h);
    let tree = h.semantics();
    assert!(!tree.contains("Synced") && tree.contains("Copied") && tree.contains("Kept"), "{tree}");
    h.advance(Duration::from_secs(7));
    settle(&mut h);
    let tree = h.semantics();
    assert!(!tree.contains("Copied") && tree.contains("Kept"), "{tree}");
    all.close(kept);
    settle(&mut h);
    assert_eq!(h.tree.overlays().count(), 0);
}

#[test]
fn a_toast_waits_while_the_pointer_is_on_it_and_lasts_again_after() {
    use quadra_tree::PointerInput;
    let (mut h, all) = page();
    all.toast(Tone::Info, "Saved");
    settle(&mut h);
    let at = card(&mut h, "Saved").center();
    h.pointer(PointerInput::Move, at.x, at.y);
    h.advance(Duration::from_secs(30));
    settle(&mut h);
    assert!(h.has_text("Saved"), "held while the pointer is on it");
    h.pointer(PointerInput::Move, 5.0, 5.0);
    h.advance(Duration::from_secs(9));
    settle(&mut h);
    assert!(h.has_text("Saved"), "as long again from when it left");
    h.advance(Duration::from_secs(2));
    settle(&mut h);
    assert!(!h.has_text("Saved"));
}

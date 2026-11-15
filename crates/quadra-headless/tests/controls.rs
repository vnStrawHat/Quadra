//! The headless crate's controls with bare parts (specs/17, "Tests"): a button, a checkbox and a
//! switch, a list, a section that opens, a split.

use std::rc::Rc;

use quadra_headless::{disclose, list, press, split};
use quadra_platform::{Key, Mods};
use quadra_reactive::{memo, signal};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role, SemanticAction};
use quadra_ui::{View, column, div, row, text};

fn harness() -> Harness {
    let mut h = Harness::new();
    h.tree.set_keymap(Rc::new(quadra_headless::keymap()));
    h
}

fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {}
}

fn drag(h: &mut Harness, (x, y): (f32, f32), to: f32) {
    h.pointer(PointerInput::Move, x, y);
    h.pointer(PointerInput::Down, x, y);
    h.pointer(PointerInput::Move, to, y);
    h.pointer(PointerInput::Up, to, y);
}

fn mounted<V: View + 'static>(view: impl Fn() -> V + 'static) -> Harness {
    let mut h = harness();
    h.mount(view);
    settle(&mut h);
    h
}

#[test]
fn a_button_does_its_thing_by_the_pointer_and_the_keys_but_not_while_off_or_busy() {
    let (count, off, busy, on) = (signal(0), signal(false), signal(false), signal(false));
    let mut h = mounted(move || {
        let face = row().size(60.0, 20.0).child(text("Save"));
        let save = press::press(face).disabled(move || off.get()).busy(memo(move || busy.get()));
        let save = save.on_click(move || count.update(|n| *n += 1));
        let bold = press::press(div().size(20.0, 20.0).label("Bold")).toggled(move || on.get());
        column().child(save).child(bold)
    });
    assert!(h.semantics().contains("Button \"Bold\" [unchecked]"), "{}", h.semantics());
    h.get_by_role(Role::Button, "Save").click();
    let save = h.get_by_role(Role::Button, "Save").node();
    h.tree.focus(save);
    h.key(Key::Enter, Mods::NONE);
    h.key(Key::Space, Mods::NONE);
    assert_eq!(count.get_untracked(), 3);
    busy.set(true);
    h.get_by_role(Role::Button, "Save").click();
    assert_eq!(count.get_untracked(), 3, "busy");
    busy.set(false);
    off.set(true);
    settle(&mut h);
    assert!(h.semantics().contains("Button \"Save\" [disabled]"), "{}", h.semantics());
    h.get_by_role(Role::Button, "Save").click();
    assert_eq!(count.get_untracked(), 3, "disabled");
}

#[test]
fn a_checkbox_and_a_switch_are_turned_by_their_face_the_keys_and_a_reader() {
    let (wrap, live) = (signal(false), signal(true));
    let mut h = mounted(move || {
        let wrap = press::toggle(wrap, row().size(80.0, 20.0).child(text("Wrap")), false, false);
        let live = press::toggle(live, row().size(80.0, 20.0).child(text("Live")), true, false);
        column().child(wrap).child(live)
    });
    let tree = h.semantics();
    assert!(
        tree.contains("Checkbox \"Wrap\" [unchecked]") && tree.contains("Switch \"Live\" [checked]"),
        "{tree}"
    );
    h.get_by_role(Role::Checkbox, "Wrap").click();
    assert!(wrap.get_untracked());
    let switch = h.get_by_role(Role::Switch, "Live").node();
    h.tree.focus(switch);
    h.key(Key::Space, Mods::NONE);
    assert!(!live.get_untracked());
    h.get_by_role(Role::Switch, "Live").act(SemanticAction::Click);
    assert!(live.get_untracked());
}

#[test]
fn a_list_selects_by_a_click_and_the_arrows_and_activates_by_enter() {
    let (selected, activated) = (signal(None), signal(None));
    let mut h = mounted(move || {
        let row = |index: quadra_reactive::Signal<usize>| {
            div().height(20.0).child(text(move || format!("Row {}", index.get())))
        };
        let rows = list::list(|| 50, selected, 20.0, row).frame(column().height(100.0));
        rows.on_activate(move |at| activated.set(Some(at))).empty(|| text("Nothing"))
    });
    assert!(h.semantics().contains("ListItem \"Row 0\""), "{}", h.semantics());
    h.get_by_role(Role::ListItem, "Row 2").click();
    assert_eq!(selected.get_untracked(), Some(2));
    let frame = h.get_by_role(Role::List, "").node();
    h.tree.focus(frame);
    h.key(Key::Up, Mods::NONE);
    h.key(Key::End, Mods::NONE);
    assert_eq!(selected.get_untracked(), Some(49));
    h.key(Key::Down, Mods::NONE); // it stops at the end
    h.key(Key::Enter, Mods::NONE);
    settle(&mut h);
    assert_eq!(activated.get_untracked(), Some(49));
    assert!(h.semantics().contains("ListItem \"Row 49\" [selected]"), "the selected row is in view");
}

#[test]
fn a_section_opens_and_closes_by_its_head() {
    let open = signal(false);
    let mut h = mounted(move || {
        let head = disclose::head(row().size(80.0, 20.0).child(text("More")), open);
        column().child(head).child(disclose::body(open, || text("Inside")))
    });
    assert!(h.semantics().contains("Button \"More\" [collapsed]") && !h.semantics().contains("Inside"));
    h.get_by_role(Role::Button, "More").click();
    settle(&mut h);
    assert!(h.semantics().contains("[expanded]") && h.semantics().contains("Inside"), "{}", h.semantics());
    let head = h.get_by_role(Role::Button, "More").node();
    h.tree.focus(head);
    h.key(Key::Enter, Mods::NONE);
    settle(&mut h);
    assert!(!open.get_untracked() && !h.semantics().contains("Inside"));
}

#[test]
fn a_split_follows_its_seam_and_keeps_the_least_for_each_pane() {
    let share = signal(0.5);
    let mut h = mounted(move || {
        let seam = div().width(10.0).role(Role::Group).test_id("seam");
        let parts = split::split(div().grow(1.0), div().grow(1.0), share, seam).least(50.0);
        div().size(410.0, 100.0).child(parts)
    });
    let seam = h.get_by_test_id("seam").node();
    let at = h.tree.window_rect(seam).center();
    drag(&mut h, (at.x, at.y), at.x + 102.5);
    assert!((share.get_untracked() - 0.75).abs() < 0.01, "{}", share.get_untracked());
    settle(&mut h); // the seam is where the share puts it now
    let at = h.tree.window_rect(seam).center();
    drag(&mut h, (at.x, at.y), 0.0);
    assert!((share.get_untracked() - 50.0 / 410.0).abs() < 0.01, "{}", share.get_untracked());
}

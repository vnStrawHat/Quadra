//! The headless crate components with bare parts (specs/17, "Tests"): boxes and texts with no look,
//! driven through the semantic tree.

use std::rc::Rc;

use quadra_headless::{choice, popup};
use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_ui::{NodeRef, Place, column, div, row, text};

fn harness() -> Harness {
    let mut h = Harness::new();
    h.tree.set_keymap(Rc::new(quadra_headless::keymap()));
    h
}

fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {}
}

/// A button that opens a popup with two buttons in it, and something else to press.
fn popup_page() -> (Harness, Signal<bool>) {
    let open = signal(false);
    let mut h = harness();
    h.mount(move || {
        let opener = NodeRef::new();
        let toggle = div().role(Role::Button).label("Open").focusable().node_ref(&opener);
        let toggle = toggle.size(80.0, 20.0).on_click(move |_| open.update(|open| *open = !*open));
        let inside = || {
            let one = div().role(Role::Button).label("One").focusable().size(40.0, 20.0);
            column().child(one).child(div().role(Role::Button).label("Two").focusable().size(40.0, 20.0))
        };
        let place = Place::Beside(opener.clone(), quadra_tree::Side::Below, quadra_tree::Along::Start, 0.0);
        let shown = popup::popup(open, place, Some(opener), || column().role(Role::Dialog), inside);
        column().child(toggle).child(shown).child(div().size(200.0, 200.0).child(text("elsewhere")))
    });
    settle(&mut h);
    (h, open)
}

#[test]
fn a_popup_takes_the_focus_keeps_tab_inside_and_gives_the_focus_back() {
    let (mut h, open) = popup_page();
    let opener = h.get_by_role(Role::Button, "Open").node();
    h.tree.focus(opener);
    open.set(true);
    settle(&mut h);
    // The focus is on the first thing inside; Tab goes round the two and stays in.
    let (one, two) = (h.get_by_role(Role::Button, "One").node(), h.get_by_role(Role::Button, "Two").node());
    assert_eq!(h.tree.focused(), Some(one));
    h.key(Key::Tab, Mods::NONE);
    assert_eq!(h.tree.focused(), Some(two));
    h.key(Key::Tab, Mods::NONE);
    assert_eq!(h.tree.focused(), Some(one));
    // Escape closes it, and the focus is back on the opener.
    h.key(Key::Escape, Mods::NONE);
    settle(&mut h);
    assert!(!open.get_untracked() && !h.semantics().contains("Dialog"));
    assert_eq!(h.tree.focused(), Some(opener));
    // A press outside closes it; a press on the opener closes it and does not open it again.
    for press_opener in [false, true] {
        open.set(true);
        settle(&mut h);
        let at = if press_opener {
            h.tree.window_rect(opener).center()
        } else {
            quadra_prim::Point::new(150.0, 150.0)
        };
        h.click(at.x, at.y);
        settle(&mut h);
        assert!(!open.get_untracked(), "closed by a press on the opener: {press_opener}");
    }
}

#[test]
fn a_choice_is_one_stop_of_tab_its_arrows_go_round_and_a_click_chooses() {
    let (chosen, tab) = (signal(0usize), signal(1usize));
    let mut h = harness();
    h.mount(move || {
        let radios = (0..3)
            .map(|i| choice::option(div().size(20.0, 20.0).label(format!("R{i}")), Role::Radio, i, chosen));
        let radios = choice::group(row(), Role::Group, 3, chosen).children(radios.collect::<Vec<_>>());
        let tabs =
            (0..2).map(|i| choice::option(div().size(20.0, 20.0).label(format!("T{i}")), Role::Tab, i, tab));
        let tabs = choice::group(row(), Role::TabList, 2, tab).children(tabs.collect::<Vec<_>>());
        column().child(radios).child(tabs)
    });
    settle(&mut h);
    let tree = h.semantics();
    for line in ["Radio \"R0\" [checked]", "Radio \"R1\" [unchecked]", "Tab \"T1\" [selected]"] {
        assert!(tree.contains(line), "{line} is not in:\n{tree}");
    }
    // The group has the focus, and the arrows go round its ends.
    let group = h.get_by_role(Role::Group, "").node();
    h.tree.focus(group);
    h.key(Key::Left, Mods::NONE);
    assert_eq!(chosen.get_untracked(), 2);
    h.key(Key::Right, Mods::NONE);
    assert_eq!(chosen.get_untracked(), 0);
    // Tab goes from the group to the tab list: the options are not stops of their own.
    h.key(Key::Tab, Mods::NONE);
    assert_eq!(h.tree.focused(), Some(h.get_by_role(Role::TabList, "").node()));
    h.get_by_role(Role::Tab, "T0").click();
    settle(&mut h);
    assert_eq!(tab.get_untracked(), 0);
}

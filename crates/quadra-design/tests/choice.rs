//! The controls that hold a choice (specs/design): a checkbox and a switch are on or
//! off, a radio group and a segmented control hold one of a few. Each writes the signal it
//! was given, by the pointer, by the keyboard, or when a reader asks.

use std::rc::Rc;

use quadra_design::{KitTheme, checkbox, dark, radio_group, segmented, select, slider, switch, token};
use quadra_platform::{Key, Mods};
use quadra_prim::{Color, Size};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{NodeId, Role, SemanticAction};
use quadra_view::column;

struct Page {
    h: Harness,
    theme: KitTheme,
    wrap: Signal<bool>,
    live: Signal<bool>,
    mode: Signal<usize>,
    view: Signal<usize>,
}

fn page() -> Page {
    let (wrap, live, locked) = (signal(false), signal(true), signal(false));
    let (mode, view) = (signal(0usize), signal(1usize));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let wrap = checkbox(wrap).label("Word wrap").test_id("wrap");
        let off = checkbox(locked).label("Locked").disabled(true).test_id("locked");
        let live = switch(live).label("Live").test_id("live");
        let modes = radio_group(["Light", "Dark", "System"], mode);
        let views = segmented(["List", "Grid"], view);
        quadra_design::root()
            .child(column().gap(12.0).child(wrap).child(off).child(live).child(modes).child(views))
    });
    h.frame();
    Page { h, theme: dark(), wrap, live, mode, view }
}

impl Page {
    fn node(&mut self, role: Role, name: &str) -> NodeId {
        self.h.get_by_role(role, name).node()
    }

    fn fill(&mut self, role: Role, name: &str) -> Color {
        let node = self.node(role, name);
        self.h.tree.paint_style(node).background
    }
}

#[test]
fn a_checkbox_and_a_switch_are_turned_by_the_pointer_the_keyboard_and_a_reader() {
    let mut p = page();
    let tree = p.h.semantics();
    for line in [
        "Checkbox \"Word wrap\" [unchecked] #wrap",
        "Checkbox \"Locked\" [unchecked] [disabled] #locked",
        "Switch \"Live\" [checked] #live",
    ] {
        assert!(tree.contains(line), "{line} is not in:\n{tree}");
    }
    // The box is 16 px square; the switch 38 by 26 with its ring.
    let (wrap, live) = (p.node(Role::Checkbox, "Word wrap"), p.node(Role::Switch, "Live"));
    let (boxed, track) = (p.h.tree.children(wrap)[0], p.h.tree.children(live)[0]);
    assert_eq!(
        (p.h.tree.rect(boxed).size(), p.h.tree.rect(track).size()),
        (Size::new(16.0, 16.0), Size::new(38.0, 26.0))
    );

    // A click on the label turns the box, and the check shows; Space turns it back.
    let at = p.h.tree.window_rect(wrap);
    p.h.click(at.right() - 4.0, at.center().y);
    p.h.frame();
    assert!(p.wrap.get_untracked() && p.h.semantics().contains("Checkbox \"Word wrap\" [checked] [focused]"));
    let place = p.h.tree.children(boxed)[0];
    assert_eq!(p.h.tree.children(place).len(), 1); // the check is shown
    p.h.key(Key::Space, Mods::NONE);
    assert!(!p.wrap.get_untracked());

    // A reader toggles the switch; the one that is disabled refuses.
    p.h.get_by_test_id("live").act(SemanticAction::Toggle);
    p.h.frame();
    assert!(!p.live.get_untracked());
    let knob_end = |p: &mut Page| {
        let inner = p.h.tree.children(track)[0];
        let knob = p.h.tree.children(inner)[0];
        p.h.tree.rect(knob).x
    };
    assert_eq!(knob_end(&mut p), 2.0 + 1.0); // off: at the start, after the padding and the border
    let locked = p.node(Role::Checkbox, "Locked");
    assert!(
        !p.h.tree.act(locked, &SemanticAction::Click) || !p.h.semantics().contains("\"Locked\" [checked]")
    );
    // Tab: the box, then the switch; the disabled one is passed by. The focus is a line
    // around the control.
    p.h.tree.blur();
    p.h.key(Key::Tab, Mods::NONE);
    p.h.key(Key::Tab, Mods::NONE);
    p.h.frame();
    assert_eq!(p.h.tree.focused(), Some(live));
    assert_eq!(p.h.tree.paint_style(live).outline, p.theme.color(token::BORDER_FOCUSED));
}

#[test]
fn a_radio_group_and_a_segmented_control_hold_one_of_a_few() {
    let mut p = page();
    let tree = p.h.semantics();
    assert!(
        tree.contains("Radio \"Light\" [checked]") && tree.contains("Radio \"System\" [unchecked]"),
        "{tree}"
    );
    assert!(tree.contains("Radio \"Grid\" [checked]"));

    p.h.get_by_role(Role::Radio, "System").click();
    p.h.frame();
    assert_eq!(p.mode.get_untracked(), 2);
    assert!(p.h.semantics().contains("Radio \"System\" [checked]"));
    // The group is one stop of Tab; the arrows move the choice, round the ends.
    let system = p.node(Role::Radio, "System");
    let group = p.h.tree.parent(system).unwrap();
    p.h.tree.focus(group);
    p.h.key(Key::Down, Mods::NONE);
    assert_eq!(p.mode.get_untracked(), 0);
    p.h.key(Key::Up, Mods::NONE);
    p.h.key(Key::Left, Mods::NONE);
    assert_eq!(p.mode.get_untracked(), 1);

    // The chosen cell is tinted, with an accent label; the others have no fill.
    let (tint, none) = (p.theme.color(token::ACCENT_TINT), Color::TRANSPARENT);
    assert_eq!((p.fill(Role::Radio, "Grid"), p.fill(Role::Radio, "List")), (tint, none));
    p.h.get_by_role(Role::Radio, "List").click();
    p.h.frame();
    assert_eq!(
        (p.view.get_untracked(), p.fill(Role::Radio, "List"), p.fill(Role::Radio, "Grid")),
        (0, tint, none)
    );
    let list = p.node(Role::Radio, "List");
    // The group is the md control height, 28 with its border: a cell is 26 inside it.
    assert_eq!(p.h.tree.rect(list).height, 26.0);
    assert_eq!(p.h.tree.rect(p.h.tree.parent(list).unwrap()).height, 28.0);
    assert_eq!(p.h.tree.inherited(p.h.tree.children(list)[0]).color, p.theme.color(token::ACCENT_TEXT));
}

#[test]
fn a_label_names_a_group_a_select_and_a_slider() {
    let (mode, theme, size) = (signal(0usize), signal(1usize), signal(40.0));
    let mut h = Harness::new();
    h.mount(move || {
        let what = move || if mode.get() == 0 { "Text size" } else { "Line height" };
        column()
            .child(radio_group(["Insert", "Append"], mode).label("A file there"))
            .child(select(["Dark", "Light"], theme).label("Theme"))
            .child(select(["Dark", "Light"], theme))
            .child(slider(size).label(what))
    });
    h.frame();
    h.get_by_role(Role::Group, "A file there");
    // Labelled, the select tells the option it shows as its value; without, as its name.
    let labelled = h.get_by_role(Role::Button, "Theme").node();
    let says = h.tree.semantic_tree();
    let node = says.nodes.iter().find(|n| h.tree.node_of(n.id) == Some(labelled)).unwrap();
    assert_eq!(node.semantics.value.as_deref(), Some("Light"));
    h.get_by_role(Role::Button, "Light");
    h.get_by_role(Role::Slider, "Text size");
    mode.set(1);
    h.frame();
    h.get_by_role(Role::Slider, "Line height");
}

#[test]
fn a_hint_is_under_an_option_and_describes_it() {
    let lock = signal(0);
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let after = radio_group(["After 1 min", "After 5 min"], lock).hints(["the default", ""]);
        quadra_design::root().child(after)
    });
    h.frame();
    // Named by its label alone, the hint under it where the label starts.
    let option = h.get_by_role(Role::Radio, "After 1 min").node();
    let label = h.query(|n| n.semantics.role == Role::Text && n.semantics.name == "After 1 min")[0];
    let hint = h.get_by_text("the default").node();
    let (label, hint) = (h.tree.window_rect(label), h.tree.window_rect(hint));
    assert!(hint.y >= label.bottom() && (hint.x - label.x).abs() < 0.5, "{hint:?} under {label:?}");
    assert!(h.tree.window_rect(option).contains(hint.center()));
    // A click on the hint chooses its option, as one on the label does.
    h.get_by_role(Role::Radio, "After 5 min").click();
    h.frame();
    assert_eq!(lock.get_untracked(), 1);
    h.click(hint.center().x, hint.center().y);
    h.frame();
    assert_eq!(lock.get_untracked(), 0);
}

//! The parts a page is laid out with (specs/13, "Components"): what shows and does nothing,
//! the accordion, and the split.

use std::rc::Rc;

use quadra_kit::{
    Tone, accordion, avatar, badge, card, card_head, dark, divider, heading, key_label, split, tag,
};
use quadra_platform::{Key, Mods};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role};
use quadra_ui::{View, column, div, text};

fn mount<V: View + 'static>(build: impl FnOnce() -> V + 'static) -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_kit::keymap()));
    h.mount(move || quadra_kit::root().child(build()));
    h.frame();
    h
}

fn settle(h: &mut Harness) {
    h.frame();
    while h.frame_if_needed().is_some() {}
}

#[test]
fn what_shows_is_sized_and_named_by_the_design_language() {
    let mut h = mount(|| {
        let about = card().child(card_head("Ada Lovelace", "Notes on the engine")).child(divider());
        let chips = column().child(badge("beta", Tone::Info)).child(tag("v1.2"));
        column().child(heading("People", 1)).child(avatar("Ada Lovelace", 32.0)).child(chips).child(about)
    });
    let tree = h.semantics();
    for line in
        ["Heading \"People\"", "Image \"Ada Lovelace\"", "Text \"AL\"", "Text \"Notes on the engine\""]
    {
        assert!(tree.contains(line), "{line} is not in:\n{tree}");
    }
    // A badge is 16 px tall, a tag 18, and each as wide as its text, not as the column.
    let (beta, version) = (h.get_by_text("beta").node(), h.get_by_text("v1.2").node());
    let (beta, version) = (h.tree.parent(beta).unwrap(), h.tree.parent(version).unwrap());
    assert_eq!((h.tree.rect(beta).height, h.tree.rect(version).height), (16.0, 18.0));
    assert!(h.tree.rect(beta).width < 60.0 && h.tree.rect(version).width < 60.0);
    let face = h.get_by_role(Role::Image, "Ada Lovelace").node();
    assert_eq!(h.tree.rect(face).size(), quadra_prim::Size::new(32.0, 32.0));

    // Shortcuts are written one way: the modifiers in one order, the strokes of a chord apart.
    assert_eq!(key_label("shift-ctrl-p"), "Ctrl-Shift-P");
    assert_eq!(key_label("ctrl-k ctrl-s"), "Ctrl-K Ctrl-S");
    assert_eq!(
        (key_label("alt-pageup"), key_label("ctrl-minus"), key_label("cmd-enter")),
        ("Alt-PageUp".to_string(), "Ctrl--".to_string(), "Win-Enter".to_string())
    );
}

#[test]
fn a_section_of_an_accordion_is_opened_and_closed_by_its_head() {
    let (first, second) = (signal(true), signal(false));
    let mut h = mount(move || {
        accordion()
            .item("General", first, || text("Name and language"))
            .item("Advanced", second, || text("Proxy and certificates"))
    });
    let tree = h.semantics();
    assert!(
        tree.contains("Button \"General\" [expanded]") && tree.contains("Button \"Advanced\" [collapsed]"),
        "{tree}"
    );
    assert!(h.has_text("Name and language") && !h.has_text("Proxy and certificates"));
    // A head is 28 px tall, the line between two sections included.
    let (general, advanced) =
        (h.get_by_role(Role::Button, "General").node(), h.get_by_role(Role::Button, "Advanced").node());
    assert_eq!((h.tree.rect(general).height, h.tree.rect(advanced).height), (28.0, 28.0));

    h.get_by_role(Role::Button, "Advanced").click();
    settle(&mut h);
    assert!(second.get_untracked() && h.has_text("Proxy and certificates"));
    // The keyboard does the same: the head with the focus closes its section.
    h.tree.focus(general);
    h.key(Key::Enter, Mods::NONE);
    settle(&mut h);
    assert!(!first.get_untracked() && !h.has_text("Name and language"));
    assert!(h.semantics().contains("Button \"General\" [collapsed]"));
}

fn panes(share: Signal<f32>, stacked: bool) -> Harness {
    mount(move || {
        let both = split(div().child(text("left")), div().child(text("right")), share);
        if stacked { both.stacked() } else { both }
    })
}

#[test]
fn the_seam_of_a_split_is_dragged_to_give_a_pane_more_room() {
    let share = signal(0.25f32);
    let mut h = panes(share, false);
    let width = |h: &mut Harness, words: &str| {
        let pane = h.get_by_text(words).node();
        let pane = h.tree.parent(h.tree.parent(pane).unwrap()).unwrap();
        h.tree.window_rect(pane)
    };
    // A quarter of the 800 px, the seam's 6, and the rest.
    let (left, right) = (width(&mut h, "left"), width(&mut h, "right"));
    assert_eq!((left.width, right.x, right.width, left.height), (200.0, 206.0, 594.0, 600.0));

    h.pointer(PointerInput::Move, 203.0, 300.0);
    h.pointer(PointerInput::Down, 203.0, 300.0);
    h.pointer(PointerInput::Move, 303.0, 20.0); // 100 px of 800 to the right
    settle(&mut h);
    assert_eq!((share.get_untracked(), width(&mut h, "left").width), (0.375, 300.0));
    h.pointer(PointerInput::Move, -500.0, 300.0); // no pane is dragged under 80 px
    assert_eq!(share.get_untracked(), 0.1);
    h.pointer(PointerInput::Up, -500.0, 300.0);
    h.pointer(PointerInput::Move, 400.0, 300.0); // let go: a move changes nothing
    settle(&mut h);
    assert_eq!(width(&mut h, "left").width, 80.0);

    // A double click on the seam makes the panes even.
    h.click(83.0, 300.0);
    h.click(83.0, 300.0);
    settle(&mut h);
    assert_eq!(share.get_untracked(), 0.5);

    // Stacked, the share is of the height.
    let mut h = panes(signal(0.5), true);
    let (top, bottom) = (width(&mut h, "left"), width(&mut h, "right"));
    assert_eq!((top.height, bottom.y, bottom.height, top.width), (300.0, 306.0, 294.0, 800.0));
}

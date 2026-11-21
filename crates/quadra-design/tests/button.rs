//! The button (specs/behaviour/15): its sizes and the colours of its variants and states come
//! from the theme, a click and the keyboard do the same thing, and a disabled one does
//! nothing. On the fake backend a character is 0.6 of the text size wide.

use std::rc::Rc;

use quadra_design::{KitTheme, Size, Variant, button, dark, token};
use quadra_platform::{Key, Mods};
use quadra_prim::Color;
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{NodeId, PointerInput, Role, Token};
use quadra_view::row;

struct Page {
    h: Harness,
    theme: KitTheme,
    saved: Signal<u32>,
    dirty: Signal<bool>,
    wrap: Signal<bool>,
}

/// Five buttons in a row: a primary "Save" that is disabled until something is dirty, a
/// secondary "Cancel", a ghost toggle "Wrap", a small danger "Delete" and a link.
fn page() -> Page {
    let (saved, dirty, wrap) = (signal(0), signal(false), signal(false));
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.tree.set_keymap(Rc::new(quadra_design::keymap()));
    h.mount(move || {
        let save = button("Save").variant(Variant::Primary).test_id("save");
        let save = save.disabled(move || !dirty.get()).on_click(move || saved.update(|n| *n += 1));
        let cancel = button("Cancel").test_id("cancel");
        let toggle = button("Wrap").variant(Variant::Ghost).toggled(wrap).test_id("wrap");
        let toggle = toggle.on_click(move || wrap.update(|on| *on = !*on));
        let delete = button("Delete").variant(Variant::Danger).size(Size::Xs).test_id("delete");
        let link = button("More").variant(Variant::Link).test_id("more");
        quadra_design::root()
            .child(row().gap(8.0).child(save).child(cancel).child(toggle).child(delete).child(link))
    });
    h.frame();
    Page { h, theme: dark(), saved, dirty, wrap }
}

impl Page {
    fn node(&mut self, id: &str) -> NodeId {
        self.h.get_by_test_id(id).node()
    }

    /// The fill, the border and the label's colour of a button.
    fn look(&mut self, id: &str) -> (Color, Color, Color) {
        let node = self.node(id);
        let (paint, label) = (self.h.tree.paint_style(node), self.h.tree.children(node)[0]);
        (paint.background, paint.border_color, self.h.tree.inherited(label).color)
    }

    fn is(&self, role: Token) -> Color {
        self.theme.color(role)
    }

    fn over(&mut self, id: &str) {
        let node = self.node(id);
        let at = self.h.tree.window_rect(node).center();
        self.h.pointer(PointerInput::Move, at.x, at.y);
        self.h.frame();
    }
}

#[test]
fn buttons_have_the_sizes_and_the_colours_of_their_variants() {
    let mut p = page();
    let tree = p.h.semantics();
    let lines: Vec<&str> = tree.lines().map(str::trim).collect();
    assert_eq!(
        lines[1..],
        [
            "Button \"Save\" [disabled] #save",
            "Button \"Cancel\" #cancel",
            "Button \"Wrap\" [unchecked] #wrap",
            "Button \"Delete\" #delete",
            "Button \"More\" #more",
        ]
    );
    // 28 px high, 8 px beside a label of 14 px text; the smallest is 18 high around 12 px text.
    let (save, delete) = (p.node("save"), p.node("delete"));
    let (save, delete) = (p.h.tree.rect(save).size(), p.h.tree.rect(delete).size());
    // A tinted button has a border of its colour, a pixel each side.
    assert!((save.width - (18.0 + 4.0 * 8.4)).abs() < 0.01 && save.height == 28.0, "{save:?}");
    assert!((delete.width - (10.0 + 6.0 * 7.2)).abs() < 0.01 && delete.height == 18.0, "{delete:?}");

    let none = Color::TRANSPARENT;
    let colours = p.theme.theme();
    let (accent, error) =
        (colours.resolve(token::ACCENT.alpha(0.42)), colours.resolve(token::ERROR.alpha(0.42)));
    // Disabled: its fill, and the colour of what is off.
    assert_eq!(p.look("save"), (p.is(token::ACCENT_TINT), accent, p.is(token::TEXT_DISABLED)));
    assert_eq!(p.look("cancel"), (none, p.is(token::LINE_STRONG), p.is(token::TEXT)));
    assert_eq!(p.look("wrap"), (none, none, p.is(token::TEXT)));
    assert_eq!(p.look("delete"), (p.is(token::ERROR_TINT), error, p.is(token::TEXT)));
    assert_eq!(p.look("more"), (none, none, p.is(token::ACCENT_TEXT)));

    // Under the pointer: a ghost and an outlined one get the hover step, a disabled one
    // nothing.
    p.over("wrap");
    assert_eq!(p.look("wrap").0, p.is(token::PANEL_HOVER));
    p.over("cancel");
    assert_eq!(p.look("cancel"), (p.is(token::PANEL_HOVER), p.is(token::LINE_STRONG), p.is(token::TEXT)));
    p.over("save");
    assert_eq!(p.look("save").0, p.is(token::ACCENT_TINT));
    p.dirty.set(true);
    p.h.frame();
    assert_eq!(p.look("save"), (p.is(token::ACCENT_TINT_HOVER), accent, p.is(token::TEXT)));
}

#[test]
fn a_click_and_the_keyboard_do_the_same_and_a_disabled_button_does_nothing() {
    let mut p = page();
    p.h.get_by_role(Role::Button, "Save").click();
    let save = p.node("save");
    let at = p.h.tree.window_rect(save).center();
    p.h.click(at.x, at.y);
    assert_eq!(p.saved.get_untracked(), 0);
    p.h.key(Key::Tab, Mods::NONE); // the first button that is not disabled
    let cancel = p.node("cancel");
    assert_eq!(p.h.tree.focused(), Some(cancel));
    p.h.frame();
    // A focus that came by the keyboard: the border of an outlined button says so.
    assert_eq!(p.look("cancel").1, p.is(token::BORDER_FOCUSED));
    // One with no border: a line of a pixel along its edge.
    p.h.key(Key::Tab, Mods::NONE);
    p.h.frame();
    let wrap = p.node("wrap");
    assert_eq!(p.h.tree.paint_style(wrap).outline, p.is(token::BORDER_FOCUSED));

    p.dirty.set(true);
    p.h.frame();
    p.h.click(at.x, at.y);
    p.h.key(Key::Enter, Mods::NONE); // the click gave it the focus: Enter and Space press it
    p.h.key(Key::Space, Mods::NONE);
    assert_eq!(p.saved.get_untracked(), 3);

    // A toggle: its owner turns it, and it shows the accent and the selected step while on.
    p.h.get_by_test_id("wrap").click();
    p.h.frame();
    assert!(p.wrap.get_untracked() && p.h.semantics().contains("Button \"Wrap\" [checked]"));
    let on = p.look("wrap");
    assert_eq!((on.0, on.2), (p.is(token::PRESS), p.is(token::ACCENT_TEXT)));
}

#[test]
fn a_button_shows_an_icon_before_its_label_or_alone() {
    use quadra_design::{icon_button, icons};

    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(|| {
        let save = button("Save").size(Size::Sm).icon(icons::SAVE).test_id("save");
        let large = button("Open").size(Size::Lg).icon(icons::FOLDER).test_id("open");
        let close = icon_button(icons::X, "Close").test_id("close");
        quadra_design::root().child(row().gap(8.0).child(save).child(large).child(close))
    });
    let (stats, _) = h.frame();
    // A reader hears the label, or the name of a button that has none.
    assert!(
        h.semantics().contains("Button \"Save\" #save") && h.semantics().contains("Button \"Close\" #close")
    );
    let size = |h: &mut Harness, id: &str, child: usize| {
        let button = h.get_by_test_id(id).node();
        let part = h.tree.children(button)[child];
        (h.tree.rect(button).size(), h.tree.rect(part))
    };
    // 14 px in a small button, after its border and its 4 px, and 4 px before the label; 16
    // in the two larger sizes.
    let (save, icon) = size(&mut h, "save", 0);
    assert_eq!((icon.x.round(), icon.width, icon.height), (5.0, 14.0, 14.0));
    assert!((save.width - (2.0 + 4.0 + 14.0 + 4.0 + 4.0 * 8.4 + 4.0)).abs() < 0.01, "{save:?}");
    assert_eq!(size(&mut h, "open", 0).1.width, 16.0);
    // Alone it sits in the middle of a square of 22.
    let (close, x) = size(&mut h, "close", 0);
    assert_eq!((close, x.x, x.y), (quadra_prim::Size::new(22.0, 22.0), 4.0, 4.0));
    // An icon is one instance: a tile of the atlas in the colour of the label.
    // The canvas, three faces (a ghost at rest shows nothing, and keeps its place for when
    // the pointer comes), three icons, and eight letters.
    assert_eq!(stats.instances_written, 1 + 3 + 3 + 8);
}

#[test]
fn a_keymap_file_names_the_kits_actions() {
    let mut p = page();
    let mut keymap = quadra_design::keymap();
    keymap.load("[control && !menu]\nf9 = kit::confirm\n", &quadra_design::actions()).unwrap();
    p.h.tree.set_keymap(Rc::new(keymap));
    p.dirty.set(true);
    p.h.frame();
    let save = p.node("save");
    p.h.tree.focus(save);
    p.h.key(Key::F9, Mods::NONE);
    assert_eq!(p.saved.get_untracked(), 1);
}

#[test]
fn a_label_names_a_button_apart_from_its_words() {
    let (reset, which) = (signal(0), signal("Copy".to_string()));
    let mut h = Harness::new();
    h.mount(move || {
        let first = button("Reset").label(move || format!("Reset {}", which.get()));
        let second = button("Reset").label("Reset Paste").on_click(move || reset.set(2));
        quadra_design::root().child(row().child(first).child(second))
    });
    h.frame();
    assert!(h.has_text("Reset"), "the words shown are the label's");
    h.get_by_role(Role::Button, "Reset Paste").click();
    assert_eq!(reset.get_untracked(), 2);
    which.set("Lock".to_string());
    h.frame();
    h.get_by_role(Role::Button, "Reset Lock");
    assert!(h.query(|n| n.semantics.name == "Reset Copy").is_empty());
}

#[test]
fn an_icon_after_the_label_is_sized_and_spaced_as_one_before_it() {
    use quadra_design::icons;

    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(|| {
        let open = button("Open role").size(Size::Sm).icon_end(icons::CHEVRON_RIGHT).test_id("open");
        quadra_design::root().child(row().child(open))
    });
    h.frame();
    // The reader hears the label alone; the chevron is 14 px, 4 px after the label and 4 px
    // before the border.
    assert!(h.semantics().contains("Button \"Open role\" #open"));
    let button = h.get_by_test_id("open").node();
    let (label, chevron) = (h.tree.children(button)[0], h.tree.children(button)[1]);
    let (at, label, chevron) = (h.tree.rect(button), h.tree.rect(label), h.tree.rect(chevron));
    assert_eq!((chevron.width, chevron.height), (14.0, 14.0));
    assert!(
        (chevron.x - label.right() - 4.0).abs() < 0.01 && (at.width - chevron.right() - 5.0).abs() < 0.01
    );
}

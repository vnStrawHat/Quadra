//! Roles of the theme in the styles of views (specs/13, "Tokens"): a box takes a role where
//! it takes a colour, constant, by state or reactive, and has another colour under another
//! theme without any of its view running again.

use std::rc::Rc;

use quadra_prim::Color;
use quadra_reactive::{signal, stats};
use quadra_testing::Harness;
use quadra_tree::{Paint, PointerInput, Theme, Token};
use quadra_ui::{column, div, rich_text, show, span};

const SURFACE: Token = Token(0);
const LINE: Token = Token(1);
const ACCENT: Token = Token(2);

fn theme(colors: [u32; 3]) -> Rc<Theme> {
    Rc::new(Theme::new(colors.map(Color::rgb).to_vec()))
}

#[test]
fn boxes_take_roles_where_they_take_colours() {
    let on = signal(false);
    let mut h = Harness::new();
    h.tree.set_theme(theme([0x202020, 0x404040, 0x3366FF]));
    h.mount(move || {
        let card = div().size(100.0, 40.0).border(1.0).bg(SURFACE).border_color(LINE);
        let card = card.hover(|s| s.bg(ACCENT.alpha(0.5)));
        // A role or a colour, as a signal says.
        let dot = div().size(10.0, 10.0);
        let dot = dot.bg(move || if on.get() { Paint::from(ACCENT) } else { Color::rgb(0x808080).into() });
        column().child(card).child(dot)
    });
    h.frame();
    let nodes = h.tree.children(h.tree.root().unwrap()).to_vec();
    let look = |h: &Harness| {
        let (card, dot) = (h.tree.paint_style(nodes[0]), h.tree.paint_style(nodes[1]));
        (card.background, card.border_color, dot.background)
    };
    assert_eq!(look(&h), (Color::rgb(0x202020), Color::rgb(0x404040), Color::rgb(0x808080)));
    on.set(true);
    h.pointer(PointerInput::Move, 50.0, 20.0);
    h.frame();
    assert_eq!(look(&h).0, Color::rgb(0x3366FF).alpha(0.5));
    assert_eq!(look(&h).2, Color::rgb(0x3366FF));

    // Another theme: the two boxes are styled and painted again. No view code runs, no
    // binding, and nothing is laid out.
    let before = stats();
    h.tree.set_theme(theme([0xFFFFFF, 0xD0D0D0, 0xCC0000]));
    let (frame, _) = h.frame();
    assert_eq!(
        (frame.nodes_styled, frame.bindings_run, frame.nodes_laid_out, frame.nodes_painted),
        (2, 0, 0, 2)
    );
    assert_eq!(look(&h), (Color::rgb(0xCC0000).alpha(0.5), Color::rgb(0xD0D0D0), Color::rgb(0xCC0000)));
    assert_eq!(stats(), before);
}

#[test]
fn a_box_gives_what_is_under_it_a_theme_of_its_own() {
    let (more, gone, dark) = (signal(false), signal(false), theme([0x101010, 0x202020, 0x00AA00]));

    let mut h = Harness::new();
    h.tree.set_theme(theme([0xFFFFFF, 0xD0D0D0, 0xCC0000]));
    h.mount(move || {
        let card = || div().size(40.0, 20.0).bg(SURFACE);
        let words = rich_text([span("x").color(ACCENT)]);
        let panel =
            div().theme(dark.clone()).child(card()).child(words).child(show(move || more.get(), card));
        column()
            .child(card())
            .child(panel)
            .child(show(move || !gone.get(), move || div().theme(dark.clone())))
    });
    h.frame();
    let root = h.tree.root().unwrap();
    let (outside, panel) = (h.tree.children(root)[0], h.tree.children(root)[1]);
    let bg = |h: &Harness, id| h.tree.paint_style(id).background;
    let inside = h.tree.children(panel)[0];
    assert_eq!((bg(&h, outside), bg(&h, inside)), (Color::rgb(0xFFFFFF), Color::rgb(0x101010)));
    // A widget that paints with roles paints with the panel's: its glyph is the panel's accent.
    let inked = |h: &Harness, color: Color| {
        let layer = h.tree.scene().layer(h.tree.scene().roots()[0]);
        layer.instances().iter().filter(|i| i.color == color.to_array()).count()
    };
    assert_eq!((inked(&h, Color::rgb(0x00AA00)), inked(&h, Color::rgb(0xCC0000))), (1, 0));
    // A box with a theme that goes leaves nothing to look up.
    gone.set(true);
    h.frame();
    assert_eq!(h.tree.themes_held(), 1);

    // What comes under it later has its theme too; the window's theme leaves it as it is.
    more.set(true);
    h.frame();
    let mut later = panel;
    while let Some(&last) = h.tree.children(later).last() {
        later = last;
    }
    assert_ne!(later, inside);
    assert_eq!(bg(&h, later), Color::rgb(0x101010));
    h.tree.set_theme(theme([0xEEEEEE, 0xD0D0D0, 0xCC0000]));
    h.frame();
    assert_eq!((bg(&h, outside), bg(&h, inside)), (Color::rgb(0xEEEEEE), Color::rgb(0x101010)));
    // Given up, the window's again.
    h.tree.set_theme_of(panel, None);
    h.frame();
    assert_eq!(bg(&h, inside), Color::rgb(0xEEEEEE));
}

#[test]
fn spaces_and_radii_can_be_measures_of_the_theme() {
    const SPACE: quadra_tree::Measure = quadra_tree::Measure(0);
    const ROUND: quadra_tree::Measure = quadra_tree::Measure(1);
    let measured =
        |space: f32, round: f32| Rc::new(Theme::new(vec![Color::rgb(0)]).with_measures(vec![space, round]));
    let mut h = Harness::new();
    h.tree.set_theme(measured(8.0, 4.0));
    h.mount(|| {
        let padded = div().padding(SPACE).child(div().size(10.0, 10.0));
        let rounded = div().size(20.0, 20.0).radius(ROUND).bg(Color::rgb(0x808080));
        // A number given after a measure of the same property takes its place.
        let square = div().size(20.0, 20.0).radius(ROUND).radius(0.0).bg(SURFACE).bg(Color::rgb(0x112233));
        column().child(padded).child(rounded).child(div().size(30.0, 30.0).padding(2.0)).child(square)
    });
    h.frame();
    let nodes = h.tree.children(h.tree.root().unwrap()).to_vec();
    assert_eq!((h.tree.rect(nodes[0]).height, h.tree.paint_style(nodes[1]).radii.top_left), (26.0, 4.0));
    let square = h.tree.paint_style(nodes[3]);
    assert_eq!((square.radii.top_left, square.background), (0.0, Color::rgb(0x112233)));
    // Other numbers: the two boxes that hold measures are styled again; the padded one is
    // laid out, the rounded one only painted. The third holds none and is not looked at.
    h.tree.set_theme(measured(12.0, 6.0));
    let (frame, _) = h.frame();
    assert_eq!(frame.nodes_styled, 2);
    assert_eq!((h.tree.rect(nodes[0]).height, h.tree.paint_style(nodes[1]).radii.top_left), (34.0, 6.0));
}

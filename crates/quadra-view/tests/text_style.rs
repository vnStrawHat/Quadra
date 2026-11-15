//! Text values (specs/13, "Style values"): what a box says of text holds for the text in it
//! and in what it holds, until a box or a text on the way says otherwise. A change of what
//! shapes text lays out the texts under it; a change of colour only paints them.

use std::rc::Rc;

use quadra_prim::Color;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::{NodeId, PointerInput, Theme, Token};
use quadra_view::{column, div, text};

const TEXT: Token = Token(0);
const ACCENT: Token = Token(1);

fn theme(text: u32, accent: u32) -> Rc<Theme> {
    Rc::new(Theme::new(vec![Color::rgb(text), Color::rgb(accent)]))
}

#[test]
fn text_is_as_the_boxes_around_it_say_until_one_says_otherwise() {
    let size = signal(10.0f32);
    let mut h = Harness::new();
    h.tree.set_theme(theme(0x202020, 0x3366FF));
    h.mount(move || {
        // A page of 10 px text in the theme's text colour; a note inside it is larger and
        // red; a button's label turns the accent under the pointer.
        let note = div().text_color(Color::rgb(0xCC0000)).child(text("note").size(20.0)).child(text("more"));
        let button = div().size(100.0, 20.0).hover(|s| s.text_color(ACCENT)).child(text("Save"));
        let page = column().text_size(size).text_color(TEXT).font_weight(600);
        page.child(text("plain")).child(note).child(button)
    });
    h.frame();
    let root = h.tree.root().unwrap();
    let under = |h: &Harness, path: &[usize]| path.iter().fold(root, |at, &i| h.tree.children(at)[i]);
    let (plain, note, more, save) =
        (under(&h, &[0]), under(&h, &[1, 0]), under(&h, &[1, 1]), under(&h, &[2, 0]));
    let look = |h: &Harness, id: NodeId| {
        let at = h.tree.inherited(id);
        (at.style.size, at.style.weight, at.color)
    };
    assert_eq!(look(&h, plain), (10.0, 600, Color::rgb(0x202020)));
    assert_eq!(look(&h, note), (20.0, 600, Color::rgb(0xCC0000))); // its own size, the box's colour
    assert_eq!(look(&h, more), (10.0, 600, Color::rgb(0xCC0000)));
    // On the fake backend a character is 0.6 of the size wide, and a line 1.4 of it high.
    assert_eq!(h.tree.rect(plain).size(), quadra_prim::Size::new(800.0, 14.0));
    assert_eq!(h.tree.rect(note).height, 28.0);

    // The page's text grows: the texts that take their size from it are laid out again,
    // the one with a size of its own is not measured.
    size.set(12.0);
    let (stats, _) = h.frame();
    // A line is 1.4 of the size, in whole pixels.
    assert_eq!((h.tree.rect(plain).height, h.tree.rect(note).height), (17.0, 28.0));
    assert_eq!(look(&h, save).0, 12.0);
    assert!(stats.nodes_measured >= 3, "{stats:?}");

    // The pointer over the button: its label turns the accent. One node styled, the label
    // painted with it, nothing laid out.
    h.pointer(PointerInput::Move, 50.0, h.tree.window_rect(save).y + 5.0);
    let (stats, _) = h.frame();
    assert_eq!(look(&h, save).2, Color::rgb(0x3366FF));
    assert_eq!((stats.nodes_styled, stats.nodes_laid_out, stats.nodes_painted), (1, 0, 1));

    // Another theme: the texts in the theme's colour are painted again, and none is laid out.
    h.tree.set_theme(theme(0xF0F0F0, 0xFF8800));
    let (stats, _) = h.frame();
    assert_eq!((look(&h, plain).2, look(&h, save).2), (Color::rgb(0xF0F0F0), Color::rgb(0xFF8800)));
    assert_eq!(look(&h, more).2, Color::rgb(0xCC0000)); // a colour is a colour under any theme
    assert_eq!(stats.nodes_laid_out, 0);
    assert_eq!(stats.nodes_painted, 2); // "plain" and "Save": the note's two keep their red
}

#[test]
fn a_text_with_an_ellipsis_shows_what_fits_and_dots_where_its_box_ends() {
    let width = signal(30.0f32);
    let mut h = Harness::new();
    h.mount(move || {
        let title = text("abcdefghij").ellipsis();
        column()
            .text_size(10.0)
            .child(div().width(move || quadra_layout::Length::Px(width.get())).child(title))
    });
    h.frame();
    // 6 px a letter on the fake backend, and as much for the dots: four letters and them.
    let glyphs = |h: &Harness| {
        let scene = h.tree.scene();
        let all = scene.roots().iter().flat_map(|&root| scene.layer(root).instances());
        all.filter(|i| i.kind == quadra_render::Kind::Mono as u32).count()
    };
    assert_eq!(glyphs(&h), 5);
    // Room for all of it: no dots.
    width.set(200.0);
    h.frame();
    assert_eq!(glyphs(&h), 10);
    let node = h.get_by_text("abcdefghij").node();
    assert_eq!(h.tree.window_rect(node).height, 14.0); // one line all the way
}

//! Rich text (specs/05, "Rules"): spans of one paragraph, their colours and their links. On
//! the fake backend a character is 0.6 of its size wide: 6 px at size 10, on lines of 14.

use std::rc::Rc;

use quadra_platform::{Cursor, Mods, MouseButton};
use quadra_prim::{Color, Point, Size};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{NodeId, PointerInput, Theme, Token};
use quadra_ui::{column, rich_text, span};

const BLUE: Color = Color::rgb(0x0000FF);
const RED: Color = Color::rgb(0xFF0000);

/// "ab cdef gh" in a box of a width: "cd" twice as large and in the theme's colour, "gh" a
/// link in it.
fn mounted(width: f32) -> (Harness, NodeId, Signal<String>) {
    let clicked = signal(String::new());
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(Theme::new(vec![BLUE])));
    h.mount(move || {
        let large = span("cd").size(20.0).color(Token(0));
        let words = rich_text([span("ab "), large, span("ef "), span("gh").link("there").color(Token(0))]);
        let words = words.on_link(move |target| clicked.set(target.to_string()));
        column().width(width).text_size(10.0).line_height(1.4).child(words)
    });
    h.frame();
    let words = h.tree.children(h.tree.root().unwrap())[0];
    (h, words, clicked)
}

/// How many of the instances painted are of a colour.
fn count(h: &Harness, color: Color) -> usize {
    let layer = h.tree.scene().layer(h.tree.scene().roots()[0]);
    layer.instances().iter().filter(|instance| instance.color == color.to_array()).count()
}

#[test]
fn spans_are_one_paragraph_in_their_own_sizes_and_colours() {
    let (mut h, words, _) = mounted(100.0);
    // One line: 3 and 2 and 3 characters of 6 px, two of 12; it is named by all of it.
    assert_eq!(h.tree.rect(words).size(), Size::new(100.0, 14.0));
    assert!(h.has_text("ab cdef gh"));
    // Two large letters, two of the link and the line under it are in the theme's colour;
    // the four others are in the text's.
    assert_eq!((count(&h, BLUE), count(&h, Color::BLACK)), (5, 4));

    // Another theme: the text is painted again in its colours, and nothing is laid out.
    h.tree.set_theme(Rc::new(Theme::new(vec![RED])));
    let (stats, _) = h.frame();
    assert_eq!((count(&h, RED), count(&h, BLUE), stats.nodes_laid_out), (5, 0, 0));

    // 50 px: "cdef" is one word, though of two spans, and goes to a line of its own.
    let (h, words, _) = mounted(50.0);
    assert_eq!(h.tree.rect(words).height, 42.0);
}

#[test]
fn a_link_takes_the_pointer_and_its_click() {
    let (mut h, _, clicked) = mounted(100.0);
    // "gh" starts 60 px in. The pointer is a hand over it, and over nothing else.
    h.pointer(PointerInput::Move, 5.0, 7.0);
    h.frame();
    assert_eq!(h.tree.cursor(), Cursor::default());
    h.pointer(PointerInput::Move, 65.0, 7.0);
    h.frame();
    assert_eq!(h.tree.cursor(), Cursor::Pointer);
    h.click(5.0, 7.0);
    assert_eq!(clicked.get_untracked(), "");
    h.click(65.0, 7.0);
    assert_eq!(clicked.get_untracked(), "there");
    h.pointer(PointerInput::Move, 80.0, 7.0); // past the link, on the same line
    h.frame();
    assert_eq!(h.tree.cursor(), Cursor::default());

    // Wrapped, the link is on the third line.
    let (mut h, _, clicked) = mounted(50.0);
    h.click(5.0, 7.0);
    assert_eq!(clicked.get_untracked(), "");
    h.click(5.0, 35.0);
    assert_eq!(clicked.get_untracked(), "there");
}

#[test]
fn the_fill_of_a_span_is_behind_it_on_every_line_it_is_on() {
    const FILL: Color = Color::rgb(0x00FF00);
    let fills = |width: f32| {
        let mut h = Harness::new();
        h.tree.set_theme(Rc::new(Theme::new(vec![FILL])));
        h.mount(move || {
            let words = rich_text([span("ab "), span("cd ef").background(Token(0)), span(" gh")]);
            column().width(width).text_size(10.0).line_height(1.4).child(words)
        });
        h.frame();
        let layer = h.tree.scene().layer(h.tree.scene().roots()[0]);
        let filled = layer.instances().iter().filter(|instance| instance.color == FILL.to_array());
        filled.map(|instance| instance.bounds).collect::<Vec<_>>()
    };
    // On one line: five characters of 6 px from the fourth, as high as the line.
    assert_eq!(fills(100.0), [[18.0, 0.0, 30.0, 14.0]]);
    // "ab cd " and "ef gh": the fill is on both lines, the space the line broke at with it.
    assert_eq!(fills(40.0), [[18.0, 0.0, 18.0, 14.0], [0.0, 14.0, 12.0, 14.0]]);
}

#[test]
fn a_struck_span_has_a_line_through_it_and_links_can_wait_for_ctrl() {
    let clicked = signal(String::new());
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(Theme::new(vec![BLUE])));
    h.mount(move || {
        let words = rich_text([span("ab ").strike().color(Token(0)), span("gh").link("there")]).ctrl_links();
        let words = words.on_link(move |target| clicked.set(target.to_string()));
        column().width(100.0).text_size(10.0).line_height(1.4).child(words)
    });
    h.frame();
    // The line through "ab " is one pixel high, as wide as the span, in its colour.
    let layer = h.tree.scene().layer(h.tree.scene().roots()[0]);
    let lines = layer.instances().iter().filter(|i| i.color == BLUE.to_array() && i.bounds[3] == 1.0);
    assert_eq!(lines.map(|i| (i.bounds[0], i.bounds[2])).collect::<Vec<_>>(), [(0.0, 18.0)]);

    // A plain click on the link is no click on a link, and the pointer is no hand over it.
    h.click(20.0, 7.0);
    h.frame();
    assert_eq!((clicked.get_untracked(), h.tree.cursor()), (String::new(), Cursor::default()));
    for input in [PointerInput::Move, PointerInput::Down, PointerInput::Up] {
        h.tree.pointer(input, MouseButton::Left, Point::new(20.0, 7.0), Mods::CTRL, h.now());
    }
    h.frame();
    assert_eq!((clicked.get_untracked().as_str(), h.tree.cursor()), ("there", Cursor::Pointer));
}

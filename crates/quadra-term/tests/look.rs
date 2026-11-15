//! The look as a signal (specs/18, "The view"): a new size, line or room sizes the grid again
//! without mounting the view again; the room a side, the cell's width, the minimum contrast and
//! the fallback families reach what is drawn.

use std::cell::RefCell;
use std::rc::Rc;

use quadra_platform::{Mods, MouseButton};
use quadra_prim::{Color, Edges, Point};
use quadra_reactive::{Signal, signal};
use quadra_render::{Kind, Step};
use quadra_term::vt::ResizePolicy;
use quadra_term::{Link, Session, TermColors, TermLook, contrast_ratio, terminal, with_contrast};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, Role};
use quadra_view::column;

#[test]
fn a_new_look_sizes_the_grid_again() {
    let mut h = Harness::new();
    let look = signal(TermLook { size: 10.0, line_height: 1.4, ..TermLook::default() });
    let slot = Rc::new(RefCell::new(None));
    let out = slot.clone();
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        *out.borrow_mut() = Some(s.clone());
        column().size(400.0, 140.0).child(terminal(&s).look(look))
    });
    h.settle();
    let s: Session = slot.borrow_mut().take().unwrap();
    let node = h.query(|n| n.semantics.role == Role::TextInput)[0];
    let rows = |s: &Session| s.size().rows;
    assert_eq!(rows(&s), 8, "(140 - 16) / 14");
    look.update(|it| it.size = 20.0);
    h.settle();
    assert_eq!(rows(&s), 4, "(140 - 16) / 28");
    look.update(|it| it.padding = Edges::all(20.0));
    h.settle();
    assert_eq!(rows(&s), 3, "(140 - 40) / 28");
    assert_eq!(h.query(|n| n.semantics.role == Role::TextInput)[0], node, "the same view");
}

/// A terminal of 400 x 140 at size 10 (cells 6 x 14), with what it writes to its program.
fn term(look: TermLook) -> (Harness, Session, Signal<TermLook>, Rc<RefCell<Vec<u8>>>) {
    let mut h = Harness::new();
    let look = signal(TermLook { size: 10.0, line_height: 1.4, ..look });
    let (slot, wrote) = (Rc::new(RefCell::new(None)), Rc::new(RefCell::new(Vec::new())));
    let (out, into) = (slot.clone(), wrote.clone());
    h.mount(move || {
        let s = Session::new(ResizePolicy::BottomAnchor);
        s.connect(Link::new(move |bytes| into.borrow_mut().extend_from_slice(bytes)));
        *out.borrow_mut() = Some(s.clone());
        column().size(400.0, 140.0).child(terminal(&s).look(look))
    });
    h.settle();
    let s = slot.borrow_mut().take().unwrap();
    (h, s, look, wrote)
}

/// The glyphs drawn, from the left: where each starts and its colour.
fn glyphs(h: &mut Harness) -> Vec<(f32, Color)> {
    h.frame();
    let scene = h.tree.scene();
    let layers = h.renderer.steps().iter().filter_map(|step| match step {
        Step::Layer { id, .. } => Some(scene.layer(*id).instances()),
        _ => None,
    });
    let mut out: Vec<(f32, Color)> = layers
        .flatten()
        .filter(|i| i.kind == Kind::Mono as u32)
        .map(|i| (i.bounds[0], Color { r: i.color[0], g: i.color[1], b: i.color[2], a: i.color[3] }))
        .collect();
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

#[test]
fn each_side_has_its_own_room() {
    let padding = Edges { top: 4.0, right: 30.0, bottom: 10.0, left: 20.0 };
    let (mut h, s, _, wrote) = term(TermLook { padding, ..TermLook::default() });
    assert_eq!((s.size().rows, s.size().cols), (9, 58), "(140 - 14) / 14, (400 - 50) / 6");
    s.feeder().feed(b"\x1b[?1000h\x1b[?1006h", std::time::Instant::now());
    h.settle();
    // The cell 4 of the row 2: 20 px in from the left, 4 down from the top.
    let at = Point::new(20.0 + 3.0 * 6.0 + 3.0, 4.0 + 14.0 + 5.0);
    h.tree.pointer(PointerInput::Down, MouseButton::Left, at, Mods::NONE, h.now());
    h.settle();
    assert_eq!(&*wrote.borrow(), b"\x1b[<0;4;2M");
}

#[test]
fn a_cell_width_spaces_the_glyphs_and_sets_the_columns() {
    let (mut h, s, look, _) = term(TermLook::default());
    s.feeder().feed(b"ab", std::time::Instant::now());
    h.settle();
    let xs = glyphs(&mut h);
    assert_eq!(xs[1].0 - xs[0].0, 6.0, "the advance of 0");
    look.update(|it| it.cell_width = Some(9.0));
    h.settle();
    assert_eq!(s.size().cols, 42, "(400 - 16) / 9");
    let xs = glyphs(&mut h);
    assert_eq!(xs[1].0 - xs[0].0, 9.0);
    h.tree.set_text_scale(2.0);
    h.settle();
    let xs = glyphs(&mut h);
    assert_eq!(xs[1].0 - xs[0].0, 18.0, "it grows with the text size");
}

#[test]
fn a_glyph_is_moved_to_the_contrast_unless_its_colour_was_named_exactly() {
    let (fg, bg) = (Color::rgb(0x303030), Color::rgb(0x202020));
    let colors = TermColors { foreground: Some(fg), background: Some(bg), ..TermColors::default() };
    let (mut h, s, look, _) = term(TermLook { colors, ..TermLook::default() });
    s.feeder().feed(b"a\x1b[38;2;48;48;48mb", std::time::Instant::now());
    h.settle();
    let drawn = glyphs(&mut h);
    assert_eq!((drawn[0].1, drawn[1].1), (fg, fg), "off: the colours as they are");
    look.update(|it| it.min_contrast = 4.5);
    h.settle();
    let drawn = glyphs(&mut h);
    assert!(contrast_ratio(drawn[0].1, bg) >= 4.5, "{:?}", drawn[0].1);
    assert_eq!(drawn[0].1, with_contrast(fg, bg, 4.5));
    assert_eq!(drawn[1].1, fg, "24-bit, as the program named it");
}

#[test]
fn fallback_families_reach_the_text_system_and_the_glyphs_are_shaped_again() {
    let (mut h, s, look, _) = term(TermLook::default());
    s.feeder().feed(b"a", std::time::Instant::now());
    h.settle();
    h.frame();
    let fonts = h.tree.fonts();
    fonts.borrow_mut().text.take_stats();
    look.update(|it| it.colors.cursor = Some(Color::rgb(0xFF0000)));
    h.settle();
    h.frame();
    assert_eq!(fonts.borrow_mut().text.take_stats().words_shaped, 0, "an unrelated change");
    let families = vec!["Symbols Nerd Font Mono".to_string()];
    look.update(|it| it.fallbacks = families.clone());
    h.settle();
    h.frame();
    assert_eq!(fonts.borrow().text.fallbacks(), &families[..]);
    assert!(fonts.borrow_mut().text.take_stats().words_shaped > 0, "the cells shaped again");
}

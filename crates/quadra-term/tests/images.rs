//! Images (specs/18, "Images (Sixel)"): a Sixel image is kept by the session, painted from the
//! colour atlas at its top-left cell, scrolls with its rows, and goes when the engine lets go of
//! it; the engine is told the cell's size in device pixels.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use quadra_reactive::signal;
use quadra_render::{Kind, Step};
use quadra_term::vt::ResizePolicy;
use quadra_term::{Session, TermLook, terminal};
use quadra_testing::Harness;
use quadra_view::column;

/// A terminal of 400 x 140 at size 10 (cells 6 x 14, padding 8): 8 rows of 64 columns.
fn term() -> (Harness, Session) {
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
    let s = slot.borrow_mut().take().unwrap();
    (h, s)
}

fn feed(h: &mut Harness, s: &Session, bytes: &[u8]) -> Vec<u8> {
    let replies = s.feeder().feed(bytes, Instant::now());
    h.settle();
    replies
}

/// A red image of 6 x 12 pixels: two bands of six columns.
const RED: &[u8] = b"\x1bPq#0;2;100;0;0#0~~~~~~-~~~~~~\x1b\\";

/// The pieces of images drawn: their bounds and clip, in device pixels.
fn sprites(h: &mut Harness) -> Vec<([f32; 4], [f32; 4])> {
    h.frame();
    let scene = h.tree.scene();
    let layers = h.renderer.steps().iter().filter_map(|step| match step {
        Step::Layer { id, .. } => Some(scene.layer(*id).instances()),
        _ => None,
    });
    layers.flatten().filter(|i| i.kind == Kind::Poly as u32).map(|i| (i.bounds, i.clip)).collect()
}

#[test]
fn the_engine_is_told_the_cell_in_device_pixels_and_advertises_sixel() {
    let (mut h, s) = term();
    assert_eq!((s.size().rows, s.size().cols), (8, 64));
    assert_eq!(feed(&mut h, &s, b"\x1b[14t"), b"\x1b[4;112;384t", "8 x 14 by 64 x 6");
    assert_eq!(feed(&mut h, &s, b"\x1b[c"), b"\x1b[?62;4;22c", "4 is Sixel");
}

#[test]
fn a_sixel_image_is_kept_and_painted_at_its_cell() {
    let (mut h, s) = term();
    feed(&mut h, &s, b"ab\r\n  ");
    feed(&mut h, &s, RED);
    assert_eq!(s.images(), [(6, 12)]);
    let drawn = sprites(&mut h);
    assert_eq!(drawn.len(), 1);
    assert_eq!(drawn[0].0, [8.0 + 2.0 * 6.0, 8.0 + 14.0, 6.0, 12.0], "row 2, column 3, its own size");
    assert_eq!(drawn[0].1, [8.0, 8.0, 384.0, 112.0], "clipped to the grid");
}

#[test]
fn it_scrolls_with_its_rows_and_its_part_in_view_shows() {
    let (mut h, s) = term();
    feed(&mut h, &s, b"\x1bPq#0;2;100;0;0#0~~~~~~-~~~~~~-~~~~~~-~~~~~~\x1b\\"); // 24 px: two rows
    feed(&mut h, &s, b"\x1b[8;1H\n");
    let drawn = sprites(&mut h);
    assert_eq!(drawn.len(), 1, "a row up, its first row above the grid");
    assert_eq!(drawn[0].0, [8.0, 8.0 - 14.0, 6.0, 24.0]);
    assert_eq!(drawn[0].1[1], 8.0, "cut at the grid's top");
}

#[test]
fn a_reset_releases_it_and_it_is_no_longer_painted() {
    let (mut h, s) = term();
    feed(&mut h, &s, RED);
    assert_eq!(s.images().len(), 1);
    feed(&mut h, &s, b"\x1bc");
    assert!(s.images().is_empty());
    assert!(sprites(&mut h).is_empty());
}

#[test]
fn scrolled_out_of_a_history_of_none_it_is_released() {
    let (mut h, s) = term();
    s.set_scrollback(0);
    feed(&mut h, &s, RED);
    feed(&mut h, &s, b"\x1b[8;1H\n\n");
    assert!(s.images().is_empty());
    assert!(sprites(&mut h).is_empty());
}

#[test]
fn an_image_released_before_the_view_took_it_never_reaches_it() {
    let (mut h, s) = term();
    s.feeder().feed(RED, Instant::now());
    s.feeder().feed(b"\x1bc", Instant::now());
    h.settle();
    assert!(s.images().is_empty());
}

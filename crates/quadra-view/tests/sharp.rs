//! A sharp scale (specs/05, "The pose"): a scaled box that draws what it holds again at the
//! scale it shows, its letters rasterized at the size they are seen; a move paints nothing,
//! a change of its scale paints it again, and a box scaled plainly is never painted again.

use quadra_prim::Point;
use quadra_reactive::signal;
use quadra_render::{FLAG_GLYPH, Step};
use quadra_testing::Harness;
use quadra_view::{div, text};

/// Each layer that has glyphs: how much its transform grows 10 px, and its tallest glyph.
fn layers(h: &Harness) -> Vec<(f32, f32)> {
    let mut steps = Vec::new();
    h.tree.scene().steps(quadra_prim::Rect::new(0.0, 0.0, 800.0, 600.0), &mut steps);
    let worlds = steps.iter().filter_map(|step| match step {
        Step::Layer { id, world } => Some((h.tree.scene().layer(*id), world.transform)),
        Step::Run { .. } => None,
    });
    let with_glyphs = worlds.filter_map(|(layer, transform)| {
        let glyphs = layer.instances().iter().filter(|i| i.flags & FLAG_GLYPH != 0);
        let tallest = glyphs.map(|i| i.bounds[3]).fold(0.0f32, f32::max);
        let grown = transform.apply(Point::new(10.0, 0.0)).x - transform.apply(Point::ZERO).x;
        (tallest > 0.0).then_some((grown, tallest))
    });
    with_glyphs.collect()
}

#[test]
fn a_sharp_box_draws_its_text_at_the_scale_it_shows_and_paints_again_only_when_it_changes() {
    let (sharp_k, plain_k, at) = (signal(2.0f32), signal(2.0f32), signal(0.0f32));
    let mut h = Harness::new();
    h.mount(move || {
        let words = |left: f32| div().absolute().left(left).top(100.0).size(80.0, 20.0).child(text("Hi"));
        let plain = words(20.0).scale(move || plain_k.get());
        let sharp = words(200.0).scale(move || sharp_k.get()).translate(move || Point::new(at.get(), 0.0));
        div().size(400.0, 400.0).child(plain).child(sharp.sharp_scale())
    });
    h.frame();
    // Plain: its transform doubles what it painted at its size. Sharp: painted twice as large,
    // its transform only a move. Both show the same size.
    let [(plain_grows, plain_glyph), (sharp_grows, sharp_glyph)] = layers(&h)[..] else {
        panic!("{:?}", layers(&h))
    };
    assert_eq!((plain_grows, sharp_grows), (20.0, 10.0));
    assert!((sharp_glyph - 2.0 * plain_glyph).abs() <= 1.0, "{sharp_glyph} {plain_glyph}");
    // A move paints nothing; neither does a plain scale; a sharp one paints its box again.
    at.set(5.0);
    let (moved, _) = h.frame();
    plain_k.set(1.5);
    let (plain, _) = h.frame();
    sharp_k.set(1.5);
    let (sharp, _) = h.frame();
    assert_eq!((moved.nodes_painted, plain.nodes_painted), (0, 0));
    assert!(sharp.nodes_painted >= 2, "{sharp:?}");
}

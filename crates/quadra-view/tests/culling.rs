//! A scroll container's content is painted only near what shows (specs/07, "Layers"): a long
//! page costs what its visible part costs, scrolling inside the painted band is one constant,
//! and leaving the band paints a new band once. Every frame is checked against a paint from
//! scratch by the harness.

use quadra_layout::Display;
use quadra_prim::Color;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_view::{column, div, mount_root, row, scroll, show, text};

const ROWS: u32 = 500;

/// What a range of rows emits: a box and the inked glyphs of "row N" each.
fn ink(rows: std::ops::RangeInclusive<u32>) -> u32 {
    rows.map(|i| 1 + 3 + i.to_string().len() as u32).sum()
}

/// 500 rows of 40 px in the 600 px window: 20,000 px of content, 15 rows showing.
fn page(h: &mut Harness, y: quadra_reactive::Signal<f32>, first: quadra_reactive::Signal<bool>) {
    let row = |i: u32| div().height(40.0).bg(Color::rgb(0x202020 + i)).child(text(format!("row {i}")));
    let top = show(move || first.get(), || div().height(400.0).child(text("above")));
    mount_root(&mut h.tree, scroll(y, column().child(top).children((0..ROWS).map(row))));
}

#[test]
fn a_long_page_paints_what_is_near_the_window_and_scrolls_inside_it_for_one_constant() {
    let (y, first) = (signal(0.0f32), signal(false));
    let mut h = Harness::new();
    page(&mut h, y, first);
    let (stats, _) = h.frame();
    // The window and its height below it: 30 rows, not 500.
    assert_eq!(stats.instances_written, ink(0..=29), "{stats:?}");

    y.set(100.0); // the bar shows: its thumb is painted, and nothing else
    assert_eq!(h.frame().0.nodes_painted, 1);
    y.set(500.0); // still inside the band: nothing is painted, nothing uploaded
    let (stats, render) = h.frame();
    assert_eq!((stats.nodes_painted, stats.layers_composed, render.instances_uploaded), (0, 2, 0));

    y.set(700.0); // rows past the band show: a new band, from 100 to 1900
    let (stats, _) = h.frame();
    // Row 2 crosses the band's edge with its box; the line of its text ends on the edge.
    assert_eq!((stats.layers_rebuilt, stats.instances_written), (1, ink(2..=47)), "{stats:?}");
    // Of the 500 rows and their texts, the rows are looked at, and the texts of those near
    // the old band and the new one.
    assert!(stats.nodes_walked < 600, "{stats:?}");
    y.set(900.0);
    assert_eq!(h.frame().0.nodes_painted, 0);

    y.set(19_400.0); // the end of the page
    let (stats, _) = h.frame();
    assert_eq!(stats.instances_written, ink(470..=499), "{stats:?}");
}

#[test]
fn what_layout_moves_into_the_band_is_painted() {
    let (y, first) = (signal(0.0f32), signal(true));
    let mut h = Harness::new();
    page(&mut h, y, first);
    let (stats, _) = h.frame();
    assert_eq!(stats.instances_written, 5 + ink(0..=19), "{stats:?}"); // "above", and 400 px of rows fewer
    first.set(false); // the part above goes: every row moves up, ten of them into the band
    h.frame(); // the harness compares with a paint from scratch
    first.set(true);
    y.set(300.0);
    h.frame();
}

#[test]
fn what_a_box_holds_is_painted_when_the_box_grows_into_the_band() {
    // The window shows 2000 to 2600: the band is 1400 to 3200. A box at 1300, as high as the
    // one beside it, holds one of 300 px that reaches into the band past it. The box beside
    // grows: the holder grows with it into the band, and back, by layout alone.
    let (y, high) = (signal(2000.0f32), signal(50.0f32));
    let mut h = Harness::new();
    let held = div().absolute().top(0.0).size(100.0, 300.0).bg(Color::rgb(0x336699));
    let beside = div().width(10.0).height(move || high.get());
    let line = row().child(beside).child(div().width(200.0).child(held));
    let page = column().child(div().height(1300.0)).child(line).child(div().height(5000.0));
    mount_root(&mut h.tree, scroll(y, page));
    h.frame();
    for to in [200.0, 50.0, 200.0] {
        high.set(to);
        h.frame(); // the harness compares with a paint from scratch
        let painted = h.renderer.steps().iter().any(|step| match step {
            quadra_render::Step::Layer { id, .. } => {
                h.tree.scene().layer(*id).instances().iter().any(|i| i.bounds[3] == 300.0)
            }
            quadra_render::Step::Run { .. } => false,
        });
        assert_eq!(painted, to == 200.0, "held painted at {to}");
    }
}

#[test]
fn a_box_shown_again_outside_the_band_keeps_what_it_holds_in_place() {
    // A box at the top holds many glyphs, then an empty box. Hidden, then scrolled out of the
    // band, then shown: the empty box is not painted, and the layer is rebuilt without it.
    let (hidden, y) = (signal(false), signal(0.0f32));
    let mut h = Harness::new();
    let shown = move || if hidden.get() { Display::None } else { Display::Flex };
    let held = column().display(shown).child(text("x".repeat(400))).child(div().width(10.0));
    let rows = (0..200).map(|i| div().height(40.0).child(text(format!("{i}"))));
    mount_root(&mut h.tree, scroll(y, column().child(held).children(rows)));
    h.frame();
    hidden.set(true);
    h.frame();
    y.set(4000.0);
    h.frame();
    hidden.set(false);
    h.frame();
}

#[test]
fn a_layer_shown_again_stands_where_moves_left_it() {
    // A layer in a row of no height is hidden; the box above it grows while the window shows
    // the end of the page; it shows again where it now is.
    let (above, hidden, y) = (signal(30.0f32), signal(false), signal(0.0f32));
    let mut h = Harness::new();
    let shown = move || if hidden.get() { Display::None } else { Display::Flex };
    let layer = row().cut_to_parent().display(shown).child(text(""));
    let rows = (0..200).map(|i| div().height(40.0).child(text(format!("{i}"))));
    let page = column().child(div().height(above)).child(div().height(0.0).child(layer)).children(rows);
    mount_root(&mut h.tree, scroll(y, page));
    h.frame();
    hidden.set(true);
    h.frame();
    y.set(4000.0);
    h.frame();
    above.set(100.0);
    h.frame();
    hidden.set(false);
    h.frame(); // the harness checks the layer's place against a paint from scratch
}

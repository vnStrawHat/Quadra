//! A scroll container's content is painted only near what shows (specs/07, "Layers"): a long
//! page costs what its visible part costs, scrolling inside the painted band is one constant,
//! and leaving the band paints a new band once. Every frame is checked against a paint from
//! scratch by the harness.

use quadra_prim::Color;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_view::{column, div, mount_root, scroll, show, text};

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

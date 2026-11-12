//! The strip (specs/06, "Scrolling"): a row that scrolls sideways under the wheel, as wide
//! as its viewport at least; and the box that scrolls sideways in a page.

use quadra_platform::Mods;
use quadra_prim::Point;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_ui::{column, div, mount_root, row, scroll, scroll_strip, scroll_x};

#[test]
fn a_strip_scrolls_sideways_under_the_wheel_and_passes_on_what_it_cannot_scroll() {
    let (x, page_y) = (signal(0.0f32), signal(0.0f32));
    let mut h = Harness::new();
    // Ten tabs of 50 px in a strip of 200, on a page that scrolls.
    let tabs = row().children((0..10).map(|_| div().size(50.0, 30.0).shrink(0.0)).collect::<Vec<_>>());
    let strip = scroll_strip(x, tabs).container(div().size(200.0, 30.0));
    mount_root(&mut h.tree, scroll(page_y, column().child(strip).child(div().size(200.0, 2000.0))));
    h.frame();
    let viewport = h.tree.children(h.tree.children(h.tree.root().unwrap())[0])[0];
    let content = h.tree.children(viewport)[0];
    assert_eq!((h.tree.rect(viewport).width, h.tree.rect(content).width), (200.0, 500.0));

    // The plain wheel scrolls it: the content moves by its layer, nothing is laid out or painted.
    h.wheel(50.0, 10.0, -1.0);
    assert_eq!((x.get_untracked(), page_y.get_untracked()), (40.0, 0.0));
    let (stats, _) = h.frame();
    assert_eq!((stats.nodes_laid_out, stats.nodes_painted, h.tree.window_rect(content).x), (0, 0, -40.0));

    // Never past its end; there, the wheel is the page's.
    h.wheel(50.0, 10.0, -20.0);
    assert_eq!((x.get_untracked(), page_y.get_untracked()), (300.0, 0.0));
    h.frame();
    h.wheel(50.0, 10.0, -1.0);
    assert_eq!((x.get_untracked(), page_y.get_untracked()), (300.0, 40.0));
}

#[test]
fn what_a_strip_holds_is_as_wide_as_the_strip_at_least() {
    let mut h = Harness::new();
    let tabs = row().child(div().size(50.0, 30.0)).child(div().grow(1.0));
    mount_root(&mut h.tree, scroll_strip(signal(0.0), tabs).container(div().size(200.0, 30.0)));
    h.frame();
    let content = h.tree.children(h.tree.root().unwrap())[0];
    let rest = h.tree.children(content)[1];
    assert_eq!((h.tree.rect(content).width, h.tree.rect(rest).width), (200.0, 150.0));
}

#[test]
fn a_box_that_scrolls_sideways_leaves_the_plain_wheel_to_the_page() {
    let (x, page_y) = (signal(0.0f32), signal(0.0f32));
    let mut h = Harness::new();
    // A line of code of 500 px in a box of 200, on a page that scrolls.
    let code = scroll_x(x, div().size(500.0, 30.0).shrink(0.0)).container(div().size(200.0, 30.0));
    mount_root(&mut h.tree, scroll(page_y, column().child(code).child(div().size(200.0, 2000.0))));
    h.frame();
    let wheel = |h: &mut Harness, delta: Point, mods: Mods| {
        h.tree.wheel(delta, false, Point::new(50.0, 10.0), mods);
        h.frame();
        (x.get_untracked(), page_y.get_untracked())
    };
    // With Shift, or turned sideways, the wheel is the box's; the plain wheel is the page's.
    assert_eq!(wheel(&mut h, Point::new(0.0, -1.0), Mods::SHIFT), (40.0, 0.0));
    assert_eq!(wheel(&mut h, Point::new(-1.0, 0.0), Mods::NONE), (80.0, 0.0));
    assert_eq!(wheel(&mut h, Point::new(0.0, -1.0), Mods::NONE), (80.0, 40.0));
}

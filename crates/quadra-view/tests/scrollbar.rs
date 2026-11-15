//! The scrollbar (specs/06, "Scrolling"): every scroll view has a thumb that says where the
//! viewport is in the content. It shows while something goes on, fades after a second of
//! rest, and scrolls the content when it is dragged.

use std::time::Duration;

use quadra_prim::Rect;
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{NodeId, PointerInput};
use quadra_view::{column, div, scroll, text};

/// 3000 px of content in the 800 x 600 window, and the thumb of its bar.
fn page(height: f32) -> (Harness, Signal<f32>, NodeId) {
    let y = signal(0.0f32);
    let mut h = Harness::new();
    h.mount(move || scroll(y, column().child(div().height(height).child(text("content")))));
    h.frame();
    let thumb = h.tree.children(h.tree.root().unwrap())[1];
    (h, y, thumb)
}

#[test]
fn the_thumb_is_as_long_as_the_viewport_is_of_the_content_and_goes_with_it() {
    let (mut h, y, thumb) = page(3000.0);
    // A fifth of the track of 592 px, at its start: the grip is the 14 px at the edge.
    assert_eq!(h.tree.window_rect(thumb), Rect::new(786.0, 4.0, 14.0, 118.4));
    y.set(1200.0); // half of the 2400 there are to scroll
    h.frame();
    assert_eq!(h.tree.window_rect(thumb).y, 4.0 + (592.0 - 118.4) / 2.0);
    y.set(2400.0);
    h.frame();
    assert!((h.tree.window_rect(thumb).bottom() - 596.0).abs() < 0.01);

    // Content that fits has no thumb; very long content has one of 25 px.
    assert_eq!(page(500.0).0.tree.rect(page(500.0).2).height, 0.0);
    let (long, _, thumb) = page(1.0e6);
    assert_eq!(long.tree.rect(thumb).height, 25.0);
}

#[test]
fn the_bar_shows_while_something_goes_on_and_fades_after_a_second() {
    let (mut h, y, _) = page(3000.0);
    assert_eq!(h.frame().1.instances_uploaded, 0); // at rest there is no thumb to see
    h.pointer(PointerInput::Move, 100.0, 100.0);
    let (stats, drawn) = h.frame();
    assert_eq!((stats.nodes_painted, drawn.instances_uploaded), (1, 1)); // it shows at once
    assert!(h.frame_if_needed().is_none()); // and asks for no frame while it only shows

    h.advance(Duration::from_millis(900));
    y.set(40.0); // scrolled: the second of rest starts again
    h.frame();
    h.advance(Duration::from_millis(900));
    assert!(h.frame_if_needed().is_none());
    h.advance(Duration::from_millis(100)); // a second after the last thing: it fades
    let mut painted = Vec::new();
    while let Some((stats, _)) = h.frame_if_needed() {
        assert_eq!(stats.nodes_laid_out, 0);
        painted.push(stats.nodes_painted);
        h.advance(Duration::from_millis(100));
    }
    // The frame the fade starts on, then 400 ms of it: the thumb alone is painted, fainter.
    assert_eq!(painted, [0, 1, 1, 1, 1]);
    assert_eq!(h.frame().1.instances_uploaded, 0);
}

#[test]
fn dragging_the_thumb_scrolls_the_content_in_proportion() {
    let (mut h, y, thumb) = page(3000.0);
    let travel = 592.0 - 118.4;
    h.pointer(PointerInput::Move, 793.0, 50.0);
    h.pointer(PointerInput::Down, 793.0, 50.0);
    h.pointer(PointerInput::Move, 400.0, 50.0 + travel / 4.0); // wherever the pointer goes
    h.frame();
    assert!((y.get_untracked() - 600.0).abs() < 0.01, "{}", y.get_untracked());
    h.pointer(PointerInput::Move, 400.0, 5000.0); // past the end: the content stops at its end
    assert_eq!(y.get_untracked(), 2400.0);
    h.pointer(PointerInput::Up, 400.0, 5000.0);
    h.pointer(PointerInput::Move, 400.0, 100.0); // let go: a move scrolls nothing
    h.frame();
    assert_eq!((y.get_untracked(), h.tree.window_rect(thumb).bottom().round()), (2400.0, 596.0));
}

#[test]
fn a_thumb_held_still_stays_and_fades_a_second_after_it_is_let_go() {
    let (mut h, _, _) = page(3000.0);
    h.pointer(PointerInput::Move, 793.0, 50.0);
    h.pointer(PointerInput::Down, 793.0, 50.0);
    h.frame();
    // Three seconds held still: nothing fades.
    for _ in 0..30 {
        h.advance(Duration::from_millis(100));
        if let Some((stats, _)) = h.frame_if_needed() {
            assert_eq!(stats.nodes_painted, 0);
        }
    }
    h.pointer(PointerInput::Up, 793.0, 50.0);
    h.frame();
    h.advance(Duration::from_millis(900));
    assert!(h.frame_if_needed().is_none()); // still within the second after the release
    h.advance(Duration::from_millis(200));
    let mut painted = 0;
    while let Some((stats, _)) = h.frame_if_needed() {
        painted += stats.nodes_painted;
        h.advance(Duration::from_millis(100));
    }
    assert!(painted > 0); // and then it fades
}

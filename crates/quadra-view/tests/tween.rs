//! A value that eases to its target (specs/05): it follows the clock of its window, starts
//! from where it is when the target changes, and asks for no frame once it has arrived.

use std::time::Duration;

use quadra_prim::Ease;
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::NodeId;
use quadra_view::{column, div, tween};

/// A panel 10 px high whose width goes to 200 or to 0 in 200 ms, at an even speed.
fn panel() -> (Harness, Signal<bool>, NodeId) {
    let open = signal(true);
    let mut h = Harness::new();
    h.mount(move || {
        let width =
            tween(move || if open.get() { 200.0 } else { 0.0 }, Duration::from_millis(200), Ease::Linear);
        column().child(div().height(10.0).width(width))
    });
    h.frame();
    let panel = h.tree.children(h.tree.root().unwrap())[0];
    (h, open, panel)
}

#[test]
fn a_value_goes_to_its_target_by_the_clock_and_then_rests() {
    let (mut h, open, panel) = panel();
    let width = |h: &Harness| h.tree.rect(panel).width;
    assert_eq!(width(&h), 200.0); // it starts where its target is
    assert!(h.frame_if_needed().is_none());

    open.set(false);
    h.advance(Duration::from_millis(100)); // no frame was drawn meanwhile: time passed all the same
    h.frame();
    assert_eq!(width(&h), 100.0);
    assert!(h.tree.animating()); // every frame, until it is there
    h.advance(Duration::from_millis(150));
    h.frame();
    assert_eq!(width(&h), 0.0);
    h.advance(Duration::from_secs(1));
    assert!(h.frame_if_needed().is_none()); // arrived: the window sleeps

    // A target that changes on the way is gone to from where the value has got to.
    open.set(true);
    h.advance(Duration::from_millis(50));
    h.frame();
    assert_eq!(width(&h), 50.0);
    open.set(false);
    h.advance(Duration::from_millis(100));
    h.frame();
    assert_eq!(width(&h), 25.0); // half of the way back from 50
    h.advance(Duration::from_millis(100));
    h.frame();
    assert_eq!(width(&h), 0.0);
    assert!(h.frame_if_needed().is_none());
}

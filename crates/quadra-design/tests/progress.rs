//! What says that something is under way (specs/13, "Components"): the progress bar, and
//! the three that move. What moves is a layer: a frame of them lays out and paints nothing.

use std::rc::Rc;
use std::time::Duration;

use quadra_design::{dark, progress, progress_busy, skeleton, spinner};
use quadra_prim::Size;
use quadra_reactive::signal;
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::{View, column};

fn mount<V: View + 'static>(build: impl FnOnce() -> V + 'static) -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || quadra_design::root().child(build()));
    h.frame();
    h
}

#[test]
fn a_progress_bar_is_filled_as_far_as_its_part() {
    let done = signal(0.42f32);
    let mut h = mount(move || column().child(progress(move || done.get())));
    assert!(h.semantics().contains("ProgressBar = \"42%\""), "{}", h.semantics());
    let bar = h.query(|n| n.semantics.role == Role::ProgressBar)[0];
    let fill = h.tree.children(bar)[0];
    assert_eq!((h.tree.rect(bar).size(), h.tree.rect(fill).width), (Size::new(200.0, 6.0), 84.0));
    done.set(1.5); // more than all of it is all of it
    let (stats, _) = h.frame();
    assert_eq!((h.tree.rect(fill).width, stats.nodes_laid_out <= 2), (200.0, true));
    assert!(h.semantics().contains("ProgressBar = \"100%\""));
    assert!(h.frame_if_needed().is_none()); // at rest it asks for no frame
}

#[test]
fn what_moves_is_composed_and_neither_laid_out_nor_painted() {
    let mut h =
        mount(|| column().gap(8.0).child(progress_busy()).child(spinner()).child(skeleton(120.0, 16.0)));
    let tree = h.semantics();
    assert!(tree.contains("ProgressBar \"Busy\"") && tree.contains("ProgressBar \"Loading\""), "{tree}");
    let (busy, loading) =
        (h.get_by_role(Role::ProgressBar, "Busy").node(), h.get_by_role(Role::ProgressBar, "Loading").node());
    let (piece, ring) = (h.tree.children(busy)[0], h.tree.children(loading)[0]);
    assert_eq!((h.tree.rect(piece).width, h.tree.rect(ring).size()), (70.0, Size::new(16.0, 16.0)));
    h.frame(); // the first frame of the three: where they are at the start
    let mut xs = Vec::new();
    for _ in 0..4 {
        h.advance(Duration::from_millis(350));
        let (stats, drawn) = h.frame();
        // The piece that crosses, the ring that turns, the tone that fades: three constants.
        assert_eq!((stats.nodes_laid_out, stats.nodes_painted, drawn.instances_uploaded), (0, 0, 0));
        assert_eq!(stats.layers_composed, 3);
        xs.push(h.tree.window_rect(piece).x.round());
    }
    // It comes in from the left, crosses in 1.4 s, and starts again.
    assert_eq!(xs, [-70.0 + 270.0 * 0.25, -70.0 + 270.0 * 0.5, -70.0 + 270.0 * 0.75, -70.0].map(f32::round));
    assert!(h.frame_if_needed().is_some()); // while they are there, every frame is drawn
}

#[test]
fn a_meter_says_its_value_and_its_mark_and_warns_near_the_top() {
    let (used, requested) = (signal(0.43f32), signal(0.61f32));
    let mut h = mount(move || {
        let cpu = quadra_design::meter(move || used.get()).mark(move || requested.get());
        column().child(cpu.names("used", "requested").warn_at(0.85).height(4.0))
    });
    assert!(h.semantics().contains("ProgressBar = \"used 43%, requested 61%\""), "{}", h.semantics());
    let bar = h.query(|n| n.semantics.role == Role::ProgressBar)[0];
    let (fill, holder) = (h.tree.children(bar)[0], h.tree.children(bar)[1]);
    assert_eq!((h.tree.rect(bar).size(), h.tree.rect(fill).width), (Size::new(200.0, 4.0), 86.0));
    // The tick is centred on the mark, a pixel past the track above and below.
    let tick = h.tree.window_rect(h.tree.children(holder)[0]);
    let track = h.tree.window_rect(bar);
    assert_eq!((tick.center().x - track.x, tick.height), (122.0, 6.0));
    // The fill warns from 85 %, and is an error past all of it.
    let tones: Vec<_> = [0.43, 0.9, 1.2]
        .into_iter()
        .map(|part| {
            used.set(part);
            h.frame();
            h.tree.paint_style(fill).background
        })
        .collect();
    assert!(tones[0] != tones[1] && tones[1] != tones[2] && tones[0] != tones[2], "{tones:?}");
    assert_eq!(h.tree.rect(fill).width, 200.0);
}

//! The pose and transitions (specs/05): what a box paints is moved, scaled, turned and faded
//! over the box it keeps, and a change of its colours or its pose takes a time, by the
//! window's clock, with no code of the app's running.

use std::rc::Rc;
use std::time::Duration;

use quadra_prim::{Color, Ease, Point, Rect};
use quadra_reactive::signal;
use quadra_render::Step;
use quadra_testing::Harness;
use quadra_tree::{NodeId, PointerInput, Theme, Token};
use quadra_ui::{column, div, timeline};

const RED: Color = Color::rgb(0xFF0000);
const BLUE: Color = Color::rgb(0x0000FF);

fn mounted(h: &mut Harness) -> Vec<NodeId> {
    h.frame();
    h.tree.children(h.tree.root().unwrap()).to_vec()
}

fn pass(h: &mut Harness, ms: u64) -> (quadra_tree::FrameStats, quadra_render::RenderStats) {
    h.advance(Duration::from_millis(ms));
    h.frame()
}

#[test]
fn a_pose_moves_what_is_painted_and_keeps_the_box() {
    let mut h = Harness::new();
    h.mount(|| {
        let dot = div().absolute().left(0.0).top(0.0).size(10.0, 10.0).bg(RED);
        let moved = div().absolute().left(100.0).top(100.0).size(40.0, 40.0);
        let moved = moved.translate(Point::new(10.0, 0.0)).scale(2.0).opacity(0.5);
        div().size(400.0, 400.0).child(moved.child(dot))
    });
    let moved = mounted(&mut h)[0];
    // Scaled by two about its middle, (120, 120), then moved 10 px right: its corner is at
    // (90, 80), and what is 10 px in is 20 px from there.
    let mut steps = Vec::new();
    h.tree.scene().steps(Rect::new(0.0, 0.0, 800.0, 600.0), &mut steps);
    let mut worlds = steps.iter().filter_map(|step| match step {
        Step::Layer { world, .. } => Some(*world),
        Step::Run { .. } => None,
    });
    let own = worlds.next_back().expect("the box has a layer of its own, and something in it");
    assert_eq!(own.transform.apply(Point::ZERO), Point::new(90.0, 80.0));
    assert_eq!((own.transform.apply(Point::new(10.0, 10.0)), own.opacity), (Point::new(110.0, 100.0), 0.5));
    // The box is where it was laid out, and that is where the pointer finds it.
    assert_eq!(h.tree.window_rect(moved), Rect::new(100.0, 100.0, 40.0, 40.0));
    assert_eq!((h.tree.hit_test(Point::new(139.0, 139.0)), h.tree.hit_test(Point::new(95.0, 95.0))), {
        (Some(moved), h.tree.root())
    });
}

#[test]
fn a_change_of_colour_and_pose_takes_its_time() {
    let on = signal(true);
    let mut h = Harness::new();
    h.mount(move || {
        let card = div().size(50.0, 50.0).bg(RED).transition_with(100, Ease::Linear);
        let card = card.hover(|s| s.bg(BLUE).translate(Point::new(0.0, -4.0)));
        // A colour of a signal's: the one it has when the box is mounted shows at once.
        let lamp = div().size(50.0, 50.0).bg(move || if on.get() { RED } else { BLUE }).transition(100);
        column().child(card).child(lamp)
    });
    let nodes = mounted(&mut h);
    let (card, lamp) = (nodes[0], nodes[1]);
    let look = |h: &Harness| (h.tree.paint_style(card).background, h.tree.pose(card).translate.y);
    assert_eq!((look(&h), h.tree.paint_style(lamp).background), ((RED, 0.0), RED));
    assert!(h.frame_if_needed().is_none());

    // The pointer comes over the card: nothing shows yet, and the window wants frames.
    h.pointer(PointerInput::Move, 25.0, 25.0);
    h.frame();
    assert_eq!((look(&h), h.tree.needs_frame()), ((RED, 0.0), true));
    let (stats, _) = pass(&mut h, 50);
    assert_eq!(look(&h), (RED.lerp(BLUE, 0.5), -2.0));
    // A frame of it: the card styled and painted, its layer composed, nothing laid out.
    assert_eq!(
        (stats.nodes_styled, stats.nodes_painted, stats.nodes_laid_out, stats.layers_composed),
        (1, 1, 0, 1)
    );
    // The card moved under the pointer and the pointer is still on it: the box did not move.
    assert_eq!((h.tree.window_rect(card).y, h.tree.is_hovered(card)), (0.0, true));

    // The pointer leaves half way: back from where it is, in the same time.
    h.pointer(PointerInput::Move, 300.0, 300.0);
    h.frame();
    pass(&mut h, 50);
    let half = RED.lerp(BLUE, 0.5);
    assert_eq!(look(&h), (half.lerp(RED, 0.5), -1.0));
    pass(&mut h, 60);
    assert_eq!(look(&h), (RED, 0.0));
    assert!(h.frame_if_needed().is_none()); // arrived: no more frames

    // A signal's colour eases too, once the box has shown.
    on.set(false);
    h.frame();
    pass(&mut h, 200);
    assert_eq!(h.tree.paint_style(lamp).background, BLUE);
}

#[test]
fn a_pose_that_eases_paints_nothing_and_a_new_theme_shows_at_once() {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(Theme::new(vec![RED])));
    h.mount(|| {
        let card = div().size(50.0, 50.0).bg(Token(0)).transition(100);
        column().child(card.hover(|s| s.scale(1.1).rotate(6.0)).active(|s| s.opacity(0.5)))
    });
    let card = mounted(&mut h)[0];
    h.pointer(PointerInput::Move, 25.0, 25.0);
    let (stats, drawn) = h.frame();
    assert_eq!((stats.nodes_styled, stats.nodes_painted, drawn.instances_uploaded), (1, 0, 0));
    for _ in 0..4 {
        let (stats, drawn) = pass(&mut h, 20);
        assert_eq!((stats.nodes_styled, stats.nodes_painted, drawn.instances_uploaded), (0, 0, 0));
        assert_eq!(stats.layers_composed, 1);
    }
    pass(&mut h, 30);
    assert_eq!((h.tree.pose(card).scale, h.tree.pose(card).rotate), (1.1, 6.0));

    h.tree.set_theme(Rc::new(Theme::new(vec![BLUE])));
    h.frame();
    assert_eq!((h.tree.paint_style(card).background, h.tree.needs_frame()), (BLUE, false));
}

#[test]
fn a_timeline_plays_its_steps_by_the_clock_and_paints_nothing() {
    let mut h = Harness::new();
    h.mount(|| {
        let up = |s: quadra_ui::Refine| s.translate(Point::new(0.0, -20.0));
        let hop = timeline().to(100, Ease::Linear, up).to(100, Ease::Linear, |s| s.translate(Point::ZERO));
        let turn = timeline().to(200, Ease::Linear, |s| s.rotate(360.0)).forever();
        let pulse = timeline().to(100, Ease::Linear, |s| s.scale(2.0).opacity(0.5)).pause(100);
        let dot = || div().size(20.0, 20.0).bg(RED);
        let hops = dot().animate(hop.forever());
        column().child(hops).child(dot().animate(turn)).child(dot().animate(pulse.repeat(2).alternate()))
    });
    let nodes = mounted(&mut h);
    // Where the three are: how far up, how far round, how large.
    let look = |h: &Harness| {
        let pose = |i: usize| h.tree.pose(nodes[i]);
        [pose(0).translate.y, pose(1).rotate, pose(2).scale, pose(2).opacity]
    };
    let near = |is: [f32; 4], wanted: [f32; 4]| {
        assert!(
            is.iter().zip(wanted).all(|(is, wanted)| (is - wanted).abs() < 1e-3),
            "{is:?} is not {wanted:?}"
        );
    };
    near(look(&h), [0.0, 0.0, 1.0, 1.0]);
    let (stats, drawn) = pass(&mut h, 50);
    near(look(&h), [-10.0, 90.0, 1.5, 0.75]);
    // A frame of the three: their layers composed, nothing styled, laid out or painted.
    assert_eq!((stats.layers_composed, stats.nodes_styled, stats.nodes_laid_out), (3, 0, 0));
    assert_eq!((stats.nodes_painted, drawn.instances_uploaded), (0, 0));
    // The second step brings the first back; the third waits, as large as it got.
    pass(&mut h, 100);
    near(look(&h), [-10.0, 270.0, 2.0, 0.5]);
    // Again from the start, each from what its style says; the third goes backwards now.
    pass(&mut h, 100);
    near(look(&h), [-10.0, 90.0, 2.0, 0.5]);
    pass(&mut h, 100);
    near(look(&h), [-10.0, 270.0, 1.5, 0.75]);
    // Two plays and the third is over, back where it started; the others go on.
    pass(&mut h, 100);
    near(look(&h), [-10.0, 90.0, 1.0, 1.0]);
    assert!(h.tree.animating());
}

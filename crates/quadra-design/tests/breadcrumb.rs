//! The breadcrumb (specs/13, "The bars"): one line however narrow its box, its middle parts
//! cut first, each with an ellipsis, the first and the last whole while they fit.

use std::rc::Rc;

use quadra_design::{breadcrumb, dark};
use quadra_prim::Rect;
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::Role;
use quadra_view::div;

const PARTS: [&str; 4] = ["Cluster", "Workloads", "Deployments", "api-gateway"];

fn way(width: Signal<f32>) -> Harness {
    let mut h = Harness::new();
    h.tree.set_theme(Rc::new(dark().theme()));
    h.mount(move || div().width(move || width.get()).min_width(0.0).child(breadcrumb(PARTS, |_| ())));
    h.frame();
    h
}

/// Each part's box, the last (where the user is) by its text.
fn parts(h: &mut Harness) -> Vec<Rect> {
    let links = PARTS[..3].iter().map(|name| h.get_by_role(Role::Link, name).node()).collect::<Vec<_>>();
    let last = h.get_by_text(PARTS[3]).node();
    links.into_iter().chain([last]).map(|node| h.tree.window_rect(node)).collect()
}

#[test]
fn a_breadcrumb_keeps_one_line_and_cuts_its_middle_parts_first() {
    let width = signal(800.0);
    let mut h = way(width);
    let whole = parts(&mut h);
    let line = whole[0].height;
    // Narrower than it needs: the middle parts give way, the first and the last stay whole.
    let needs = whole[3].right() - whole[0].x;
    width.set(needs - 60.0);
    h.frame();
    let cut = parts(&mut h);
    assert!(cut.iter().all(|r| r.height == line), "{cut:?}");
    assert!(cut[3].right() <= needs - 60.0 + 0.01, "{cut:?}");
    let kept = |a: f32, b: f32| (a - b).abs() < 0.01;
    assert!(kept(cut[0].width, whole[0].width) && kept(cut[3].width, whole[3].width), "{cut:?}");
    assert!(cut[1].width < whole[1].width && cut[2].width < whole[2].width, "{cut:?}");
    // Narrower still: all of them give way, and it is still one line.
    width.set(160.0);
    h.frame();
    let cut = parts(&mut h);
    assert!(cut.iter().all(|r| r.height == line) && cut[3].right() <= 160.01, "{cut:?}");
    assert!(cut[0].width < whole[0].width && cut[3].width < whole[3].width, "{cut:?}");
}

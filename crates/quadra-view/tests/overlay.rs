//! Overlays (specs/05, specs/06): content above the main tree, placed against an anchor, hit
//! before what is under it, told of presses outside it, and closed with the part of the view
//! that opened it.

use quadra_platform::{Key, Mods};
use quadra_prim::{Color, Point, Rect};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{Along, Side};
use quadra_view::{NodeRef, Place, column, div, mount_root, overlay, show};

struct Page {
    h: Harness,
    open: Signal<bool>,
    picked: Signal<i32>,
    top: Signal<f32>,
}

/// A button 100 px down the window, and a menu of three items that opens below it while
/// `open` is true. An item sets `picked`; a press outside the menu closes it.
fn page() -> Page {
    let (open, picked, top) = (signal(false), signal(0), signal(100.0f32));
    let button = NodeRef::new();
    let anchor = button.clone();
    let menu = move || {
        let item = |n: i32| {
            let item = div().size(120.0, 24.0).focusable().hover(|s| s.bg(Color::rgb(0xDCE6FF)));
            item.on_click(move |_| picked.set(n))
        };
        let items = column().bg(Color::rgb(0xF0F0F0)).focus_scope().children((1..=3).map(item));
        let place = Place::Beside(anchor.clone(), Side::Below, Along::Start, 4.0);
        overlay(place, items).on_outside_press(move || open.set(false))
    };
    let spacer = div().height(top);
    let trigger = div().size(80.0, 20.0).node_ref(&button).on_click(move |_| open.update(|o| *o = !*o));
    let mut h = Harness::new();
    mount_root(&mut h.tree, column().child(spacer).child(trigger).child(show(move || open.get(), menu)));
    h.frame();
    Page { h, open, picked, top }
}

impl Page {
    fn menu(&self) -> Option<Rect> {
        self.h.tree.overlays().next().map(|root| self.h.tree.window_rect(root))
    }
}

#[test]
fn an_overlay_opens_beside_its_anchor_above_the_main_tree_and_closes_with_its_part() {
    let mut p = page();
    assert_eq!((p.menu(), p.h.tree.scene().roots().len()), (None, 1));
    p.h.click(40.0, 110.0); // the button
    p.h.frame();
    assert_eq!(p.menu(), Some(Rect::new(0.0, 124.0, 120.0, 72.0))); // 4 px below it
    assert_eq!(p.h.tree.scene().roots().len(), 2); // its own layer, drawn after the main tree's

    p.h.click(60.0, 160.0); // the second item: the overlay is hit before what is under it
    assert_eq!((p.picked.get_untracked(), p.open.get_untracked()), (2, true));
    // The pointer is over that item: its hover style shows, painted in the overlay's layer.
    let (stats, _) = p.h.frame();
    let item = p.h.tree.children(p.h.tree.overlays().next().unwrap())[1];
    assert_eq!(p.h.tree.paint_style(item).background, Color::rgb(0xDCE6FF));
    assert_eq!(
        (stats.nodes_styled, stats.nodes_painted, stats.layers_patched + stats.layers_rebuilt),
        (1, 1, 1)
    );
    p.h.click(400.0, 400.0); // outside: the menu is told, and closes
    p.h.frame();
    assert_eq!((p.menu(), p.h.tree.scene().roots().len(), p.picked.get_untracked()), (None, 1, 2));
    p.h.click(60.0, 160.0); // nothing is there any more
    assert_eq!(p.picked.get_untracked(), 2);
    assert!(p.h.frame_if_needed().is_none());
}

#[test]
fn an_overlay_follows_its_anchor_and_opens_where_it_fits() {
    let mut p = page();
    p.open.set(true);
    p.h.frame();
    p.top.set(300.0); // the button moves down: the menu moves with it, and is not painted again
    let (stats, _) = p.h.frame();
    assert_eq!(p.menu().map(|m| m.y), Some(324.0));
    let layers_painted = stats.layers_patched + stats.layers_rebuilt;
    assert!(layers_painted <= 1 && stats.layers_composed == 1, "{stats:?}"); // the main tree's, and one constant
    p.top.set(560.0); // no room below a button at the bottom of the window: it opens above
    p.h.frame();
    assert_eq!(p.menu().map(|m| m.y), Some(560.0 - 4.0 - 72.0));
}

#[test]
fn a_focus_scope_in_an_overlay_keeps_tab_inside_and_a_point_or_the_centre_can_anchor() {
    let mut p = page();
    p.open.set(true);
    p.h.frame();
    let root = p.h.tree.overlays().next().unwrap();
    let items = p.h.tree.children(root).to_vec();
    p.h.tree.focus(items[0]);
    let mut seen = Vec::new();
    for _ in 0..4 {
        p.h.key(Key::Tab, Mods::NONE);
        seen.push(p.h.tree.focused().unwrap());
    }
    assert_eq!(seen, [items[1], items[2], items[0], items[1]]);

    let mut h = Harness::new();
    let dialog = overlay(Place::Center, div().size(200.0, 100.0));
    let context = overlay(Place::At(Point::new(700.0, 550.0)), div().size(150.0, 120.0));
    mount_root(&mut h.tree, column().child(dialog).child(context));
    h.frame();
    let rects: Vec<Rect> = h.tree.overlays().map(|root| h.tree.window_rect(root)).collect();
    // In the middle of the 800 x 600 window; and up and left of a point near its corner.
    assert_eq!(rects, [Rect::new(300.0, 250.0, 200.0, 100.0), Rect::new(550.0, 430.0, 150.0, 120.0)]);
    assert_eq!(h.tree.hit_test(Point::new(600.0, 500.0)), h.tree.overlays().last()); // the last opened is on top
}

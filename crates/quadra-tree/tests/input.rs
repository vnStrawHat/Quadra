//! Pointer input over the tree (specs/09): hit testing reads what layout left, an event goes
//! down its path and back up, hover follows the pointer and the content under it, a click is
//! a press and a release that share a node.

mod common;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use common::Window;
use quadra_layout::{Direction, Overflow};
use quadra_platform::{Mods, MouseButton};
use quadra_prim::Point;
use quadra_tree::{NodeId, PointerEvent, PointerInput, PointerKind, Tree};

type Log = Rc<RefCell<Vec<String>>>;

struct Page {
    w: Window,
    log: Log,
    now: Instant,
    card: NodeId,
    button: NodeId,
    rows: Vec<NodeId>,
    content: NodeId,
}

/// A 200 x 100 card with a 60 x 20 button at its corner, then a 200 x 100 viewport scrolling
/// 50 rows of 20 px. The window is 800 x 600: the rest of it is the root.
fn page() -> Page {
    let mut w = Window::new();
    let button = w.block(60.0, 20.0, 0x808080);
    let card = w.block(200.0, 100.0, 0x202020);
    w.tree.set_children(card, &[button]);
    let rows: Vec<_> = (0..50).map(|i| w.block(180.0, 20.0, 0x300000 + i)).collect();
    let content = w.column(&rows);
    w.tree.make_layer(content, true);
    let viewport = w.block(200.0, 100.0, 0x101010);
    let scrolls = quadra_layout::Style {
        direction: Direction::Column,
        overflow_y: Overflow::Scroll,
        ..w.tree.layout_style(viewport).clone()
    };
    w.tree.set_layout_style(viewport, scrolls);
    w.tree.set_children(viewport, &[content]);
    let root = w.column(&[card, viewport]);
    w.tree.set_root(root);
    w.frame();
    Page { w, log: Log::default(), now: Instant::now(), card, button, rows, content }
}

impl Page {
    /// Writes `name` and the event's kind to the log when the node gets an event of a kind.
    fn listen(&mut self, node: NodeId, kind: PointerKind, name: &'static str) {
        let log = self.log.clone();
        self.w.tree.on_pointer(node, kind, move |e| log.borrow_mut().push(format!("{name} {:?}", e.kind)));
    }

    fn send(&mut self, input: PointerInput, x: f32, y: f32) {
        self.w.tree.pointer(input, MouseButton::Left, Point::new(x, y), Mods::NONE, self.now);
    }

    fn click(&mut self, x: f32, y: f32) {
        self.send(PointerInput::Down, x, y);
        self.send(PointerInput::Up, x, y);
    }

    fn take(&mut self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
}

#[test]
fn the_deepest_node_under_a_point_is_hit_through_scrolling_and_clips() {
    let mut p = page();
    let hit = |tree: &Tree, x: f32, y: f32| tree.hit_test(Point::new(x, y));
    assert_eq!(hit(&p.w.tree, 10.0, 10.0), Some(p.button));
    assert_eq!(hit(&p.w.tree, 100.0, 50.0), Some(p.card));
    assert_eq!(hit(&p.w.tree, 500.0, 50.0), p.w.tree.root()); // beside the card
    assert_eq!(hit(&p.w.tree, 900.0, 50.0), None); // outside the window
    assert_eq!(hit(&p.w.tree, 10.0, 105.0), Some(p.rows[0])); // the viewport starts at 100
    assert_eq!(hit(&p.w.tree, 10.0, 199.0), Some(p.rows[4]));
    // Rows below the viewport are cut off: there the point is on the root.
    assert_eq!(hit(&p.w.tree, 10.0, 205.0), p.w.tree.root());

    p.w.tree.set_layer_offset(p.content, Point::new(0.0, -410.0));
    p.w.frame();
    assert_eq!(hit(&p.w.tree, 10.0, 105.0), Some(p.rows[20])); // 410 px down the list
}

#[test]
fn an_event_goes_down_the_path_and_back_up_until_a_handler_stops_it() {
    let mut p = page();
    let root = p.w.tree.root().unwrap();
    for (node, name) in [(root, "root"), (p.card, "card"), (p.button, "button")] {
        p.listen(node, PointerKind::Down, name);
        let log = p.log.clone();
        let down =
            move |e: &mut PointerEvent| log.borrow_mut().push(format!("{name} first, at {:?}", e.local));
        p.w.tree.on_pointer_capture(node, PointerKind::Down, down);
    }
    p.send(PointerInput::Down, 70.0, 30.0); // on the card, beside the button
    assert_eq!(
        p.take(),
        [
            "root first, at Point { x: 70.0, y: 30.0 }",
            "card first, at Point { x: 70.0, y: 30.0 }",
            "card Down",
            "root Down"
        ]
    );
    p.send(PointerInput::Up, 70.0, 30.0);

    let stop = |e: &mut PointerEvent| e.stop();
    p.w.tree.on_pointer(p.button, PointerKind::Down, stop);
    p.send(PointerInput::Down, 10.0, 10.0);
    assert_eq!(p.take().len(), 4); // three on the way down, the button's own, and no more
    assert!(!p.w.tree.needs_frame()); // handlers that write nothing ask for no frame
}

#[test]
fn hover_follows_the_pointer_and_what_scrolls_under_it() {
    let mut p = page();
    for (node, name) in [(p.card, "card"), (p.button, "button"), (p.rows[0], "row 0"), (p.rows[2], "row 2")] {
        p.listen(node, PointerKind::Enter, name);
        p.listen(node, PointerKind::Leave, name);
    }
    p.send(PointerInput::Move, 10.0, 10.0);
    assert_eq!(p.take(), ["card Enter", "button Enter"]); // from the outside in
    assert!(p.w.tree.is_hovered(p.card) && p.w.tree.is_hovered(p.button));
    p.send(PointerInput::Move, 100.0, 50.0);
    assert_eq!(p.take(), ["button Leave"]); // still on the card
    p.send(PointerInput::Move, 10.0, 110.0);
    assert_eq!(p.take(), ["card Leave", "row 0 Enter"]);

    // The list scrolls two rows up under a pointer that does not move.
    p.w.tree.set_layer_offset(p.content, Point::new(0.0, -40.0));
    p.w.frame();
    assert_eq!(p.take(), ["row 0 Leave", "row 2 Enter"]);
    p.send(PointerInput::Leave, 0.0, 0.0);
    assert_eq!(p.take(), ["row 2 Leave"]);
    assert!(!p.w.tree.is_hovered(p.rows[2]));
}

#[test]
fn a_click_is_a_press_and_a_release_that_share_a_node() {
    let mut p = page();
    let clicks: Rc<RefCell<Vec<u8>>> = Rc::default();
    let seen = clicks.clone();
    p.w.tree.on_pointer(p.button, PointerKind::Click, move |e| seen.borrow_mut().push(e.clicks));
    p.listen(p.card, PointerKind::Click, "card");

    p.click(10.0, 10.0);
    assert_eq!((clicks.borrow().clone(), p.take()), (vec![1], vec!["card Click".to_string()])); // it bubbles
    // Pressed on the button, released beside it: the card was under both, the button was not.
    p.send(PointerInput::Down, 10.0, 10.0);
    assert!(p.w.tree.is_pressed(p.button));
    p.send(PointerInput::Up, 100.0, 50.0);
    assert_eq!((clicks.borrow().len(), p.take()), (1, vec!["card Click".to_string()]));
    assert!(!p.w.tree.is_pressed(p.button));
    // Pressed outside, released on the button: no click on either.
    p.send(PointerInput::Down, 500.0, 300.0);
    p.send(PointerInput::Up, 10.0, 10.0);
    assert_eq!((clicks.borrow().len(), p.take().len()), (1, 0));

    // Clicks close in time and place count up; a late one starts again.
    clicks.borrow_mut().clear();
    p.now += Duration::from_secs(2);
    p.click(10.0, 10.0);
    p.now += Duration::from_millis(200);
    p.click(11.0, 10.0);
    p.now += Duration::from_millis(200);
    p.click(11.0, 11.0);
    p.now += Duration::from_millis(900);
    p.click(11.0, 11.0);
    assert_eq!(*clicks.borrow(), [1, 2, 3, 1]);
}

#[test]
fn a_node_that_captures_the_pointer_gets_the_moves_and_the_release() {
    let mut p = page();
    p.w.tree.on_pointer(p.button, PointerKind::Down, |e| e.capture_pointer());
    p.listen(p.button, PointerKind::Move, "button");
    p.listen(p.button, PointerKind::Up, "button");
    p.listen(p.button, PointerKind::Leave, "button");
    p.listen(p.rows[0], PointerKind::Move, "row 0");
    p.listen(p.rows[0], PointerKind::Enter, "row 0");

    p.send(PointerInput::Move, 10.0, 10.0);
    p.take();
    p.send(PointerInput::Down, 10.0, 10.0);
    p.send(PointerInput::Move, 10.0, 110.0); // far outside the button, over a row
    p.send(PointerInput::Up, 10.0, 110.0);
    // The row heard nothing until the release ended the capture; then hover caught up.
    assert_eq!(p.take(), ["button Move", "button Up", "button Leave", "row 0 Enter"]);
    p.send(PointerInput::Move, 12.0, 110.0);
    assert_eq!(p.take(), ["row 0 Move"]);
}

#[test]
fn a_node_removed_by_what_its_handler_set_off_is_gone_after_the_dispatch() {
    let mut p = page();
    let handle = p.w.tree.handle();
    let button = p.button;
    // A handler cannot reach the tree: it queues the removal, which the next frame applies.
    p.w.tree.on_pointer(button, PointerKind::Click, move |_| handle.with(move |tree| tree.remove(button)));
    p.listen(p.card, PointerKind::Click, "card");
    p.send(PointerInput::Move, 10.0, 10.0);
    p.click(10.0, 10.0);
    assert_eq!(p.take(), ["card Click"]); // the path was walked to its end
    assert!(p.w.tree.is_alive(button) && p.w.tree.needs_frame());
    p.w.frame();
    assert!(!p.w.tree.is_alive(button) && !p.w.tree.is_hovered(button));
    assert_eq!(p.w.tree.hit_test(Point::new(10.0, 10.0)), Some(p.card));
    p.click(10.0, 10.0); // nothing left of it to press or to click
    assert_eq!(p.take(), ["card Click"]);
}

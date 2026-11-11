//! Drag and drop between boxes (specs/09): a press becomes a drag past a threshold, a preview
//! follows the pointer without being there for it, the box that takes the value restyles
//! while it is over it and gets it at the release; Escape gives the drag up.

use quadra_platform::{Key, Mods};
use quadra_prim::{Color, Point};
use quadra_reactive::{Signal, signal};
use quadra_testing::Harness;
use quadra_tree::{PointerInput, State};
use quadra_ui::{NodeRef, column, div, row, text};

struct Task(&'static str);
struct File;

struct Board {
    h: Harness,
    done: NodeRef,
    dropped: Signal<Vec<(&'static str, Point)>>,
    clicks: Signal<u32>,
}

/// Where the card "ship" is, the middle of the box that takes tasks, of the one that takes
/// files, and a place outside all of them.
const SHIP: (f32, f32) = (88.0, 54.0);
const DONE: (f32, f32) = (320.0, 150.0);
const FILES: (f32, f32) = (540.0, 150.0);
const NOWHERE: (f32, f32) = (700.0, 500.0);

/// Two cards in a column, a box that takes tasks, and one that takes files.
fn board() -> Board {
    let (dropped, clicks, done) = (signal(Vec::new()), signal(0), NodeRef::new());
    let target = done.clone();
    let mut h = Harness::new();
    h.mount(move || {
        let card = move |name: &'static str| {
            let preview = move || div().size(160.0, 28.0).child(text(format!("moving {name}")));
            let card = div().size(160.0, 28.0).bg(Color::rgb(0xE5E7EB));
            let card = card.draggable_with(move || Task(name), preview);
            card.on_click(move |_| clicks.update(|n| *n += 1)).child(text(name))
        };
        let todo = column().size(200.0, 300.0).padding(8.0).gap(4.0);
        let todo = todo.child(card("write spec")).child(card("ship"));
        let done = div().size(200.0, 300.0).node_ref(&target).drop_target(|s| s.bg(Color::rgb(0xBFDBFE)));
        let done = done.on_drop(move |task: &Task, at| dropped.update(|d| d.push((task.0, at))));
        let files =
            div().size(200.0, 300.0).on_drop(move |_: &File, at| dropped.update(|d| d.push(("file", at))));
        row().gap(20.0).child(todo).child(done).child(files)
    });
    h.frame();
    Board { h, done, dropped, clicks }
}

impl Board {
    /// Presses at a point and moves the pointer to another, in two steps.
    fn drag(&mut self, from: (f32, f32), to: (f32, f32)) {
        self.h.pointer(PointerInput::Move, from.0, from.1);
        self.h.pointer(PointerInput::Down, from.0, from.1);
        self.h.pointer(PointerInput::Move, (from.0 + to.0) / 2.0, (from.1 + to.1) / 2.0);
        self.h.pointer(PointerInput::Move, to.0, to.1);
        self.h.frame();
    }

    fn release(&mut self, at: (f32, f32)) {
        self.h.pointer(PointerInput::Up, at.0, at.1);
        self.h.frame_if_needed();
    }

    fn over_target(&self) -> bool {
        self.h.tree.state(self.done.get().unwrap()).contains(State::DROP_TARGET)
    }
}

#[test]
fn a_card_dropped_on_the_box_that_takes_it_is_given_to_it() {
    let mut b = board();
    b.drag(SHIP, DONE);
    b.release(DONE);
    assert_eq!(b.dropped.get_untracked(), [("ship", Point::new(DONE.0, DONE.1))]);
    assert_eq!(b.clicks.get_untracked(), 0); // a drag is not a click
    assert!(!b.over_target() && b.h.tree.overlays().count() == 0 && !b.h.tree.dragging());
}

#[test]
fn the_preview_follows_without_being_there_and_the_target_restyles() {
    let mut b = board();
    b.drag(SHIP, DONE);
    let preview = b.h.tree.overlays().next().expect("the preview is an overlay");
    // It is held where the card was pressed (80 px in, 14 down), and what is under the
    // pointer is still the box.
    assert_eq!(b.h.tree.window_rect(preview).origin(), Point::new(DONE.0 - 80.0, DONE.1 - 14.0));
    assert_eq!(b.h.tree.hit_test(Point::new(DONE.0, DONE.1)), b.done.get());
    assert!(!b.h.has_text("moving ship")); // nor is it there for a reader
    assert!(b.over_target());
    let done = b.done.get().unwrap();
    assert_eq!(b.h.tree.paint_style(done).background, Color::rgb(0xBFDBFE));

    // Moving on moves the preview: one layer placed again, nothing painted.
    b.h.pointer(PointerInput::Move, DONE.0 + 30.0, DONE.1);
    let (stats, _) = b.h.frame();
    assert_eq!((stats.nodes_painted, stats.nodes_laid_out, stats.layers_composed), (0, 0, 1));
    assert_eq!(b.h.tree.window_rect(preview).x, DONE.0 - 80.0 + 30.0);

    b.h.key(Key::Escape, Mods::NONE); // gives the drag up: the release drops nothing
    b.release(DONE);
    assert!(b.dropped.get_untracked().is_empty() && b.clicks.get_untracked() == 0);
    assert!(!b.over_target() && b.h.tree.overlays().count() == 0);
    assert_eq!(b.h.tree.paint_style(done).background, Color::TRANSPARENT);
}

#[test]
fn a_small_move_is_a_click_and_other_boxes_take_nothing() {
    let mut b = board();
    b.drag(SHIP, (SHIP.0 + 2.0, SHIP.1 + 1.0)); // under the threshold
    assert!(!b.h.tree.dragging());
    b.release((SHIP.0 + 2.0, SHIP.1 + 1.0));
    assert_eq!(b.clicks.get_untracked(), 1);

    b.drag(SHIP, FILES); // a box for another kind of thing
    assert!(b.h.tree.dragging() && !b.over_target());
    b.release(FILES);
    b.drag(SHIP, NOWHERE);
    b.release(NOWHERE);
    assert!(b.dropped.get_untracked().is_empty());
    assert_eq!(b.clicks.get_untracked(), 1);
}

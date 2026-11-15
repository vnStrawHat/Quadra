//! The view of a pane tree (specs/behaviour/16, "Pane tree"). The tree is not mirrored by
//! nested boxes: every leaf and every seam is a child of one box, placed where the tree puts
//! it. A change of the tree moves them; a leaf is built once, and kept while its group is in
//! the tree. The look gives the seams and the veil over a pane a tab would be dropped on.

use std::cell::Cell;
use std::rc::Rc;

use quadra_layout::{Direction, Overflow};
use quadra_prim::{Rect, Size};
use quadra_reactive::{Memo, Signal, effect, memo, signal};
use quadra_tree::{NodeId, PointerEvent, PointerKind};
use quadra_view::{AnyView, Div, MountCx, View, div, each};

use super::dock::{Dock, DockEvent, TabDrag, Zone, zone_at, zone_rect};
use super::pane::{Axis, Pane, Part};

/// The least a side of a split is dragged to, in pixels, where the split has the room.
pub const LEAST: f32 = 80.0;

/// What a pane tree is drawn with: how wide the strip of a seam is, the seam on the box the
/// tree puts it in (its axis, and whether it is the fixed line beside a folded leaf), and the
/// veil over the part of a pane a drop would take (none while no tab is over it).
#[derive(Clone)]
pub struct Parts {
    pub grip: f32,
    pub seam: Rc<dyn Fn(Div, Axis, bool) -> Div>,
    pub veil: Rc<dyn Fn(Memo<Option<Rect>>) -> Div>,
}

pub struct Panes<F> {
    root: Signal<Pane>,
    leaf: F,
    header: f32,
    parts: Parts,
    dock: Option<Dock>,
}

/// Shows a pane tree: `leaf` builds what a group shows, once, and the seams between the
/// sides of the splits are dragged to give a side more of the room (which writes the tree).
pub fn panes<V, F>(root: Signal<Pane>, leaf: F, parts: Parts) -> Panes<F>
where
    V: View + 'static,
    F: Fn(&str) -> V + 'static,
{
    Panes { root, leaf, header: 0.0, parts, dock: None }
}

impl<F> Panes<F> {
    /// The height a collapsed leaf keeps, that of its header.
    pub fn header(mut self, px: f32) -> Self {
        self.header = px;
        self
    }

    /// Makes every pane take the tabs torn off the strips of a docking workbench: dropped on
    /// its middle a tab joins its group, on an edge it has a new pane split off there. The
    /// part a drop would take is veiled while a tab is over the pane.
    pub fn dock(mut self, dock: Dock) -> Self {
        self.dock = Some(dock);
        self
    }
}

fn rect_of(part: &Part) -> Rect {
    match part {
        Part::Leaf { rect, .. } | Part::Seam { rect, .. } => *rect,
    }
}

/// A pane that takes dragged tabs for a group.
fn target(pane: Div, part: Signal<Part>, (dock, parts): (Dock, &Parts), group: String) -> Div {
    let over = signal(None::<Zone>);
    let mark = memo(move || {
        let size = part.with(rect_of).size();
        over.get().map(|zone| zone_rect(Rect::new(0.0, 0.0, size.width, size.height), zone))
    });
    pane.child((parts.veil)(mark)).on_mount(move |tree, id| {
        // Where the pane is in the window is the tree's to say: after the event.
        let (moved, dropped) = (tree.handle(), tree.handle());
        tree.on_drag_over::<TabDrag>(id, move |at| {
            moved.with(move |tree| over.set(at.map(|at| zone_at(tree.window_rect(id), at))));
        });
        tree.on_drop::<TabDrag>(id, move |drag: &TabDrag, at| {
            let (tab, from, to) = (drag.tab.clone(), drag.group.clone(), group.clone());
            dropped.with(move |tree| {
                let zone = zone_at(tree.window_rect(id), at);
                dock.apply(&DockEvent { tab, from, to, zone, index: None });
            });
        });
    })
}

/// What tells a part from the others: a leaf is its group; a seam is its split, and what
/// kind of seam it is there.
type Key = Result<String, (Vec<usize>, bool, bool)>;

fn key(part: &Part) -> Key {
    match part {
        Part::Leaf { group, .. } => Ok(group.clone()),
        Part::Seam { path, axis, fixed, .. } => Err((path.clone(), *axis == Axis::Column, *fixed)),
    }
}

/// A box where the tree puts a part.
fn placed(part: Signal<Part>) -> Div {
    let rect = move || part.with(rect_of);
    let place = div().absolute().left(move || rect().x).top(move || rect().y);
    place.width(move || rect().width).height(move || rect().height)
}

/// The seam of a split. A press on it holds it, and the share of the first side follows the
/// pointer; a double click makes the sides even.
fn seam(root: Signal<Pane>, part: Signal<Part>, parts: &Parts) -> Div {
    let Part::Seam { path, axis, fixed, .. } = part.get_untracked() else { return div() };
    let strip = (parts.seam)(placed(part), axis, fixed);
    if fixed {
        return strip;
    }
    let stacked = axis == Axis::Column;
    let at = move |e: &PointerEvent| if stacked { e.at.y } else { e.at.x };
    let held: Rc<Cell<Option<(f32, f32)>>> = Rc::default();
    let (press, drag, of, even) = (held.clone(), held.clone(), path.clone(), path.clone());
    let strip = strip.on_pointer(PointerKind::Down, move |e| {
        let share = root.with_untracked(|tree| tree.ratio(&of));
        press.set(share.map(|share| (at(e), share)));
        e.capture_pointer();
        e.stop();
    });
    let strip = strip.on_pointer(PointerKind::Move, move |e| {
        let (Some((from, was)), Part::Seam { whole, .. }) = (drag.get(), part.get_untracked()) else {
            return;
        };
        let length = if stacked { whole.height } else { whole.width };
        if length > 2.0 * LEAST {
            let to = (was + (at(e) - from) / length).clamp(LEAST / length, 1.0 - LEAST / length);
            root.update(|tree| tree.set_ratio(&path, to));
        }
    });
    let strip = strip.on_pointer(PointerKind::Up, move |_| held.set(None));
    strip.on_click(move |e| {
        if e.clicks == 2 {
            root.update(|tree| tree.set_ratio(&even, 0.5));
        }
    })
}

impl<V, F> View for Panes<F>
where
    V: View + 'static,
    F: Fn(&str) -> V + 'static,
{
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (root, header, leaf, dock, parts) = (self.root, self.header, self.leaf, self.dock, self.parts);
        // The box of the tree is known once it is laid out: the parts are placed in it then.
        let (whole, grip) = (signal(Size::default()), parts.grip);
        let placing = memo(move || {
            let Size { width, height } = whole.get();
            root.with(|tree| tree.place(Rect::new(0.0, 0.0, width, height), (header, grip)))
        });
        let part = move |part: Signal<Part>| match part.get_untracked() {
            Part::Leaf { group, .. } => {
                let pane = placed(part).direction(Direction::Column);
                let pane = pane.overflow_x(Overflow::Hidden).overflow_y(Overflow::Hidden);
                let pane = pane.child(leaf(&group));
                AnyView::new(match dock {
                    Some(dock) => target(pane, part, (dock, &parts), group),
                    None => pane,
                })
            }
            Part::Seam { .. } => AnyView::new(seam(root, part, &parts)),
        };
        let node = each(move || placing.get(), key, part).container(div().grow(1.0)).mount(cx);
        let real = cx.tree.size_signal(node);
        effect(move || whole.set(real.get()));
        node
    }
}

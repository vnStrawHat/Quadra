//! The pointer on a graph: a drag pans it, the wheel with Ctrl zooms it about the pointer, a
//! double click fits it; a click or the pointer near a link tells of it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use quadra_platform::Mods;
use quadra_prim::{Point, Rect, Size};
use quadra_reactive::Signal;
use quadra_tree::{PointerEvent, PointerKind};
use quadra_view::Div;

use super::Hit;
use super::links::distance;

/// Each link's place and its line in the graph's plane.
pub(super) type Routes = Rc<Vec<(usize, Vec<Point>)>>;

/// What the pointer is over, and whether a press has become a drag.
pub(super) type Shared = (Rc<RefCell<Option<Hit>>>, Rc<Cell<bool>>);

pub(super) type Told = (Option<Rc<dyn Fn(Hit)>>, Option<Rc<dyn Fn(Option<Hit>)>>);

/// The pointer on the graph: a drag pans, the wheel with Ctrl zooms about the pointer, a
/// double click fits; a click or the pointer near a link tells of it. A still graph only
/// tells.
pub(super) fn pointer(
    all: Div,
    view: Signal<GraphView>,
    routes: Routes,
    (click, hover): Told,
    (last, moved): Shared,
    (bounds, still): (Rect, bool),
) -> Div {
    // Within 5 px of the line as it shows: the pointer taken back to the plane.
    let link_at = move |at: Point| {
        let v = view.get_untracked();
        let at = Point::new((at.x - v.pan.x) / v.zoom, (at.y - v.pan.y) / v.zoom);
        routes.iter().find(|(_, line)| distance(line, at) * v.zoom <= 5.0).map(|&(k, _)| k)
    };
    let held = Rc::new(Cell::new(None::<(Point, Point)>));
    let (press, drag, up) = (held.clone(), held.clone(), held);
    let (fresh, dragged, held_still) = (moved.clone(), moved.clone(), moved);
    let (near, tap) = (link_at.clone(), link_at);
    let all = match still {
        true => all,
        false => all.on_pointer(PointerKind::Down, move |e| {
            e.capture_pointer();
            press.set(Some((e.at, view.get_untracked().pan)));
            fresh.set(false);
        }),
    };
    let all = all.on_pointer(PointerKind::Move, move |e: &mut PointerEvent| match drag.get() {
        Some((from, pan)) => {
            let (dx, dy) = (e.at.x - from.x, e.at.y - from.y);
            if dx.hypot(dy) > 3.0 || dragged.get() {
                dragged.set(true);
                view.update(|v| v.pan = Point::new(pan.x + dx, pan.y + dy));
            }
        }
        // Over a node, the node tells of it.
        None if matches!(*last.borrow(), Some(Hit::Node(_))) => {}
        None => {
            let now = near(e.local).map(Hit::Link);
            if *last.borrow() != now {
                *last.borrow_mut() = now.clone();
                hover.iter().for_each(|f| f(now.clone()));
            }
        }
    });
    let all = all.on_pointer(PointerKind::Up, move |_| up.set(None));
    let all = all.on_pointer(PointerKind::Click, move |e| match e.clicks {
        2 if !still => view.set(fitted(bounds, e.size)),
        _ if !held_still.get() => {
            if let (Some(k), Some(f)) = (tap(e.local), &click) {
                f(Hit::Link(k));
            }
        }
        _ => {}
    });
    all.on_pointer(PointerKind::Wheel, move |e| {
        if e.mods.contains(Mods::CTRL) && !still {
            e.stop();
            let v = view.get_untracked();
            let zoom = (v.zoom * 2f32.powf(e.delta.y / 400.0)).clamp(0.25, 4.0);
            let k = zoom / v.zoom;
            let pan =
                Point::new(e.local.x - (e.local.x - v.pan.x) * k, e.local.y - (e.local.y - v.pan.y) * k);
            view.set(GraphView { pan, zoom });
        }
    })
}

/// Where the graph's plane shows in its box: a point `p` of it is at `p * zoom + pan`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GraphView {
    pub pan: Point,
    pub zoom: f32,
}

impl Default for GraphView {
    fn default() -> Self {
        GraphView { pan: Point::ZERO, zoom: 1.0 }
    }
}

impl GraphView {
    pub fn point(self, p: Point) -> Point {
        Point::new(p.x * self.zoom + self.pan.x, p.y * self.zoom + self.pan.y)
    }

    pub fn rect(self, r: Rect) -> Rect {
        let at = self.point(r.origin());
        Rect::new(at.x, at.y, r.width * self.zoom, r.height * self.zoom)
    }
}

/// The view that shows all of `bounds` in a box of `size`: 16 px of margin, its middle in
/// the box's, never larger than it is.
pub fn fitted(bounds: Rect, size: Size) -> GraphView {
    let zoom = ((size.width - 32.0) / bounds.width).min((size.height - 32.0) / bounds.height);
    let zoom = zoom.clamp(0.25, 1.0);
    let middle = bounds.center();
    let pan = Point::new(size.width / 2.0 - middle.x * zoom, size.height / 2.0 - middle.y * zoom);
    GraphView { pan, zoom }
}

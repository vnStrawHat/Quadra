//! The pointer on a graph: a drag pans it, the wheel with Ctrl zooms it about the pointer, a
//! double click fits it; a click or the pointer near a link tells of it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use quadra_platform::Mods;
use quadra_prim::{Point, Rect};
use quadra_reactive::Signal;
use quadra_tree::{PointerEvent, PointerKind};
use quadra_view::Div;

use super::links::{distance, route};
use super::{GraphView, Hit, fitted};

/// Each link's place and its two nodes.
pub(super) type Ends = Rc<Vec<(usize, usize, usize)>>;

pub(super) type Told = (Option<Rc<dyn Fn(Hit)>>, Option<Rc<dyn Fn(Option<Hit>)>>);

/// The pointer on the graph: a drag pans, the wheel with Ctrl zooms about the pointer, a
/// double click fits; a click or the pointer near a link tells of it.
pub(super) fn pointer(
    all: Div,
    view: Signal<GraphView>,
    (rects, ends): (Rc<Vec<Rect>>, Ends),
    (click, hover): Told,
    bounds: Rect,
) -> Div {
    // ponytail: every link is routed again at each move; cache the routes for graphs of
    // thousands of links.
    let link_at = move |at: Point| {
        let v = view.get_untracked();
        let near = |&&(_, a, b): &&(usize, usize, usize)| {
            let line: Vec<Point> = route(rects[a], rects[b]).into_iter().map(|p| v.point(p)).collect();
            distance(&line, at) <= 5.0
        };
        ends.iter().find(near).map(|&(k, ..)| k)
    };
    let (held, moved, last) =
        (Rc::new(Cell::new(None::<(Point, Point)>)), Rc::new(Cell::new(false)), Rc::new(RefCell::new(None)));
    let (press, drag, up) = (held.clone(), held.clone(), held);
    let (fresh, dragged, still) = (moved.clone(), moved.clone(), moved);
    let (near, tap) = (link_at.clone(), link_at);
    let all = all.on_pointer(PointerKind::Down, move |e| {
        e.capture_pointer();
        press.set(Some((e.at, view.get_untracked().pan)));
        fresh.set(false);
    });
    let all = all.on_pointer(PointerKind::Move, move |e: &mut PointerEvent| match drag.get() {
        Some((from, pan)) => {
            let (dx, dy) = (e.at.x - from.x, e.at.y - from.y);
            if dx.hypot(dy) > 3.0 || dragged.get() {
                dragged.set(true);
                view.update(|v| v.pan = Point::new(pan.x + dx, pan.y + dy));
            }
        }
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
        2 => view.set(fitted(bounds, e.size)),
        _ if !still.get() => {
            if let (Some(k), Some(f)) = (tap(e.local), &click) {
                f(Hit::Link(k));
            }
        }
        _ => {}
    });
    all.on_pointer(PointerKind::Wheel, move |e| {
        if e.mods.contains(Mods::CTRL) {
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

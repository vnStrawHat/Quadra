//! What every popup does (specs/17, "The components"; specs/13, "Overlays in the kit"): it
//! opens beside what opened it, at a place, or along an edge of the window; keeps Tab inside
//! itself; closes on a press outside or on Escape; and gives the focus back to what had it.
//! The look gives the plane it stands on and what it holds.

use std::cell::Cell;
use std::rc::Rc;

use quadra_layout::{Display, Length};
use quadra_prim::Point;
use quadra_reactive::{Scope, Signal, on_cleanup, untrack};
use quadra_tree::{Anchor, NodeId, Tree};
use quadra_view::{Div, MountCx, NodeRef, Place, Region, View, div, overlay, show};

use crate::action::{Cancel, POPUP};

/// When a popup is open, and what closes it.
#[derive(Clone)]
pub struct Shown {
    pub is: Rc<dyn Fn() -> bool>,
    pub close: Rc<dyn Fn()>,
}

impl From<Signal<bool>> for Shown {
    fn from(open: Signal<bool>) -> Shown {
        Shown { is: Rc::new(move || open.get()), close: Rc::new(move || open.set(false)) }
    }
}

/// An edge of the window a popup lies along over all of its length: a drawer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Bottom,
}

/// Where a popup stands: its place, and the edge of the window it lies along, if it does.
#[derive(Clone)]
pub struct Stand {
    pub place: Place,
    pub along: Option<Edge>,
}

impl From<Place> for Stand {
    fn from(place: Place) -> Stand {
        Stand { place, along: None }
    }
}

impl From<Edge> for Stand {
    fn from(edge: Edge) -> Stand {
        let place = match edge {
            Edge::Left => Place::At(Point::ZERO),
            Edge::Right => Place::BottomRight(0.0, 0.0),
            Edge::Bottom => Place::Bottom(0.0),
        };
        Stand { place, along: Some(edge) }
    }
}

/// The plane a popup stands on: the look's, built each time it opens.
pub type Plane = Rc<dyn Fn() -> Div>;

/// A popup while it is open.
struct Open<V> {
    close: Rc<dyn Fn()>,
    stand: Stand,
    opener: Option<NodeRef>,
    plane: Plane,
    content: V,
}

impl<V: View + 'static> View for Open<V> {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (close, outside, panel_ref) = (self.close.clone(), self.close, NodeRef::new());
        // The plane can have the focus itself, for a popup that holds nothing that can.
        let Stand { place, along } = self.stand;
        let panel = (self.plane)().focus_scope().tab_index(-1).key_context(POPUP).node_ref(&panel_ref);
        // Along an edge it is as long as the window there: an overlay is laid out in its size.
        let panel = match along {
            Some(Edge::Bottom) => panel.width(Length::Fill),
            Some(_) => panel.height(Length::Fill),
            None => panel,
        };
        let panel = panel.on_action::<Cancel>(move |_| close()).child(self.content);
        let shown = overlay(place, panel).on_outside_press(move || outside());
        let shown = match &self.opener {
            Some(opener) => shown.opener(opener),
            None => shown,
        };
        // The focus goes into the popup, and back to what had it when the popup closes.
        let before = cx.tree.focused();
        let id = shown.mount(cx);
        if let Some(panel) = panel_ref.get() {
            cx.tree.focus_first(panel);
        }
        let handle = cx.handle.clone();
        on_cleanup(move || {
            handle.with(move |tree| {
                if let Some(before) = before {
                    tree.focus(before);
                }
            })
        });
        id
    }
}

/// Shows what `content` builds on the look's `plane`, at `stand`, while `open` is true.
/// `content` runs each time the popup opens. A press on `opener` is not a press outside: the
/// button that opened a popup closes it by its own click and does not open it again.
pub fn popup<V, F>(
    open: impl Into<Shown>,
    stand: impl Into<Stand>,
    opener: Option<NodeRef>,
    plane: impl Fn() -> Div + 'static,
    content: F,
) -> Region
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    let (Shown { is, close }, stand, plane) = (open.into(), stand.into(), Rc::new(plane) as Plane);
    let shown = move || {
        let (close, stand, opener, plane) = (close.clone(), stand.clone(), opener.clone(), plane.clone());
        Open { close, stand, opener, plane, content: content() }
    };
    // The region takes no room where it stands, not even a gap: what it shows is above.
    show(move || is(), shown).container(div().display(Display::None))
}

/// Opens a popup now, from outside any view: a menu at the pointer. `content` is given what
/// closes it. It ends with `owner` if it is still open then.
pub fn open_now<V: View + 'static>(
    tree: &mut Tree,
    owner: Scope,
    place: Place,
    plane: impl Fn() -> Div + 'static,
    content: impl FnOnce(Rc<dyn Fn()>) -> V,
) {
    if !owner.is_alive() {
        return;
    }
    let (scope, handle) = (owner.run(Scope::child), tree.handle());
    let later = handle.clone();
    let close: Rc<dyn Fn()> = Rc::new(move || later.with(move |_| scope.dispose()));
    scope.run(|| {
        let (stand, plane) = (Stand::from(place), Rc::new(plane) as Plane);
        let view = Open { close: close.clone(), stand, opener: None, plane, content: content(close) };
        // What stands for the popup in the flow stands nowhere: it goes with the popup.
        let node = view.mount(&mut MountCx::new(tree));
        on_cleanup(move || handle.with(move |tree| tree.remove(node)));
    });
}

/// Shows what `build` gives as an overlay against `anchor` while `shown` is true, opened by
/// hand: for what adds no node around the view `of` it is about (a tooltip, a hover card).
/// It is built in a scope of its own each time it shows, and goes with the scope it was
/// asked in. It takes neither the focus nor a press outside.
pub fn hang<V: View + 'static>(
    cx: &mut MountCx,
    of: NodeId,
    shown: Signal<bool>,
    anchor: Anchor,
    build: impl Fn() -> V + 'static,
) {
    hang_with(cx, of, shown, move || anchor, build);
}

/// As [`hang`], against an anchor read each time it shows: what follows a part of a view (a
/// link of a text).
pub fn hang_with<V: View + 'static>(
    cx: &mut MountCx,
    of: NodeId,
    shown: Signal<bool>,
    anchor: impl Fn() -> Anchor + 'static,
    build: impl Fn() -> V + 'static,
) {
    let (owner, open, build) = (Scope::current(), Rc::new(Cell::new(None)), Rc::new(build));
    let (writer, last, anchor) = (cx.handle.clone(), open.clone(), Rc::new(anchor));
    cx.handle.bind(move || {
        let (on, build, open, anchor) = (shown.get(), build.clone(), open.clone(), anchor.clone());
        writer.with(move |tree| {
            if let Some(old) = open.take() {
                tree.close_overlay(old);
            }
            if !on || !tree.is_alive(of) || !owner.is_alive() {
                return;
            }
            let scope = owner.run(Scope::child);
            let root = scope.run(|| build().mount(&mut MountCx::new(tree)));
            tree.set_scope(root, scope);
            tree.open_overlay(root, untrack(&*anchor));
            open.set(Some(root));
        });
    });
    let handle = cx.handle.clone();
    on_cleanup(move || {
        handle.with(move |tree| {
            if let Some(root) = last.take() {
                tree.close_overlay(root);
            }
        })
    });
}

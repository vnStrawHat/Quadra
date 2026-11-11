//! What every popup of the kit does (specs/13, "Overlays in the kit"): it opens beside what
//! opened it, keeps Tab inside itself, closes on a press outside or on Escape, and gives the
//! focus back to what had it. Menus, selects, popovers and dialogs are this, with content.

use std::cell::Cell;
use std::rc::Rc;

use quadra_layout::{Display, Length};
use quadra_prim::Point;
use quadra_reactive::{Scope, Signal, on_cleanup};
use quadra_tree::{Along, Anchor, NodeId, Side, Tree};
use quadra_ui::{Div, MountCx, NodeRef, Place, Region, View, column, div, overlay, show};

use crate::action::{Cancel, POPUP};
use crate::{size, token};

/// The plane of an overlay: its own fill, a hairline border, the largest radius, and the
/// shadow of its level above the window (1: menus and tooltips; 2: dialogs).
pub(crate) fn surface(level: u8) -> Div {
    let plane = column().bg(token::OVERLAY_BG).border(1.0).border_color(token::LINE);
    plane.radius(size::RADIUS_XL).elevation(level)
}

/// What the border of the plane adds to the width of what it holds: a popup that is so wide
/// asks for that less for its content.
pub(crate) const EDGE: f32 = 2.0;

/// When a popup is open, and what closes it.
#[derive(Clone)]
pub(crate) struct Shown {
    pub is: Rc<dyn Fn() -> bool>,
    pub close: Rc<dyn Fn()>,
}

impl From<Signal<bool>> for Shown {
    fn from(open: Signal<bool>) -> Shown {
        Shown { is: Rc::new(move || open.get()), close: Rc::new(move || open.set(false)) }
    }
}

/// An edge of the window a drawer lies along.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Bottom,
}

/// Where a popup stands: its place, its level above the window, and the edge of the window
/// it lies along over all of its length, if it does.
#[derive(Clone)]
pub(crate) struct Stand {
    place: Place,
    level: u8,
    along: Option<Edge>,
}

impl From<(Place, u8)> for Stand {
    fn from((place, level): (Place, u8)) -> Stand {
        Stand { place, level, along: None }
    }
}

impl From<Edge> for Stand {
    fn from(edge: Edge) -> Stand {
        let place = match edge {
            Edge::Left => Place::At(Point::ZERO),
            Edge::Right => Place::BottomRight(0.0, 0.0),
            Edge::Bottom => Place::Bottom(0.0),
        };
        Stand { place, level: 2, along: Some(edge) }
    }
}

/// A popup while it is open.
struct Open<V> {
    close: Rc<dyn Fn()>,
    stand: Stand,
    opener: Option<NodeRef>,
    content: V,
}

impl<V: View + 'static> View for Open<V> {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (close, outside, panel_ref) = (self.close.clone(), self.close, NodeRef::new());
        // The panel can have the focus itself, for a popup that holds nothing that can.
        let Stand { place, level, along } = self.stand;
        let panel = surface(level).focus_scope().tab_index(-1).key_context(POPUP).node_ref(&panel_ref);
        // Along an edge: as long as the window there (an overlay is laid out in its size),
        // and square.
        let panel = match along {
            Some(Edge::Bottom) => panel.width(Length::Fill).radius(0.0),
            Some(_) => panel.height(Length::Fill).radius(0.0),
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

/// Shows what `content` builds, in a popup below `opener`, while `open` is true. `content`
/// runs each time the popup opens.
pub fn popover<V, F>(open: Signal<bool>, opener: &NodeRef, content: F) -> impl View + use<V, F>
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    popup(
        open,
        (Place::Beside(opener.clone(), Side::Below, Along::Start, 4.0), 1),
        Some(opener.clone()),
        content,
    )
}

/// A popup at a place of the caller's choosing (beside a node on another side, at a point,
/// under the top of the window) and at a level above the window, or along an edge of it.
pub(crate) fn popup<V, F>(
    open: impl Into<Shown>,
    stand: impl Into<Stand>,
    opener: Option<NodeRef>,
    content: F,
) -> Region
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    let (Shown { is, close }, stand) = (open.into(), stand.into());
    let shown = move || {
        let (close, stand, opener) = (close.clone(), stand.clone(), opener.clone());
        Open { close, stand, opener, content: content() }
    };
    // The region takes no room where it stands, not even a gap: what it shows is above.
    show(move || is(), shown).container(div().display(Display::None))
}

/// Opens a popup now, from outside any view: a menu at the pointer. `content` is given what
/// closes it. It ends with `owner` if it is still open then.
pub(crate) fn open_now<V: View + 'static>(
    tree: &mut Tree,
    owner: Scope,
    place: Place,
    content: impl FnOnce(Rc<dyn Fn()>) -> V,
) {
    if !owner.is_alive() {
        return;
    }
    let (scope, handle) = (owner.run(Scope::child), tree.handle());
    let later = handle.clone();
    let close: Rc<dyn Fn()> = Rc::new(move || later.with(move |_| scope.dispose()));
    scope.run(|| {
        let stand = Stand::from((place, 1));
        let view = Open { close: close.clone(), stand, opener: None, content: content(close) };
        // What stands for the popup in the flow stands nowhere: it goes with the popup.
        let node = view.mount(&mut MountCx::new(tree));
        on_cleanup(move || handle.with(move |tree| tree.remove(node)));
    });
}

/// Shows what `build` gives as an overlay against `anchor` while `shown` is true, opened by
/// hand: for what adds no node around the view `of` it is about (a tooltip, a hover card).
/// It is built in a scope of its own each time it shows, and goes with the scope it was
/// asked in.
pub(crate) fn hang<V: View + 'static>(
    cx: &mut MountCx,
    of: NodeId,
    shown: Signal<bool>,
    anchor: Anchor,
    build: impl Fn() -> V + 'static,
) {
    let (owner, open, build) = (Scope::current(), Rc::new(Cell::new(None)), Rc::new(build));
    let (writer, last) = (cx.handle.clone(), open.clone());
    cx.handle.bind(move || {
        let (on, build, open) = (shown.get(), build.clone(), open.clone());
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
            tree.open_overlay(root, anchor);
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

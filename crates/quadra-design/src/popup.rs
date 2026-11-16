//! The kit's popups (specs/13, "Overlays in the kit"): the headless crate's (`quadra_headless::popup`) on
//! the kit's plane. Menus, selects, popovers and dialogs are this, with content.

use std::rc::Rc;

use quadra_headless::popup as base;
use quadra_reactive::{Scope, Signal};
use quadra_tree::{Along, Side, Tree};
use quadra_view::{Div, NodeRef, Place, Region, View, column};

pub use quadra_headless::popup::Edge;
pub(crate) use quadra_headless::popup::{Shown, hang};

use crate::token;

/// The plane of an overlay: its own fill, a hairline border, the largest radius, and the
/// shadow of its level above the window (1: menus and tooltips; 2: dialogs).
pub(crate) fn surface(level: u8) -> Div {
    let plane = column().bg(token::OVERLAY_BG).border(1.0).border_color(token::LINE_STRONG);
    plane.radius(token::RADIUS_XL).elevation(level)
}

/// The plane of a list to choose from (menus, selects, the command palette): the panel's fill,
/// a quiet border and small corners.
pub(crate) fn menu_surface() -> Div {
    let plane = column().bg(token::PANEL).border(1.0).border_color(token::LINE);
    plane.radius(token::RADIUS_SM).elevation(1)
}

/// What the border of the plane adds to the width of what it holds: a popup that is so wide
/// asks for that less for its content.
pub(crate) const EDGE: f32 = 2.0;

/// Where a popup stands: its place, its level above the window, and the edge of the window
/// it lies along over all of its length, if it does (with square corners).
#[derive(Clone)]
pub(crate) struct Stand {
    place: Place,
    level: u8,
    along: Option<Edge>,
    menu: bool,
}

impl From<(Place, u8)> for Stand {
    fn from((place, level): (Place, u8)) -> Stand {
        Stand { place, level, along: None, menu: false }
    }
}

impl From<Edge> for Stand {
    fn from(edge: Edge) -> Stand {
        Stand { place: base::Stand::from(edge).place, level: 2, along: Some(edge), menu: false }
    }
}

/// A list to choose from at `place`, on the menu plane.
pub(crate) fn listing(place: Place) -> Stand {
    Stand { place, level: 1, along: None, menu: true }
}

/// Below `opener`, 4 px off it.
pub(crate) fn below(opener: &NodeRef) -> Place {
    Place::Beside(opener.clone(), Side::Below, Along::Start, 4.0)
}

/// Shows what `content` builds, in a popup below `opener`, while `open` is true. `content`
/// runs each time the popup opens.
pub fn popover<V, F>(open: Signal<bool>, opener: &NodeRef, content: F) -> impl View + use<V, F>
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    popup(open, (below(opener), 1), Some(opener.clone()), content)
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
    let Stand { place, level, along, menu } = stand.into();
    let plane = move || match (menu, along) {
        (true, _) => menu_surface(),
        (_, Some(_)) => surface(level).radius(0.0),
        _ => surface(level),
    };
    base::popup(open, base::Stand { place, along }, opener, plane, content)
}

/// Opens a popup now, from outside any view: a menu at the pointer. `content` is given what
/// closes it. It ends with `owner` if it is still open then.
pub(crate) fn open_now<V: View + 'static>(
    tree: &mut Tree,
    owner: Scope,
    place: Place,
    content: impl FnOnce(Rc<dyn Fn()>) -> V,
) {
    base::open_now(tree, owner, place, menu_surface, content)
}

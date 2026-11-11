//! What every popup of the kit does (specs/13, "Overlays in the kit"): it opens beside what
//! opened it, keeps Tab inside itself, closes on a press outside or on Escape, and gives the
//! focus back to what had it. Menus, selects, popovers and dialogs are this, with content.

use quadra_layout::Display;
use std::rc::Rc;

use quadra_reactive::{Scope, Signal, on_cleanup};
use quadra_tree::{Along, NodeId, Side, Tree};
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

/// A popup while it is open.
struct Open<V> {
    close: Rc<dyn Fn()>,
    level: u8,
    place: Place,
    opener: Option<NodeRef>,
    content: V,
}

impl<V: View + 'static> View for Open<V> {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (close, outside, panel_ref) = (self.close.clone(), self.close, NodeRef::new());
        // The panel can have the focus itself, for a popup that holds nothing that can.
        let panel = surface(self.level).focus_scope().tab_index(-1).key_context(POPUP).node_ref(&panel_ref);
        let panel = panel.on_action::<Cancel>(move |_| close()).child(self.content);
        let shown = overlay(self.place, panel).on_outside_press(move || outside());
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
/// under the top of the window) and at a level above the window.
pub(crate) fn popup<V, F>(
    open: impl Into<Shown>,
    (place, level): (Place, u8),
    opener: Option<NodeRef>,
    content: F,
) -> Region
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    let Shown { is, close } = open.into();
    let shown = move || {
        let (close, place, opener) = (close.clone(), place.clone(), opener.clone());
        Open { close, level, place, opener, content: content() }
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
        let view = Open { close: close.clone(), level: 1, place, opener: None, content: content(close) };
        // What stands for the popup in the flow stands nowhere: it goes with the popup.
        let node = view.mount(&mut MountCx::new(tree));
        on_cleanup(move || handle.with(move |tree| tree.remove(node)));
    });
}

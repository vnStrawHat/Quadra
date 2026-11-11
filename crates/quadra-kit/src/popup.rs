//! What every popup of the kit does (specs/13, "Overlays in the kit"): it opens beside what
//! opened it, keeps Tab inside itself, closes on a press outside or on Escape, and gives the
//! focus back to what had it. Menus, selects, popovers and dialogs are this, with content.

use quadra_reactive::{Signal, on_cleanup};
use quadra_tree::{Along, NodeId, Side};
use quadra_ui::{Div, MountCx, NodeRef, Place, View, column, overlay, show};

use crate::action::{Cancel, POPUP};
use crate::{size, token};

/// The plane of an overlay: its own fill, a hairline border, the largest radius.
pub(crate) fn surface() -> Div {
    // ponytail: no shadow yet; the two elevations of the design language come with shadows
    // in node styles.
    column().bg(token::OVERLAY_BG).border(1.0).border_color(token::LINE).radius(size::RADIUS_XL)
}

/// A popup while it is open.
struct Open<V> {
    open: Signal<bool>,
    place: Place,
    opener: Option<NodeRef>,
    content: V,
}

impl<V: View + 'static> View for Open<V> {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let (open, panel_ref) = (self.open, NodeRef::new());
        // The panel can have the focus itself, for a popup that holds nothing that can.
        let panel = surface().focus_scope().tab_index(-1).key_context(POPUP).node_ref(&panel_ref);
        let panel = panel.on_action::<Cancel>(move |_| open.set(false)).child(self.content);
        let shown = overlay(self.place, panel).on_outside_press(move || open.set(false));
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
    popup(open, Place::Beside(opener.clone(), Side::Below, Along::Start, 4.0), Some(opener.clone()), content)
}

/// A popup at a place of the caller's choosing: beside a node on another side, at a point,
/// or in the middle of the window.
pub(crate) fn popup<V, F>(
    open: Signal<bool>,
    place: Place,
    opener: Option<NodeRef>,
    content: F,
) -> impl View + use<V, F>
where
    V: View + 'static,
    F: Fn() -> V + 'static,
{
    show(
        move || open.get(),
        move || Open { open, place: place.clone(), opener: opener.clone(), content: content() },
    )
}

//! The overlay view (specs/05, "Overlays"): content above the window's main tree, placed
//! against an anchor. A menu, a popover, a tooltip, a dialog.

use quadra_layout::Display;
use quadra_prim::Point;
use quadra_reactive::on_cleanup;
use quadra_tree::{Along, Anchor, NodeId, Side, Style};

use crate::node_ref::NodeRef;
use crate::view::{AnyView, MountCx, View};

/// Where an overlay opens.
#[derive(Clone)]
pub enum Place {
    /// Beside the node of a [`NodeRef`]: on a side of it, aligned along that side, a gap away.
    /// It opens on the other side when it only fits there.
    Beside(NodeRef, Side, Along, f32),
    /// Its corner at a point of the window: a context menu.
    At(Point),
    /// In the middle of the window: a dialog.
    Center,
    /// Centred across the window, this far down from its top.
    Top(f32),
    /// Centred across the window, this far up from its bottom.
    Bottom(f32),
    /// In the bottom right corner, this far from the right edge and from the bottom.
    BottomRight(f32, f32),
    /// In the top right corner, this far from the right edge and from the top.
    TopRight(f32, f32),
}

pub struct OverlayView {
    place: Place,
    content: AnyView,
    outside: Option<Box<dyn FnMut()>>,
    /// A press on this node is not one outside the overlay: what opens and closes it.
    opener: Option<NodeRef>,
}

/// Shows `content` above everything else in the window, sized to itself and placed against
/// its anchor after every layout. The overlay is open while this view is mounted: put it in a
/// `show` to open and close it. Where it stands in the flow it takes no room.
pub fn overlay(place: Place, content: impl View + 'static) -> OverlayView {
    OverlayView { place, content: AnyView::new(content), outside: None, opener: None }
}

impl OverlayView {
    /// Runs `f` when a press lands outside the overlay: where a menu closes itself.
    pub fn on_outside_press(mut self, f: impl FnMut() + 'static) -> Self {
        self.outside = Some(Box::new(f));
        self
    }

    /// Names what opens and closes the overlay: a press on it is not a press outside, so the
    /// button that opened a menu closes it by its own click and does not open it again.
    pub fn opener(mut self, opener: &NodeRef) -> Self {
        self.opener = Some(opener.clone());
        self
    }
}

impl View for OverlayView {
    fn mount(self, cx: &mut MountCx) -> NodeId {
        let root = self.content.mount(cx);
        let anchor = match self.place {
            Place::Beside(node, side, along, gap) => match node.get() {
                Some(node) => Anchor::Beside { node, side, along, gap },
                None => Anchor::Center, // the anchor was never mounted
            },
            Place::At(point) => Anchor::At(point),
            Place::Center => Anchor::Center,
            Place::Top(down) => Anchor::Top(down),
            Place::Bottom(up) => Anchor::Bottom(up),
            Place::BottomRight(left, up) => Anchor::BottomRight(left, up),
            Place::TopRight(left, down) => Anchor::TopRight(left, down),
        };
        cx.tree.open_overlay(root, anchor);
        if let Some(mut outside) = self.outside {
            let opener = self.opener;
            cx.tree.on_outside_press(root, move |path| {
                let on_opener =
                    opener.as_ref().and_then(NodeRef::get).is_some_and(|node| path.contains(&node));
                if !on_opener {
                    outside();
                }
            });
        }
        // It closes with the scope it was opened in: the part of the view that showed it.
        let handle = cx.handle.clone();
        on_cleanup(move || handle.with(move |tree| tree.close_overlay(root)));
        let nothing = quadra_layout::Style { display: Display::None, ..Default::default() };
        cx.tree.insert(Style { layout: nothing, ..Style::default() })
    }
}

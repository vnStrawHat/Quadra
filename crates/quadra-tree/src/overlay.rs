//! Overlays (specs/05, "Overlays"; specs/06, "Overlays"): roots above the main tree, each a
//! layer of its own, sized to its content and placed against an anchor after the main layout.
//! Menus, popovers, tooltips and dialogs are overlays.

use quadra_prim::{Point, Rect, Size};

use crate::input::Handler;
use crate::node::{NodeId, flag};
use crate::tree::Tree;

/// The side of its anchor an overlay opens on. It opens on the other side when it does not
/// fit there and does on the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Below,
    Above,
    Right,
    Left,
}

/// How an overlay is aligned along the side it opens on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Along {
    Start,
    Center,
    End,
}

/// Where an overlay goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Anchor {
    /// Beside a node: on a side of it, aligned along that side, `gap` pixels away.
    Beside { node: NodeId, side: Side, along: Along, gap: f32 },
    /// Its corner at a point of the window: a context menu.
    At(Point),
    /// In the middle of the window: a dialog.
    Center,
    /// Its corner at a point of the window, wherever that leaves it: what follows the
    /// pointer in a drag goes out of the window with it.
    Free(Point),
    /// Centred across the window, this far down from its top: pickers and dialogs.
    Top(f32),
}

pub(crate) struct Overlay {
    pub root: NodeId,
    anchor: Anchor,
}

/// The corner of an overlay of `size` in a `window`: against its anchor, on the other side
/// where it does not fit, and moved as far as needed to stay inside the window.
fn place(anchor: Anchor, target: Option<Rect>, size: Size, window: Size) -> Point {
    let inside = |v: f32, len: f32, room: f32| v.min(room - len).max(0.0);
    // Along an axis: after the anchor, or before it when only that fits.
    let beside = |before: f32, after: f32, len: f32, room: f32, first_after: bool| {
        let (fits_after, fits_before) = (after + len <= room, before - len >= 0.0);
        // The side asked for, unless only the other one has room.
        let other_only = if first_after { !fits_after && fits_before } else { !fits_before && fits_after };
        if first_after != other_only { after } else { before - len }
    };
    if let Anchor::Free(at) = anchor {
        return at;
    }
    let (x, y) = match (anchor, target) {
        (Anchor::Beside { side, along, gap, .. }, Some(a)) => {
            let aligned = |start: f32, len: f32, own: f32| match along {
                Along::Start => start,
                Along::Center => start + (len - own) / 2.0,
                Along::End => start + len - own,
            };
            match side {
                Side::Below | Side::Above => (
                    aligned(a.x, a.width, size.width),
                    beside(a.y - gap, a.bottom() + gap, size.height, window.height, side == Side::Below),
                ),
                Side::Right | Side::Left => (
                    beside(a.x - gap, a.right() + gap, size.width, window.width, side == Side::Right),
                    aligned(a.y, a.height, size.height),
                ),
            }
        }
        (Anchor::At(p), _) => (
            beside(p.x, p.x, size.width, window.width, true),
            beside(p.y, p.y, size.height, window.height, true),
        ),
        (Anchor::Top(down), _) => ((window.width - size.width) / 2.0, down),
        _ => ((window.width - size.width) / 2.0, (window.height - size.height) / 2.0),
    };
    Point::new(inside(x, size.width, window.width).round(), inside(y, size.height, window.height).round())
}

impl Tree {
    /// Makes a node the root of an overlay, on top of the ones already open. It is sized to
    /// its content, at most the window, and placed against its anchor after every layout.
    pub fn open_overlay(&mut self, root: NodeId, anchor: Anchor) {
        if self.get(root).is_none() || self.overlays.iter().any(|o| o.root == root) {
            return;
        }
        self.make_layer(root, false);
        self.overlays.push(Overlay { root, anchor });
        self.text_epoch += 1; // the text values of the window hold in it from now on
        self.sync_roots();
        self.relayout = true;
        self.mark(root, flag::PAINT | flag::SUBTREE);
        self.overlays_changed();
    }

    /// For a listener, the overlays are under the window.
    fn overlays_changed(&mut self) {
        if let Some(window) = self.root {
            self.semantics_changed(window);
        }
    }

    /// Puts an open overlay somewhere else. Only its layer's offset changes: nothing of it
    /// is laid out or painted again.
    pub fn move_overlay(&mut self, root: NodeId, anchor: Anchor) {
        if let Some(overlay) = self.overlays.iter_mut().find(|o| o.root == root) {
            overlay.anchor = anchor;
            self.shared.wake.set(true); // placed in the next frame
        }
    }

    /// Closes an overlay: its root and everything under it is removed.
    pub fn close_overlay(&mut self, root: NodeId) {
        if self.overlays.iter().any(|o| o.root == root) {
            self.remove(root);
        }
    }

    /// The roots of the open overlays, the lowest first.
    pub fn overlays(&self) -> impl DoubleEndedIterator<Item = NodeId> + '_ {
        self.overlays.iter().map(|o| o.root)
    }

    /// Runs `f` when a press lands outside an overlay: how a menu knows to close. `f` is given
    /// the nodes the press is on, the deepest first: a press on what opened the menu is that
    /// node's to deal with.
    pub fn on_outside_press(&mut self, root: NodeId, f: impl FnMut(&[NodeId]) + 'static) {
        self.add_handler(root, Handler::OutsidePress(Box::new(f)));
    }

    /// Tells the overlays a press missed: the top one first.
    pub(crate) fn press_outside(&mut self, path: &[NodeId]) {
        let missed: Vec<NodeId> = self.overlays().rev().filter(|root| !path.contains(root)).collect();
        for root in missed {
            if !self.is_alive(root) {
                continue; // closed by what an overlay above it did
            }
            for handler in &mut self.nodes[root.index as usize].handlers {
                if let Handler::OutsidePress(run) = handler {
                    run(path);
                }
            }
        }
    }

    /// A removed node was an overlay's root: it leaves the stack.
    pub(crate) fn forget_overlay(&mut self, root: NodeId) {
        let before = self.overlays.len();
        self.overlays.retain(|o| o.root != root);
        if self.overlays.len() != before {
            self.sync_roots();
            self.overlays_changed();
        }
    }

    /// The scene draws the main tree's layer, then the overlays' in stack order.
    fn sync_roots(&mut self) {
        let layer = |id: NodeId| self.nodes[id.index as usize].layer.as_ref().map(|s| s.id);
        let mut roots = vec![self.root_layer];
        roots.extend(self.overlays.iter().filter_map(|o| layer(o.root)));
        self.scene.set_roots(&roots);
    }

    /// After a layout: every overlay is put where its anchor is now. An overlay that only
    /// moves changes its layer's offset: nothing is laid out and nothing painted for it.
    pub(crate) fn place_overlays(&mut self) {
        for i in 0..self.overlays.len() {
            let Overlay { root, anchor } = self.overlays[i];
            let target = match anchor {
                Anchor::Beside { node, .. } if self.is_alive(node) => Some(self.window_rect(node)),
                _ => None,
            };
            let at = place(anchor, target, self.rect(root).size(), self.size);
            self.set_layer_offset(root, at);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Size = Size { width: 800.0, height: 600.0 };

    fn beside(side: Side, along: Along, anchor: Rect, size: Size) -> Point {
        let at = Anchor::Beside { node: NodeId { index: 0, generation: 0 }, side, along, gap: 4.0 };
        place(at, Some(anchor), size, WINDOW)
    }

    #[test]
    fn an_overlay_opens_beside_its_anchor_or_on_the_other_side_and_stays_in_the_window() {
        let (button, menu) = (Rect::new(100.0, 50.0, 80.0, 20.0), Size::new(120.0, 200.0));
        assert_eq!(beside(Side::Below, Along::Start, button, menu), Point::new(100.0, 74.0));
        assert_eq!(beside(Side::Below, Along::Center, button, menu), Point::new(80.0, 74.0));
        assert_eq!(beside(Side::Below, Along::End, button, menu), Point::new(60.0, 74.0));
        assert_eq!(beside(Side::Right, Along::Start, button, menu), Point::new(184.0, 50.0));
        // No room above (50 px): it opens below. No room below a button near the bottom: above.
        assert_eq!(beside(Side::Above, Along::Start, button, menu), Point::new(100.0, 74.0));
        let low = Rect::new(100.0, 560.0, 80.0, 20.0);
        assert_eq!(beside(Side::Below, Along::Start, low, menu), Point::new(100.0, 356.0));
        // Near the right edge it is moved left to fit; one larger than the window starts at 0.
        let right = Rect::new(760.0, 50.0, 30.0, 20.0);
        assert_eq!(beside(Side::Below, Along::Start, right, menu), Point::new(680.0, 74.0));
        assert_eq!(beside(Side::Below, Along::Start, button, Size::new(900.0, 700.0)), Point::ZERO);

        assert_eq!(place(Anchor::At(Point::new(300.0, 200.0)), None, menu, WINDOW), Point::new(300.0, 200.0));
        assert_eq!(place(Anchor::At(Point::new(750.0, 500.0)), None, menu, WINDOW), Point::new(630.0, 300.0));
        assert_eq!(place(Anchor::Center, None, menu, WINDOW), Point::new(340.0, 200.0));
    }
}

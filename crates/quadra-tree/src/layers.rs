//! Nodes that are layers, and the compose pass that places them (specs/07, "Layers").
//!
//! A layer's space starts at its root's border box, so a layer that moves keeps every
//! instance: only its transform changes. A scroll container's content is such a node, cut to
//! its parent's box, and scrolling is its offset.

use quadra_prim::{Affine, Point, Rect, Size};
use quadra_render::{Backdrop, LayerConstants, LayerId, Mask};

use crate::node::{NodeId, Pose, flag};
use crate::tree::Tree;

/// What makes a node a layer.
pub(crate) struct LayerState {
    pub id: LayerId,
    /// Added to the node's place: a scroll offset, a slide. Logical pixels.
    pub offset: Point,
    pub opacity: f32,
    /// How far the layer is turned about its middle, in degrees.
    pub turn: f32,
    /// The blur of what is behind the node's box, in logical pixels: 0 none.
    pub backdrop: f32,
    /// The shape it is cut to, if any.
    pub clip: Option<Box<crate::mask::ClipShape>>,
    /// What the node's style says of where its paint goes (specs/05): over the rest.
    pub pose: Pose,
    /// Cut to the parent's padding box: the content of a scroll container.
    pub clip_to_parent: bool,
    /// Its marker stands in the list of the layer around it.
    pub marked: bool,
    /// Paint only what is near the part that shows (specs/07, "Layers").
    pub culls: bool,
    /// The part of the layer, in its own space, that is painted when it culls.
    pub band: Option<Rect>,
    /// Painted at the scale it shows rather than scaled by its transform (`.sharp_scale()`).
    pub sharp: bool,
    /// The node's range in its own layer (`len` with its subtree, `own` its own instances).
    /// The node's `start`, `len` and `own` describe its marker in the parent's layer.
    pub len: u32,
    pub own: u32,
}

impl LayerState {
    pub fn new(id: LayerId) -> LayerState {
        LayerState {
            id,
            offset: Point::ZERO,
            opacity: 1.0,
            turn: 0.0,
            backdrop: 0.0,
            clip: None,
            pose: Pose::default(),
            clip_to_parent: false,
            marked: false,
            culls: false,
            band: None,
            sharp: false,
            len: 0,
            own: 0,
        }
    }
}

impl Tree {
    /// Makes a node a layer: it and what is under it are painted into a display list of their
    /// own, placed by a transform. With `clip_to_parent` the layer is cut to the parent's box.
    pub fn make_layer(&mut self, id: NodeId, clip_to_parent: bool) {
        let Some(node) = self.get(id) else { return };
        if node.layer.is_some() {
            return;
        }
        let layer = self.scene.add_layer();
        self.nodes[id.index as usize].layer =
            Some(Box::new(LayerState { clip_to_parent, ..LayerState::new(layer) }));
        // Its instances leave the parent's list for a marker, and are painted into the layer.
        self.mark(id, flag::PAINT | flag::SUBTREE | flag::RELIST);
        self.recompose.push(id);
    }

    /// Puts a layer's content back into the layer around it. The root stays a layer.
    pub fn remove_layer(&mut self, id: NodeId) {
        if self.root == Some(id) {
            return;
        }
        let Some(state) = self.get_mut(id).and_then(|n| n.layer.take()) else { return };
        self.scene.remove_layer(state.id);
        if state.offset != Point::ZERO {
            self.semantics_changed(id); // what is under it is back where it is laid out
        }
        // Nothing of it is in the parent's list but the marker.
        let node = &mut self.nodes[id.index as usize];
        (node.len, node.own) = (0, 0);
        self.mark(id, flag::PAINT | flag::SUBTREE | flag::RELIST);
    }

    /// Makes a node a layer painted at the scale its pose shows, not scaled by its transform:
    /// its text is rasterized at the size it is seen. A change of the scale paints it again.
    pub fn sharp_scale(&mut self, id: NodeId) {
        self.make_layer(id, false);
        let Some(state) = self.get_mut(id).and_then(|n| n.layer.as_mut()) else { return };
        if !std::mem::replace(&mut state.sharp, true) && state.pose.scale != 1.0 {
            self.mark(id, flag::PAINT | flag::SUBTREE);
            self.recompose.push(id);
        }
    }

    /// Device pixels per logical pixel in a layer's own space: the window's, times the
    /// layer's scale when it is painted at it.
    // ponytail: a layer inside a sharp one is placed and drawn at the window's scale; fold
    // the outer scale in when sharp layers hold layers of their own.
    pub(crate) fn paint_scale(&self, id: NodeId) -> f32 {
        let sharp = self.get(id).and_then(|n| n.layer.as_ref()).filter(|state| state.sharp);
        self.scale * sharp.map_or(1.0, |state| state.pose.scale)
    }

    /// Makes a layer that is cut to its parent paint only what is near the part that shows:
    /// the content of a scroll container.
    pub fn cull_layer(&mut self, id: NodeId) {
        let Some(state) = self.get_mut(id).and_then(|n| n.layer.as_mut()) else { return };
        if !std::mem::replace(&mut state.culls, true) {
            self.culled.push(id);
        }
    }

    /// The part of a layer its parent's padding box shows, in the layer's own space.
    fn layer_visible(&self, id: NodeId) -> Option<Rect> {
        let node = &self.nodes[id.index as usize];
        let (state, parent) = (node.layer.as_ref()?, &self.nodes[node.parent?.index as usize]);
        let own = self.layout.rect(node.layout).origin();
        let size = self.layout.rect(parent.layout).size();
        let shown =
            Rect::new(0.0, 0.0, size.width, size.height).inset(self.layout.style(parent.layout).border);
        Some(shown.translate(-own.x - state.offset.x, -own.y - state.offset.y))
    }

    /// Before the paint pass: a culling layer whose visible part left the band it was painted
    /// for gets a band around that part, and is painted again.
    pub(crate) fn cull_pass(&mut self) {
        let mut list = std::mem::take(&mut self.culled);
        list.retain(|&id| self.get(id).is_some_and(|n| n.layer.as_ref().is_some_and(|s| s.culls)));
        for &id in &list {
            let Some(shown) = self.layer_visible(id) else { continue };
            let Some(state) = self.nodes[id.index as usize].layer.as_mut() else { continue };
            let covers = |band: Rect| {
                band.x <= shown.x
                    && band.y <= shown.y
                    && band.right() >= shown.right()
                    && band.bottom() >= shown.bottom()
            };
            if !state.band.is_some_and(covers) {
                let (w, h) = (shown.width, shown.height);
                state.band = Some(Rect::new(shown.x - w, shown.y - h, w * 3.0, h * 3.0));
                self.mark(id, flag::PAINT | flag::SUBTREE);
            }
        }
        self.culled = list;
    }

    pub fn is_layer(&self, id: NodeId) -> bool {
        self.get(id).is_some_and(|n| n.layer.is_some())
    }

    /// Moves a layer by an offset in logical pixels: scrolling, a slide. Nothing is painted.
    pub fn set_layer_offset(&mut self, id: NodeId, offset: Point) {
        let Some(state) = self.get_mut(id).and_then(|n| n.layer.as_mut()) else { return };
        if state.offset != offset {
            state.offset = offset;
            self.recompose.push(id);
            self.semantics_changed(id); // what is under it is elsewhere in the window
        }
    }

    /// Fades a layer. Nothing is painted.
    pub fn set_layer_opacity(&mut self, id: NodeId, opacity: f32) {
        let Some(state) = self.get_mut(id).and_then(|n| n.layer.as_mut()) else { return };
        if state.opacity != opacity {
            state.opacity = opacity;
            self.recompose.push(id);
        }
    }

    /// Turns a layer about its middle, by degrees: a spinner. Nothing is painted, and the
    /// pointer goes by where the layer would be unturned.
    pub fn set_layer_turn(&mut self, id: NodeId, degrees: f32) {
        let Some(state) = self.get_mut(id).and_then(|n| n.layer.as_mut()) else { return };
        if state.turn != degrees {
            state.turn = degrees;
            self.recompose.push(id);
        }
    }

    /// Blurs what is behind a node's box under it, by `blur` logical pixels; 0 stops. The
    /// node becomes a layer: its constants say where the backdrop is taken.
    pub fn set_backdrop(&mut self, id: NodeId, blur: f32) {
        if blur > 0.0 {
            self.make_layer(id, false);
        }
        let Some(state) = self.get_mut(id).and_then(|n| n.layer.as_mut()) else { return };
        if state.backdrop != blur {
            state.backdrop = blur;
            self.recompose.push(id);
            self.mark(id, flag::PAINT);
        }
    }

    /// The compose pass: gives the renderer the constants of the layers whose place, offset or
    /// opacity changed. No widget code runs.
    pub(crate) fn compose_pass(&mut self) {
        let list = std::mem::take(&mut self.recompose);
        for &id in &list {
            self.refresh_mask(id);
            let Some(layer) = self.get(id).and_then(|n| n.layer.as_ref()).map(|s| s.id) else { continue };
            let constants = self.layer_constants(id);
            let layer = self.scene.layer_mut(layer);
            if layer.constants != constants {
                layer.constants = constants;
                self.stats.layers_composed += 1;
            }
        }
        self.recompose = list;
        self.recompose.clear();
    }

    /// What a layer's constants must be: its place in the layer around it, its offset, its
    /// cut and its opacity, in device pixels of that layer.
    pub(crate) fn layer_constants(&self, id: NodeId) -> LayerConstants {
        let node = &self.nodes[id.index as usize];
        let Some(state) = node.layer.as_ref().filter(|_| self.root != Some(id)) else {
            return LayerConstants::default();
        };
        // Its place: the origins of the node and of its ancestors below the layer around it.
        let own = self.layout.rect(node.layout).origin();
        let mut parent_at = Point::ZERO;
        let mut up = node.parent;
        while let Some(parent) = up.filter(|p| self.nodes[p.index as usize].layer.is_none()) {
            let origin = self.layout.rect(self.nodes[parent.index as usize].layout).origin();
            parent_at = Point::new(parent_at.x + origin.x, parent_at.y + origin.y);
            up = self.nodes[parent.index as usize].parent;
        }
        let s = self.scale;
        // Whole device pixels: glyphs keep their positions inside the layer while it moves.
        let pose = state.pose;
        let x = ((parent_at.x + own.x + state.offset.x + pose.translate.x) * s).round();
        let y = ((parent_at.y + own.y + state.offset.y + pose.translate.y) * s).round();
        let scissor = node.parent.filter(|_| state.clip_to_parent).map(|parent| {
            let p = &self.nodes[parent.index as usize];
            let border = self.layout.style(p.layout).border;
            let inner = Rect::new(parent_at.x, parent_at.y, 0.0, 0.0);
            let size = self.layout.rect(p.layout).size();
            let padding_box = Rect::new(inner.x, inner.y, size.width, size.height).inset(border);
            let (left, top) = ((padding_box.x * s).round(), (padding_box.y * s).round());
            let (right, bottom) = ((padding_box.right() * s).round(), (padding_box.bottom() * s).round());
            Rect::new(left, top, (right - left).max(0.0), (bottom - top).max(0.0))
        });
        let mut transform = Affine::translate(x, y);
        let (turn, k) = (state.turn + pose.rotate, pose.scale);
        // A sharp layer is painted at its scale already: only the move that keeps its middle.
        let (drawn, grow) = if state.sharp { (k, 1.0) } else { (1.0, k) };
        if turn != 0.0 || k != 1.0 {
            let size = self.layout.rect(node.layout).size();
            let middle = Point::new(size.width * s / 2.0, size.height * s / 2.0);
            let (mx, my) = (middle.x * (1.0 - k), middle.y * (1.0 - k));
            // Whole device pixels when it is not turned, so its glyphs stay on the grid.
            let (mx, my) = if state.sharp && turn == 0.0 { (mx.round(), my.round()) } else { (mx, my) };
            // Turned, then scaled, both about the middle: the middle stays where it is.
            let grown = Affine::translate(mx, my).then(Affine::scale(grow));
            transform = transform.then(Affine::rotate_about(turn, middle)).then(grown);
        }
        let backdrop = (state.backdrop > 0.0).then(|| {
            let size = self.layout.rect(node.layout).size();
            Backdrop {
                size: Size::new(size.width * s * drawn, size.height * s * drawn),
                blur: state.backdrop * s,
            }
        });
        let mask = state.clip.as_ref().and_then(|clip| self.mask_of(clip)).map(|mask| {
            let size = self.layout.rect(node.layout).size();
            Mask { size: Size::new(size.width * s * drawn, size.height * s * drawn), ..mask }
        });
        LayerConstants { transform, scissor, opacity: state.opacity * pose.opacity, backdrop, mask }
    }
}

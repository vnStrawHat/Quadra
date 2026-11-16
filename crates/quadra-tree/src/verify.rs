//! The check that incremental paint is exact (specs/07, "Display lists"): everything painted
//! from scratch must be what the passes left.

use quadra_layout::Display;
use quadra_prim::{Point, Rect};
use quadra_render::Instance;

use crate::fonts::Fonts;
use crate::node::NodeId;
use crate::paint::{outside, paint_own};
use crate::tree::Tree;

impl Tree {
    /// Paints every layer from scratch and compares it with what the incremental passes left:
    /// the instances, each node's range, and each layer's constants. Returns the first
    /// difference, or `None`. Call it after a frame.
    pub fn verify_paint(&mut self) -> Option<String> {
        let fonts = self.fonts.clone();
        let fonts = &mut *fonts.borrow_mut();
        let mut layers = Vec::from_iter(self.root);
        layers.extend(self.overlays());
        let mut wrong = None;
        while let Some(root) = layers.pop() {
            let i = root.index as usize;
            let Some(state) = self.nodes[i].layer.as_ref() else { continue };
            let (layer, len, own, band) =
                (state.id, state.len, state.own, state.band.filter(|_| state.culls));
            let origin = self.layout.rect(self.nodes[i].layout).origin();
            let mut fresh = Vec::new();
            // The root's own range is kept in its layer state: check it apart from the walk.
            let as_child = (self.nodes[i].start, self.nodes[i].len, self.nodes[i].own);
            (self.nodes[i].start, self.nodes[i].len, self.nodes[i].own) = (0, len, own);
            self.verify_node(
                (root, band, false),
                root,
                Point::new(-origin.x, -origin.y),
                0,
                fonts,
                &mut fresh,
                &mut layers,
                &mut wrong,
            );
            (self.nodes[i].start, self.nodes[i].len, self.nodes[i].own) = as_child;
            let have = self.scene.layer(layer).instances();
            if have.len() != fresh.len() {
                let (a, b) = (have.len(), fresh.len());
                return Some(format!("the layer of node {i} has {a} instances, from scratch {b}"));
            }
            if let Some(at) = have.iter().zip(&fresh).position(|(a, b)| a != b) {
                return Some(format!(
                    "instance {at} of the layer of node {i} differs from the one painted from scratch"
                ));
            }
            let (have, expected) = (self.scene.layer(layer).constants, self.layer_constants(root));
            if have != expected {
                return Some(format!("the layer of node {i} has constants {have:?}, expected {expected:?}"));
            }
        }
        wrong
    }

    #[allow(clippy::too_many_arguments)]
    fn verify_node(
        &mut self,
        (id, band, culled): (NodeId, Option<Rect>, bool),
        root: NodeId,
        origin: Point,
        parent_start: usize,
        fonts: &mut Fonts,
        out: &mut Vec<Instance>,
        layers: &mut Vec<NodeId>,
        wrong: &mut Option<String>,
    ) {
        let i = id.index as usize;
        let shown = self.layout.style(self.nodes[i].layout).display != Display::None;
        let start = out.len();
        let own;
        if let Some(state) = self.nodes[i].layer.as_ref().filter(|_| id != root) {
            // Another layer: one marker here, and its content is checked as a layer of its own.
            if shown {
                out.push(Instance::marker(state.id));
                layers.push(id);
            }
            own = out.len() - start;
        } else {
            let rect = self.layout.rect(self.nodes[i].layout).translate(origin.x, origin.y);
            // Outside the band a node paints nothing, nor what it holds.
            let culled = culled || outside(band, rect);
            if !culled {
                let theme = self.theme_at(id).clone();
                let how = (self.scale, band, self.text_at(), &*theme);
                paint_own(&mut self.nodes, i, &self.layout, rect, how, fonts, out);
                // The room a shown node keeps holds empty instances after what it painted.
                if shown {
                    out.resize(out.len().max(start + self.nodes[i].own as usize), Instance::default());
                }
            }
            own = out.len() - start;
            if shown {
                for k in 0..self.nodes[i].children.len() {
                    let child = self.nodes[i].children[k];
                    let at = (child, band, culled);
                    self.verify_node(at, root, rect.origin(), start, fonts, out, layers, wrong);
                }
            }
        }
        let node = &self.nodes[i];
        let expected = ((start - parent_start) as u32, (out.len() - start) as u32, own as u32);
        if (node.start, node.len, node.own) != expected && wrong.is_none() {
            let have = (node.start, node.len, node.own);
            *wrong =
                Some(format!("node {i}: range (start, len, own) is {have:?}, from scratch {expected:?}"));
        }
    }
}

//! Layers cut to a shape (specs/07, "Layers"): the shape rasterized as coverage at the size of
//! the node's box, a tile of the coverage atlas the node holds, named by its layer's constants.

use std::rc::Rc;

use quadra_render::Mask;
use quadra_vector::Svg;

use crate::fonts::{GlyphTile, SLICE};
use crate::node::{NodeId, flag};
use crate::tree::Tree;

/// The shape a layer is cut to, and the tile of its mask at the box's size.
pub(crate) struct ClipShape {
    svg: Rc<Svg>,
    shape: u64,
    tile: Option<GlyphTile>,
}

impl Tree {
    /// Cuts a node, and all that is under it, to a shape fitted to its box (as an icon is
    /// fitted to its own); `None` stops. The node becomes a layer.
    pub fn set_clip_shape(&mut self, id: NodeId, svg: Option<Rc<Svg>>) {
        if svg.is_some() {
            self.make_layer(id, false);
        }
        let Some(state) = self.get_mut(id).and_then(|n| n.layer.as_mut()) else { return };
        state.clip = svg.map(|svg| Box::new(ClipShape { shape: svg.hash(), svg, tile: None }));
        self.recompose.push(id);
        self.mark(id, flag::PAINT);
    }

    /// Rasterizes the mask of a layer cut to a shape at its box's size in device pixels, if it
    /// has none of that size: no larger than an atlas slice, so a larger box has it at a
    /// lower resolution.
    pub(crate) fn refresh_mask(&mut self, id: NodeId) {
        // A node removed since it asked to be composed has nothing to cut.
        let Some(node) = self.get(id) else { return };
        let Some(clip) = node.layer.as_ref().and_then(|state| state.clip.as_ref()) else { return };
        let size = self.layout.rect(node.layout).size();
        let (w, h) =
            ((size.width * self.scale).round().max(1.0), (size.height * self.scale).round().max(1.0));
        let k = (SLICE as f32 / w.max(h)).min(1.0);
        let (w, h) = ((w * k).round().max(1.0) as u32, (h * k).round().max(1.0) as u32);
        let (svg, shape) = (clip.svg.clone(), clip.shape);
        let tile = self.fonts.borrow_mut().mask(shape, w, h, || svg.render_mask(w as usize, h as usize));
        if let Some(clip) = self.nodes[id.index as usize].layer.as_mut().and_then(|state| state.clip.as_mut())
        {
            clip.tile = tile;
        }
    }

    /// The mask of a layer's constants: its tile over the box, a texel a device pixel when it
    /// is as large as the box, so each pixel samples the middle of its own texel.
    pub(crate) fn mask_of(&self, clip: &ClipShape) -> Option<Mask> {
        let tile = clip.tile.as_ref().filter(|tile| tile.rect.w > 0)?;
        Some(Mask { uv: tile.uv, slice: tile.rect.slice, size: Default::default() })
    }

    /// After the GPU lost its tiles: every mask is rasterized again with the next compose.
    pub(crate) fn remask_all(&mut self) {
        for i in 0..self.nodes.len() {
            let node = &self.nodes[i];
            if node.alive && node.layer.as_ref().is_some_and(|state| state.clip.is_some()) {
                let id = NodeId { index: i as u32, generation: node.generation };
                self.recompose.push(id);
            }
        }
    }
}

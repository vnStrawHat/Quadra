//! A box's gradient (specs/07, "Instances"): the node keeps the gradient and the handle of its
//! ramp in the colour atlas, which keeps the ramp there while the node paints with it.

use quadra_render::{Gradient, RAMP, Ramp};

use crate::fonts::{Fonts, GlyphTile};
use crate::node::{NodeId, flag};
use crate::tree::Tree;

pub(crate) struct Fill {
    pub gradient: Gradient,
    tile: Option<GlyphTile>,
}

impl Fill {
    /// The ramp to paint with, baked the first time its stops are asked for.
    pub fn ramp(&mut self, fonts: &mut Fonts) -> Option<Ramp> {
        let gradient = &self.gradient;
        let tile = fonts.ramp(gradient.key(), || gradient.ramp())?;
        let ([u0, v0, u1, v1], slice) = (tile.uv, tile.rect.slice);
        let (half, row) = ((u1 - u0) / RAMP as f32 / 2.0, (v0 + v1) / 2.0);
        self.tile = Some(tile);
        Some(Ramp { kind: gradient.kind, uv: [u0 + half, row, u1 - half, row], slice })
    }
}

impl Tree {
    /// Fills a node's box with a gradient over its background, or with none. Nothing happens
    /// if it is the same or the node is gone.
    pub fn set_gradient(&mut self, id: NodeId, gradient: Option<Gradient>) {
        let Some(node) = self.get_mut(id) else { return };
        if node.fill.as_ref().map(|fill| &fill.gradient) == gradient.as_ref() {
            return;
        }
        node.fill = gradient.map(|gradient| Box::new(Fill { gradient, tile: None }));
        self.mark(id, flag::PAINT);
    }
}

//! The drawing widget: a vector drawing painted as a mask in the text colour that holds at
//! its node (specs/07, "Atlases"): an icon. It is as large as its node's style says, and is
//! rasterized once for each size in device pixels it is shown at.

use std::rc::Rc;

use quadra_layout::Known;
use quadra_prim::{Color, Corners, Rect, Size};
use quadra_render::{Kind, Sprite};
use quadra_vector::Svg;

use crate::fonts::GlyphTile;
use crate::inherit::Inherited;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// The side of a drawing whose node does not say its size, in pixels.
const DEFAULT_SIDE: f32 = 16.0;

pub struct Drawing {
    svg: Rc<Svg>,
    /// What the atlas knows the drawing by.
    shape: u64,
    color: Color,
    /// The tile last painted: holding it keeps it in the atlas.
    tile: Option<GlyphTile>,
}

impl Drawing {
    pub fn new(svg: Rc<Svg>) -> Drawing {
        let shape = svg.hash();
        Drawing { svg, shape, color: Color::BLACK, tile: None }
    }
}

impl Widget for Drawing {
    fn is_leaf(&self) -> bool {
        true
    }

    fn measure(&mut self, _cx: &mut MeasureCx, known: Known, _avail: Size) -> Size {
        Size::new(known.width.unwrap_or(DEFAULT_SIDE), known.height.unwrap_or(DEFAULT_SIDE))
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        // Whole device pixels, at a whole pixel: the mask is drawn one texel a pixel.
        let bounds = cx.device(cx.rect);
        let (left, top) = (bounds.x.round(), bounds.y.round());
        let (width, height) = (bounds.width.round() as u32, bounds.height.round() as u32);
        let svg = &self.svg;
        self.tile =
            cx.fonts.mask(self.shape, width, height, || svg.render_mask(width as usize, height as usize));
        let Some(tile) = self.tile.clone().filter(|tile| tile.rect.w > 0) else { return };
        let sprite = Sprite {
            bounds: Rect::new(left, top, tile.rect.w as f32, tile.rect.h as f32),
            uv: tile.uv,
            slice: tile.rect.slice,
            color: self.color,
            radii: Corners::default(),
            glyph: true,
        };
        cx.push(sprite.instance(Kind::Mono, &cx.clip));
    }

    fn inherit(&mut self, inherited: &Inherited) {
        self.color = inherited.color;
    }
}

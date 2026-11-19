//! The drawing widget: a vector drawing painted as a mask in the text colour that holds at
//! its node (specs/07, "Atlases"): an icon; or, with colours of its own, in them (a logo). It
//! is as large as its node's style says, and is rasterized once for each size in device
//! pixels it is shown at.

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
        let (svg, color) = (&self.svg, self.color);
        let own = !svg.is_single_color();
        self.tile = match own {
            // Its colours: a tile of the colour atlas for the size, and the text's colour.
            true => {
                let key = [
                    width.into(),
                    height.into(),
                    u64::from(
                        color.r.to_bits()
                            ^ color.g.to_bits().rotate_left(8)
                            ^ color.b.to_bits().rotate_left(16)
                            ^ color.a.to_bits().rotate_left(24),
                    ),
                ]
                .iter()
                .fold(self.shape, |h, v| (h ^ v).wrapping_mul(1099511628211));
                let render = || svg.render_rgba(width as usize, height as usize, color);
                cx.fonts.image(key, 0, 0, width, height, render)
            }
            false => {
                cx.fonts.mask(self.shape, width, height, || svg.render_mask(width as usize, height as usize))
            }
        };
        let Some(tile) = self.tile.clone().filter(|tile| tile.rect.w > 0) else { return };
        let sprite = Sprite {
            bounds: Rect::new(left, top, tile.rect.w as f32, tile.rect.h as f32),
            uv: tile.uv,
            slice: tile.rect.slice,
            color: if own { Color::WHITE } else { self.color },
            radii: Corners::default(),
            glyph: !own,
        };
        cx.push(sprite.instance(if own { Kind::Poly } else { Kind::Mono }, &cx.clip));
    }

    fn inherit(&mut self, inherited: &Inherited) {
        self.color = inherited.color;
    }
}

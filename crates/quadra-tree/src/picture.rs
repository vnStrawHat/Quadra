//! The picture widget: decoded pixels painted from tiles of the colour atlas (specs/07,
//! "Atlases"; specs/11, "Images"). An image is cut into pieces no larger than a tile may be,
//! so one of any size shows, and only the pieces that show are uploaded.

use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use quadra_layout::Known;
use quadra_platform::Decoded;
use quadra_prim::{Color, Corners, Rect, Size};
use quadra_render::{Kind, Sprite};

use crate::fonts::GlyphTile;
use crate::widget::{MeasureCx, PaintCx, Widget};

/// The side of a piece, in pixels of the image: four of them fill a slice of the atlas.
const PIECE: u32 = 512;

/// How an image fills a box of another shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fit {
    /// All of it shows, as large as the box allows, in the middle.
    #[default]
    Contain,
    /// It covers the box, in the middle; what sticks out is cut.
    Cover,
    /// It is stretched to the box.
    Fill,
}

/// Decoded pixels, and the number the atlas knows them by.
#[derive(Debug)]
pub struct Image {
    id: u64,
    pub pixels: Decoded,
}

impl Image {
    pub fn new(pixels: Decoded) -> Rc<Image> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Rc::new(Image { id: NEXT.fetch_add(1, Ordering::Relaxed), pixels })
    }

    /// The bytes its pixels take.
    pub fn bytes(&self) -> usize {
        self.pixels.pixels.len()
    }
}

pub struct Picture {
    /// The size of the file's image, which is the picture's own size in logical pixels.
    natural: Option<Size>,
    image: Option<Rc<Image>>,
    fit: Fit,
    radius: f32,
    /// The tiles last painted: holding them keeps them in the atlas.
    tiles: Vec<GlyphTile>,
    /// Told the size in device pixels the image is shown at, each time it is another: who
    /// decodes brings pixels of that size.
    shown: Box<dyn FnMut(u32, u32)>,
    told: (u32, u32),
}

impl Picture {
    pub fn new(fit: Fit, radius: f32, shown: impl FnMut(u32, u32) + 'static) -> Picture {
        let (tiles, shown) = (Vec::new(), Box::new(shown));
        Picture { natural: None, image: None, fit, radius, tiles, shown, told: (0, 0) }
    }

    /// The size of the image, once its header was read. True when that is news: the node
    /// is then laid out again.
    pub fn set_natural(&mut self, size: Size) -> bool {
        self.natural.replace(size) != Some(size)
    }

    /// The pixels to paint, at whatever size they were decoded: they are scaled to the box.
    pub fn set_image(&mut self, image: Rc<Image>) {
        self.image = Some(image);
    }

    /// The pixels it paints.
    pub fn image(&self) -> Option<&Rc<Image>> {
        self.image.as_ref()
    }

    /// Where the image goes in the node's box, both in device pixels.
    fn place(&self, outer: Rect) -> Rect {
        let Some(natural) = self.natural.filter(|n| n.width > 0.0 && n.height > 0.0) else { return outer };
        let (sx, sy) = (outer.width / natural.width, outer.height / natural.height);
        let scale = match self.fit {
            Fit::Fill => return outer,
            Fit::Contain => sx.min(sy),
            Fit::Cover => sx.max(sy),
        };
        let (width, height) = ((natural.width * scale).round(), (natural.height * scale).round());
        let (x, y) = (outer.x + (outer.width - width) / 2.0, outer.y + (outer.height - height) / 2.0);
        Rect::new(x.round(), y.round(), width, height)
    }
}

impl Widget for Picture {
    fn is_leaf(&self) -> bool {
        true
    }

    /// Its own size where nothing says another, no wider than there is room, and its own
    /// shape where one side is said.
    fn measure(&mut self, _cx: &mut MeasureCx, known: Known, avail: Size) -> Size {
        let Some(natural) = self.natural.filter(|n| n.width > 0.0 && n.height > 0.0) else {
            return Size::new(known.width.unwrap_or(0.0), known.height.unwrap_or(0.0));
        };
        let tall = natural.height / natural.width;
        match (known.width, known.height) {
            (Some(width), Some(height)) => Size::new(width, height),
            (Some(width), None) => Size::new(width, width * tall),
            (None, Some(height)) => Size::new(height / tall, height),
            (None, None) => {
                let width = natural.width.min(avail.width);
                Size::new(width, width * tall)
            }
        }
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        let outer = cx.device(cx.rect);
        let dest = self.place(outer);
        // Never more pixels than the file has: larger, it is scaled as it is drawn.
        let natural = self.natural.unwrap_or(Size::new(dest.width, dest.height));
        let want = (dest.width.min(natural.width) as u32, dest.height.min(natural.height) as u32);
        if want.0 > 0 && want.1 > 0 && std::mem::replace(&mut self.told, want) != want {
            (self.shown)(want.0, want.1);
        }
        let Some(image) = self.image.clone() else { return };
        let pixels = &image.pixels;
        let (sx, sy) = (dest.width / pixels.width as f32, dest.height / pixels.height as f32);
        let exact = sx == 1.0 && sy == 1.0;
        let clip = cx.clip.then(outer, self.radius * cx.scale);
        let s = cx.scale;
        let visible = cx.visible.map(|v| Rect::new(v.x * s, v.y * s, v.width * s, v.height * s));
        let mut tiles = Vec::new();
        for py in (0..pixels.height).step_by(PIECE as usize) {
            for px in (0..pixels.width).step_by(PIECE as usize) {
                let (w, h) = (PIECE.min(pixels.width - px), PIECE.min(pixels.height - py));
                // Whole device pixels, so two pieces meet with neither a gap nor a pixel twice.
                let (x0, y0) = ((dest.x + px as f32 * sx).round(), (dest.y + py as f32 * sy).round());
                let (x1, y1) =
                    ((dest.x + (px + w) as f32 * sx).round(), (dest.y + (py + h) as f32 * sy).round());
                let bounds = Rect::new(x0, y0, x1 - x0, y1 - y0);
                if !bounds.intersects(clip.rect) || visible.is_some_and(|v| !bounds.intersects(v)) {
                    continue;
                }
                let cut = || {
                    let row = pixels.width as usize * 4;
                    let rows = pixels.pixels.chunks_exact(row).skip(py as usize).take(h as usize);
                    rows.flat_map(|r| &r[px as usize * 4..(px + w) as usize * 4]).copied().collect()
                };
                let Some(tile) = cx.fonts.image(image.id, px, py, w, h, cut) else { continue };
                let mut uv = tile.uv;
                if !exact {
                    // Half a texel in: a piece that is scaled takes nothing from its neighbours
                    // in the atlas.
                    let (u, v) = ((uv[2] - uv[0]) / w as f32 / 2.0, (uv[3] - uv[1]) / h as f32 / 2.0);
                    uv = [uv[0] + u, uv[1] + v, uv[2] - u, uv[3] - v];
                }
                let (slice, color) = (tile.rect.slice, Color::WHITE);
                let sprite = Sprite { bounds, uv, slice, color, radii: Corners::default(), glyph: false };
                cx.push(sprite.instance(Kind::Poly, &clip));
                tiles.push(tile);
            }
        }
        self.tiles = tiles; // the old ones are let go after the new are held
    }
}

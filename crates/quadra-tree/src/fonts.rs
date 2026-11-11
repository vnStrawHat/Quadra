//! Fonts: the text system and the glyph tiles text is painted with, shared by every window of
//! an app (specs/08, "Glyph tiles").

use std::cell::RefCell;
use std::rc::Rc;

use quadra_render::{Atlas, AtlasKind, TileRef};
use quadra_text::{FaceId, TextBackend, TextSystem};

/// The horizontal positions within a pixel a glyph is rasterized at. Vertical positions are
/// whole pixels.
pub const SUBPIXEL_STEPS: u8 = 4;

/// The slices of the glyph atlases are this many pixels square.
const SLICE: u32 = 1024;
/// What the atlases try to stay within: sixteen slices of coverage, four of colour.
const COVERAGE_BUDGET: usize = 16 << 20;
const COLOR_BUDGET: usize = 16 << 20;

/// A glyph as it is rasterized: at a device size and a subpixel phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub face: FaceId,
    pub id: u16,
    /// The bits of the size in device pixels.
    pub size: u32,
    pub phase: u8,
}

/// Where a glyph's bitmap sits against the pen position on the baseline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlyphMeta {
    pub left: f32,
    pub top: f32,
    /// In the colour atlas: an emoji.
    pub color: bool,
}

/// A handle to a glyph's tile: a text node holds the handles of what it painted, which keeps
/// those tiles in the atlas.
pub type GlyphTile = TileRef<GlyphMeta>;

pub struct Fonts {
    pub text: TextSystem,
    /// Coverage glyphs.
    pub glyphs: Atlas<GlyphKey, GlyphMeta>,
    pub color_glyphs: Atlas<GlyphKey, GlyphMeta>,
}

impl Fonts {
    /// The fonts of an app on a text backend, ready to share between its windows.
    pub fn new(backend: Box<dyn TextBackend>) -> Rc<RefCell<Fonts>> {
        Rc::new(RefCell::new(Fonts {
            text: TextSystem::new(backend),
            glyphs: Atlas::new(AtlasKind::Coverage, SLICE, COVERAGE_BUDGET),
            color_glyphs: Atlas::new(AtlasKind::Color, SLICE, COLOR_BUDGET),
        }))
    }

    /// The tile of a glyph at a size in device pixels and a subpixel phase, rasterized the
    /// first time it is asked for. A glyph with no ink has a tile of no size. `None` when the
    /// backend cannot rasterize it or it is larger than an atlas slice.
    pub fn glyph(&mut self, face: FaceId, id: u16, size: f32, phase: u8) -> Option<GlyphTile> {
        let key = GlyphKey { face, id, size: size.to_bits(), phase };
        if let Some(tile) = self.glyphs.get(&key).or_else(|| self.color_glyphs.get(&key)) {
            return Some(tile);
        }
        let bitmap = self.text.rasterize(face, id, size, phase)?;
        let meta = GlyphMeta { left: bitmap.left as f32, top: bitmap.top as f32, color: bitmap.color };
        let atlas = if bitmap.color { &mut self.color_glyphs } else { &mut self.glyphs };
        atlas.insert(key, bitmap.width, bitmap.height, &bitmap.pixels, meta)
    }

    /// Forgets every tile: the GPU lost them. Text paints again and rasterizes what it needs.
    pub fn clear_tiles(&mut self) {
        self.glyphs.clear();
        self.color_glyphs.clear();
    }
}

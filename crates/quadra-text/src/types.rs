//! Text types and the boundary to the OS (specs/08, "`TextBackend`").

use std::rc::Rc;

/// A face of a backend: a family at a weight, upright or italic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FaceId(pub u32);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FontKey {
    pub family: Rc<str>,
    pub weight: u16,
    pub italic: bool,
}

/// An OpenType feature a family is shaped with: its tag, and 1 for on, 0 for off, or which
/// of its alternates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Feature {
    pub tag: [u8; 4],
    pub value: u32,
}

impl Feature {
    pub const fn on(tag: &[u8; 4]) -> Feature {
        Feature { tag: *tag, value: 1 }
    }

    pub const fn off(tag: &[u8; 4]) -> Feature {
        Feature { tag: *tag, value: 0 }
    }
}

/// Per em: multiply by the font size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FaceMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub line_gap: f32,
    /// Below the baseline, positive down.
    pub underline_position: f32,
    pub underline_thickness: f32,
}

/// One glyph of a shaped word. `x` and `y` are from the word's pen start; `cluster` is the
/// byte offset, in the shaped text, of the first character the glyph covers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShapedGlyph {
    pub face: FaceId,
    pub id: u16,
    pub x: f32,
    pub y: f32,
    pub advance: f32,
    pub cluster: u32,
}

/// What the pixels of a rasterized glyph are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GlyphFormat {
    /// One byte a pixel: how much of it the glyph covers.
    #[default]
    Coverage,
    /// RGBA: the coverage of the red, green and blue stripes of an LCD pixel, in screen
    /// order, and their maximum in alpha. Blended per channel, in the text's colour.
    Subpixel,
    /// Premultiplied RGBA: a glyph with colours of its own (emoji).
    Color,
}

/// A rasterized glyph. `left` and `top` place its top-left corner against the pen position on
/// the baseline.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlyphBitmap {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub format: GlyphFormat,
    pub pixels: Vec<u8>,
}

/// What shapes text: everything but colour and decoration, which only paint.
#[derive(Clone, Debug, PartialEq)]
pub struct TextStyle {
    /// Empty: the backend's default family.
    pub family: Rc<str>,
    pub size: f32,
    pub weight: u16,
    pub italic: bool,
    /// A multiple of the size.
    pub line_height: f32,
}

thread_local! {
    /// The family of a default style: shared, so that making one allocates nothing.
    static NO_FAMILY: Rc<str> = Rc::from("");
}

impl Default for TextStyle {
    fn default() -> TextStyle {
        let family = NO_FAMILY.with(Rc::clone);
        TextStyle { family, size: 14.0, weight: 400, italic: false, line_height: 1.4 }
    }
}

impl TextStyle {
    /// The height of a line box: the multiple of the size, in whole pixels, so that lines of
    /// any size keep to the pixel grid.
    pub fn line_px(&self) -> f32 {
        (self.size * self.line_height).round()
    }
}

/// What a run of rich text says otherwise than the style of its paragraph.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shaping {
    pub family: Option<Rc<str>>,
    pub size: Option<f32>,
    pub weight: Option<u16>,
    pub italic: Option<bool>,
    /// The run is a box so wide, with no glyphs: what is drawn there is its maker's. Its
    /// text is one character (U+FFFC stands for what is no text), and it is in the word
    /// around it.
    pub advance: Option<f32>,
}

impl Shaping {
    /// The style of the run in a paragraph of `base`.
    pub fn over(&self, base: &TextStyle) -> TextStyle {
        TextStyle {
            family: self.family.clone().unwrap_or_else(|| base.family.clone()),
            size: self.size.unwrap_or(base.size),
            weight: self.weight.unwrap_or(base.weight),
            italic: self.italic.unwrap_or(base.italic),
            line_height: base.line_height,
        }
    }
}

/// Fonts and shaping of one operating system (or a fake).
pub trait TextBackend {
    fn default_family(&self) -> String;

    /// The installed and loaded families, sorted.
    fn families(&self) -> Vec<String>;

    /// Registers bundled fonts and returns their family names.
    fn load_fonts(&mut self, files: &[&[u8]]) -> Result<Vec<String>, String>;

    /// `None` when the family is not known.
    fn face(&mut self, key: &FontKey) -> Option<FaceId>;

    fn metrics(&self, face: FaceId) -> FaceMetrics;

    /// Shapes a word. Characters the face lacks come back in fallback faces: each glyph names
    /// its own face.
    fn shape(&mut self, text: &str, face: FaceId, size: f32) -> Vec<ShapedGlyph>;

    /// Rasterizes a glyph at a size in device pixels, its pen `phase` quarters of a pixel
    /// to the right. A glyph with no ink is a bitmap of no size.
    fn rasterize(&mut self, face: FaceId, glyph: u16, size: f32, phase: u8) -> Option<GlyphBitmap>;

    /// The system's settings changed: reads again what rasterizing follows (how the OS
    /// smooths fonts). True when glyphs come out otherwise now, and must be rasterized again.
    fn system_changed(&mut self) -> bool {
        false
    }

    /// The OpenType features a family is shaped with from now on, besides the font's own
    /// defaults. Glyphs of a fallback face are not the family's. A backend that cannot apply
    /// them ignores them.
    fn set_features(&mut self, _family: &str, _features: &[Feature]) {}
}

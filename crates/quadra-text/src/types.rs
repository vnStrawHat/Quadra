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

impl Default for TextStyle {
    fn default() -> TextStyle {
        TextStyle { family: Rc::from(""), size: 14.0, weight: 400, italic: false, line_height: 1.4 }
    }
}

impl TextStyle {
    /// The height of a line box in pixels.
    pub fn line_px(&self) -> f32 {
        self.size * self.line_height
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
}

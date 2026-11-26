//! The Core Text text backend (specs/08; specs/23): font matching, fallback, shaping and
//! rasterization by the OS.

mod bidi;
mod fonts;
mod raster;
mod shape;

use std::collections::HashMap;

use objc2_core_foundation::CFRetained;
use objc2_core_text::{CTFont, CTFontDescriptor, CTFontSymbolicTraits};
use quadra_text::{FaceId, FaceMetrics, Feature, FontKey, GlyphBitmap, ShapedGlyph, TextBackend};

/// The family the system's own font answers to: it has no name a program may ask it by.
pub const SYSTEM_FAMILY: &str = "system-ui";

/// The size faces are kept at, and their metrics read at.
const BASE: f64 = 16.0;

struct Face {
    /// At [`BASE`].
    font: CFRetained<CTFont>,
    /// Its PostScript name: what a font that fallback chose is known by.
    name: String,
    /// The family it was asked by, or its own when fallback chose it: whose features it is
    /// shaped with.
    family: String,
    /// It has glyphs with colours of their own (emoji).
    color: bool,
}

pub struct CoreText {
    faces: Vec<Face>,
    /// The keys faces were asked by, each with its face. A face fallback brought has none.
    keys: Vec<(FontKey, FaceId)>,
    /// A face at a size. A font of Core Text is of one size, and the system's own is another
    /// design at another size (its optical size, its tracking), so nothing is scaled from
    /// one size to the next.
    // ponytail: grows with every size a face is seen at; emptied when that is ever much.
    sized: HashMap<(u32, u32), CFRetained<CTFont>>,
    /// Fonts loaded from memory: searched before the installed ones.
    loaded: Vec<fonts::Loaded>,
    /// The features each family is shaped with, as the app set them.
    features: Vec<(String, Vec<Feature>)>,
    /// The families tried for what a face lacks, before the system's fallback.
    cascade: Vec<CFRetained<CTFontDescriptor>>,
}

impl CoreText {
    /// Nothing here can fail; a `Result` as the other systems' backends give one.
    pub fn new() -> Result<CoreText, String> {
        Ok(CoreText {
            faces: Vec::new(),
            keys: Vec::new(),
            sized: HashMap::new(),
            loaded: Vec::new(),
            features: Vec::new(),
            cascade: Vec::new(),
        })
    }

    /// Keeps a font, at [`BASE`], as a face of a family: the one it was asked by, or its own.
    fn keep(&mut self, font: CFRetained<CTFont>, family: Option<&str>) -> FaceId {
        // SAFETY: the calls read a valid font.
        let (name, traits) = unsafe { (font.post_script_name().to_string(), font.symbolic_traits()) };
        let family = family.map_or_else(|| unsafe { font.family_name() }.to_string(), str::to_string);
        let color = traits.contains(CTFontSymbolicTraits::TraitColorGlyphs);
        self.faces.push(Face { font, name, family, color });
        FaceId(self.faces.len() as u32 - 1)
    }

    /// The face of a font that fallback chose, of the PostScript name `name`.
    fn fallback(&mut self, font: &CTFont, name: &str) -> FaceId {
        if let Some(known) = self.faces.iter().position(|f| f.name == name) {
            return FaceId(known as u32);
        }
        // SAFETY: a copy of a valid font at another size, with no matrix and no attributes.
        let base = unsafe { font.copy_with_attributes(BASE, std::ptr::null(), None) };
        self.keep(base, None)
    }

    /// A face's font at a size, with its family's features and the families tried for what
    /// it lacks.
    fn at(&mut self, face: FaceId, size: f32) -> Option<CFRetained<CTFont>> {
        if let Some(known) = self.sized.get(&(face.0, size.to_bits())) {
            return Some(known.clone());
        }
        let base = self.faces.get(face.0 as usize)?;
        let features = self.features.iter().find(|(family, _)| family.eq_ignore_ascii_case(&base.family));
        let features = features.map_or(&[][..], |(_, list)| list.as_slice());
        let more = fonts::more(features, &self.cascade);
        // SAFETY: a copy of a valid font at another size, with no matrix and the attributes of
        // a valid descriptor or none.
        let sized = unsafe { base.font.copy_with_attributes(size as f64, std::ptr::null(), more.as_deref()) };
        self.sized.insert((face.0, size.to_bits()), sized.clone());
        Some(sized)
    }
}

impl TextBackend for CoreText {
    fn default_family(&self) -> String {
        SYSTEM_FAMILY.to_string()
    }

    fn families(&self) -> Vec<String> {
        let mut all = fonts::installed();
        all.extend(self.loaded.iter().map(|font| font.family.clone()));
        all.push(SYSTEM_FAMILY.to_string());
        all.sort();
        all.dedup();
        all
    }

    fn load_fonts(&mut self, files: &[&[u8]]) -> Result<Vec<String>, String> {
        let mut names = Vec::new();
        for file in files {
            for font in fonts::load(file)? {
                if !names.contains(&font.family) {
                    names.push(font.family.clone());
                }
                self.loaded.push(font);
            }
        }
        Ok(names)
    }

    fn face(&mut self, key: &FontKey) -> Option<FaceId> {
        if let Some((_, face)) = self.keys.iter().find(|(known, _)| known == key) {
            return Some(*face);
        }
        let font = fonts::matched(&self.loaded, key)?;
        let face = self.keep(font, Some(&key.family));
        self.keys.push((key.clone(), face));
        Some(face)
    }

    fn metrics(&self, face: FaceId) -> FaceMetrics {
        let Some(face) = self.faces.get(face.0 as usize) else { return FaceMetrics::default() };
        let (font, per_em) = (&face.font, |points: f64| (points / BASE) as f32);
        // SAFETY: each call reads a number of a valid font. Core Text counts the descent
        // down and the underline's place up from the baseline.
        unsafe {
            FaceMetrics {
                ascent: per_em(font.ascent()),
                descent: per_em(font.descent()),
                line_gap: per_em(font.leading()),
                underline_position: per_em(-font.underline_position()),
                underline_thickness: per_em(font.underline_thickness()),
            }
        }
    }

    fn shape(&mut self, text: &str, face: FaceId, size: f32) -> Vec<ShapedGlyph> {
        self.shape_word(text, face, size)
    }

    fn rasterize(&mut self, face: FaceId, glyph: u16, size: f32, phase: u8) -> Option<GlyphBitmap> {
        self.raster(face, glyph, size, phase)
    }

    fn bidi_levels(&mut self, text: &str, rtl: bool) -> Option<Vec<u8>> {
        bidi::levels(text, rtl)
    }

    fn set_features(&mut self, family: &str, features: &[Feature]) {
        self.features.retain(|(name, _)| !name.eq_ignore_ascii_case(family));
        if !features.is_empty() {
            self.features.push((family.to_string(), features.to_vec()));
        }
        self.sized.clear();
    }

    fn set_fallbacks(&mut self, families: &[String]) {
        let key = |family: &String| FontKey { family: family.as_str().into(), weight: 400, italic: false };
        let found = families.iter().filter_map(|family| fonts::matched(&self.loaded, &key(family)));
        // SAFETY: the descriptor of a valid font.
        self.cascade = found.map(|font| unsafe { font.font_descriptor() }).collect();
        self.sized.clear();
    }
}

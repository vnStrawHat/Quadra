//! The DirectWrite text backend (specs/08; behaviour/04): font matching, script analysis,
//! fallback, shaping and rasterization by the OS.

mod analysis;
mod fonts;
mod raster;
mod shape;

use std::ffi::c_void;

use quadra_text::{FaceId, FaceMetrics, Feature, FontKey, GlyphBitmap, ShapedGlyph, TextBackend};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_FEATURE, DWRITE_FONT_FEATURE_TAG, DWRITE_FONT_METRICS,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE, DWRITE_FONT_STYLE_ITALIC, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT, DWRITE_SCRIPT_ANALYSIS, DWriteCreateFactory, IDWriteFactory2, IDWriteFactory5,
    IDWriteFontCollection, IDWriteFontFace, IDWriteFontFace2, IDWriteFontFallback, IDWriteFontFamily,
    IDWriteTextAnalyzer,
};
use windows::Win32::UI::WindowsAndMessaging::{
    SPI_GETFONTSMOOTHING, SPI_GETFONTSMOOTHINGORIENTATION, SPI_GETFONTSMOOTHINGTYPE,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SystemParametersInfoW,
};
use windows::core::{BOOL, HSTRING, Interface, w};

struct Face {
    face: IDWriteFontFace,
    metrics: DWRITE_FONT_METRICS,
    /// It has glyphs with colour layers (emoji).
    color: bool,
}

pub struct DirectWrite {
    factory: IDWriteFactory5,
    /// The same factory, as the version whose rasterization calls are used.
    factory2: IDWriteFactory2,
    analyzer: IDWriteTextAnalyzer,
    system: IDWriteFontCollection,
    fallback: IDWriteFontFallback,
    /// Fonts loaded from memory: searched before the installed ones.
    loaded: Option<fonts::Loaded>,
    faces: Vec<Face>,
    keys: Vec<FontKey>,
    /// "en-us", as the OS takes it.
    locale: HSTRING,
    /// The script analyses of printable ASCII: without a letter, and with one.
    ascii: Option<[DWRITE_SCRIPT_ANALYSIS; 2]>,
    scratch: shape::Scratch,
    /// Glyphs are rasterized per subpixel, as the Windows setting says; `bgr` when the
    /// display's stripes run blue, green, red.
    pub cleartype: bool,
    bgr: bool,
    /// The features each family is shaped with, as the app set them.
    features: Vec<(String, Vec<DWRITE_FONT_FEATURE>)>,
}

fn failed(e: windows::core::Error) -> String {
    e.message().to_string()
}

/// Whether Windows smooths fonts per subpixel, and whether the stripes run blue to red.
fn font_smoothing() -> (bool, bool) {
    let (mut on, mut kind, mut orientation) = (0u32, 0u32, 1u32);
    let none = SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0);
    let out = |v: &mut u32| Some(v as *mut u32 as *mut c_void);
    // SAFETY: each query writes one 32-bit value through the pointer it is given.
    unsafe {
        let _ = SystemParametersInfoW(SPI_GETFONTSMOOTHING, 0, out(&mut on), none);
        let _ = SystemParametersInfoW(SPI_GETFONTSMOOTHINGTYPE, 0, out(&mut kind), none);
        let _ = SystemParametersInfoW(SPI_GETFONTSMOOTHINGORIENTATION, 0, out(&mut orientation), none);
    }
    (on != 0 && kind == 2, orientation == 0)
}

/// The name of a family: the English one when it has one.
fn family_name(family: &IDWriteFontFamily) -> Option<String> {
    // SAFETY: the out pointers are valid, and the buffer is as long as the call is told.
    unsafe {
        let names = family.GetFamilyNames().ok()?;
        let (mut index, mut exists) = (0, BOOL(0));
        let _ = names.FindLocaleName(w!("en-us"), &mut index, &mut exists);
        if !exists.as_bool() {
            index = 0;
        }
        let mut buffer = vec![0u16; names.GetStringLength(index).ok()? as usize + 1];
        names.GetString(index, &mut buffer).ok()?;
        buffer.pop(); // the terminator
        Some(String::from_utf16_lossy(&buffer))
    }
}

fn family_names(collection: &IDWriteFontCollection) -> Vec<String> {
    // SAFETY: indices below the count exist.
    let count = unsafe { collection.GetFontFamilyCount() };
    let family = |i| unsafe { collection.GetFontFamily(i) }.ok().and_then(|f| family_name(&f));
    (0..count).filter_map(family).collect()
}

fn style_of(italic: bool) -> DWRITE_FONT_STYLE {
    if italic { DWRITE_FONT_STYLE_ITALIC } else { DWRITE_FONT_STYLE_NORMAL }
}

/// The face of a collection closest to a key; `None` when it has no such family.
fn match_in(collection: &IDWriteFontCollection, key: &FontKey) -> Option<Face> {
    // SAFETY: the out pointers are valid for the calls, and the index is one the collection
    // just gave.
    unsafe {
        let (mut index, mut exists) = (0, BOOL(0));
        collection.FindFamilyName(&HSTRING::from(&*key.family), &mut index, &mut exists).ok()?;
        if !exists.as_bool() {
            return None;
        }
        let family = collection.GetFontFamily(index).ok()?;
        let weight = DWRITE_FONT_WEIGHT(key.weight as i32);
        let font =
            family.GetFirstMatchingFont(weight, DWRITE_FONT_STRETCH_NORMAL, style_of(key.italic)).ok()?;
        let face = font.CreateFontFace().ok()?;
        let mut metrics = DWRITE_FONT_METRICS::default();
        face.GetMetrics(&mut metrics);
        let color = face.cast::<IDWriteFontFace2>().is_ok_and(|f| f.IsColorFont().as_bool());
        Some(Face { face, metrics, color })
    }
}

impl DirectWrite {
    pub fn new() -> Result<DirectWrite, String> {
        // SAFETY: plain factory calls; the out pointer is valid.
        unsafe {
            let factory: IDWriteFactory5 = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).map_err(failed)?;
            let factory2: IDWriteFactory2 = factory.cast().map_err(failed)?;
            let mut system = None;
            factory2.GetSystemFontCollection(&mut system, false).map_err(failed)?;
            let (cleartype, bgr) = font_smoothing();
            Ok(DirectWrite {
                analyzer: factory.CreateTextAnalyzer().map_err(failed)?,
                system: system.ok_or("DirectWrite gave no font collection")?,
                fallback: factory2.GetSystemFontFallback().map_err(failed)?,
                factory,
                factory2,
                loaded: None,
                faces: Vec::new(),
                keys: Vec::new(),
                locale: HSTRING::from("en-us"),
                ascii: None,
                scratch: shape::Scratch::default(),
                cleartype,
                bgr,
                features: Vec::new(),
            })
        }
    }
}

impl TextBackend for DirectWrite {
    fn default_family(&self) -> String {
        "Segoe UI".to_string()
    }

    fn families(&self) -> Vec<String> {
        let mut all = family_names(&self.system);
        if let Some(loaded) = &self.loaded {
            all.extend(family_names(&loaded.collection));
        }
        all.sort();
        all.dedup();
        all
    }

    fn load_fonts(&mut self, files: &[&[u8]]) -> Result<Vec<String>, String> {
        self.load(files).map_err(failed)
    }

    fn face(&mut self, key: &FontKey) -> Option<FaceId> {
        if let Some(known) = self.keys.iter().position(|k| k == key) {
            return Some(FaceId(known as u32));
        }
        let loaded = self.loaded.as_ref().and_then(|l| match_in(&l.collection, key));
        let face = loaded.or_else(|| match_in(&self.system, key))?;
        self.faces.push(face);
        self.keys.push(key.clone());
        Some(FaceId(self.faces.len() as u32 - 1))
    }

    fn metrics(&self, face: FaceId) -> FaceMetrics {
        let Some(face) = self.faces.get(face.0 as usize) else { return FaceMetrics::default() };
        let (m, em) = (&face.metrics, face.metrics.designUnitsPerEm.max(1) as f32);
        FaceMetrics {
            ascent: m.ascent as f32 / em,
            descent: m.descent as f32 / em,
            line_gap: m.lineGap as f32 / em,
            underline_position: -(m.underlinePosition as f32) / em,
            underline_thickness: m.underlineThickness as f32 / em,
        }
    }

    fn shape(&mut self, text: &str, face: FaceId, size: f32) -> Vec<ShapedGlyph> {
        self.shape_word(text, face, size)
    }

    fn rasterize(&mut self, face: FaceId, glyph: u16, size: f32, phase: u8) -> Option<GlyphBitmap> {
        self.raster(face, glyph, size, phase)
    }

    fn bidi_levels(&mut self, text: &str, rtl: bool) -> Option<Vec<u8>> {
        let units: Vec<u16> = text.encode_utf16().collect();
        let levels = self.levels(&units, rtl)?;
        let (mut out, mut unit) = (Vec::with_capacity(text.len()), 0);
        for c in text.chars() {
            out.extend(std::iter::repeat_n(levels[unit], c.len_utf8()));
            unit += c.len_utf16();
        }
        Some(out)
    }

    fn set_features(&mut self, family: &str, features: &[Feature]) {
        let tag = |t: [u8; 4]| DWRITE_FONT_FEATURE_TAG(u32::from_le_bytes(t));
        let list = features.iter().map(|f| DWRITE_FONT_FEATURE { nameTag: tag(f.tag), parameter: f.value });
        self.features.retain(|(name, _)| !name.eq_ignore_ascii_case(family));
        if !features.is_empty() {
            self.features.push((family.to_string(), list.collect()));
        }
    }

    fn system_changed(&mut self) -> bool {
        let now = font_smoothing();
        let changed = now != (self.cleartype, self.bgr);
        (self.cleartype, self.bgr) = now;
        changed
    }
}

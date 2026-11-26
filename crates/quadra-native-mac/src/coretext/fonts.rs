//! The fonts faces are made of: the installed families, the system's own font, and fonts
//! loaded from memory.

use objc2_core_foundation::{CFData, CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_core_text::{
    CTFont, CTFontDescriptor, CTFontManagerCopyAvailableFontFamilyNames,
    CTFontManagerCreateFontDescriptorsFromData, CTFontSymbolicTraits, CTFontUIFontType,
    kCTFontFamilyNameAttribute, kCTFontTraitsAttribute, kCTFontWeightTrait,
};
use quadra_text::FontKey;

use super::{BASE, SYSTEM_FAMILY};

/// A font loaded from memory.
pub(super) struct Loaded {
    pub family: String,
    /// As Core Text counts a weight: -1 to 1, the regular at 0.
    weight: f64,
    italic: bool,
    /// At [`BASE`].
    font: CFRetained<CTFont>,
}

/// A weight of 100 to 900 as Core Text counts it.
fn weight_of(weight: u16) -> f64 {
    const STEPS: [f64; 9] = [-0.8, -0.6, -0.4, 0.0, 0.23, 0.3, 0.4, 0.56, 0.62];
    let at = (f64::from(weight.clamp(100, 900)) - 100.0) / 100.0;
    let low = at.floor() as usize;
    let high = (low + 1).min(STEPS.len() - 1);
    STEPS[low] + (STEPS[high] - STEPS[low]) * at.fract()
}

fn is_italic(font: &CTFont) -> bool {
    // SAFETY: reads a valid font.
    unsafe { font.symbolic_traits() }.contains(CTFontSymbolicTraits::TraitItalic)
}

/// The weight of a font, as Core Text counts it.
fn weight_in(font: &CTFont) -> f64 {
    // SAFETY: the traits of a font are a dictionary of Core Text's names to values, of which
    // the weight is a number; what is not one is passed over.
    unsafe {
        let traits = font.traits();
        let weight = traits.cast_unchecked::<CFString, CFType>().get(kCTFontWeightTrait);
        weight.and_then(|it| it.downcast_ref::<CFNumber>().and_then(CFNumber::as_f64)).unwrap_or(0.0)
    }
}

/// The attributes of a descriptor that ask for a weight, and for a family when one is named.
fn asking(family: Option<&str>, weight: u16) -> CFRetained<CFDictionary<CFString, CFType>> {
    let weight = CFNumber::new_f64(weight_of(weight));
    let weight: &CFType = &weight;
    // SAFETY: the names are Core Text's constants, alive for the program's life.
    let (family_name, traits_name, weight_name) =
        unsafe { (kCTFontFamilyNameAttribute, kCTFontTraitsAttribute, kCTFontWeightTrait) };
    let traits = CFDictionary::<CFString, CFType>::from_slices(&[weight_name], &[weight]);
    let traits: &CFType = &traits;
    match family {
        Some(family) => {
            let family = CFString::from_str(family);
            let family: &CFType = &family;
            CFDictionary::from_slices(&[family_name, traits_name], &[family, traits])
        }
        None => CFDictionary::from_slices(&[traits_name], &[traits]),
    }
}

/// The installed font of a key's family nearest its weight; none when no family is so named.
fn installed_font(key: &FontKey) -> Option<CFRetained<CTFont>> {
    let attributes = asking(Some(&key.family), key.weight);
    // SAFETY: a descriptor of valid attributes, and the font Core Text matches to it.
    unsafe {
        let descriptor = CTFontDescriptor::with_attributes(attributes.as_opaque());
        let font = CTFont::with_font_descriptor(&descriptor, BASE, std::ptr::null());
        // A family that is not installed is answered with a font of another: no match.
        font.family_name().to_string().eq_ignore_ascii_case(&key.family).then_some(font)
    }
}

/// The system's own font at a weight.
fn system(key: &FontKey) -> Option<CFRetained<CTFont>> {
    // SAFETY: the font the system names its interface in, and a copy of its descriptor that
    // asks for another weight of it.
    unsafe {
        let font = CTFont::new_ui_font_for_language(CTFontUIFontType::System, BASE, None)?;
        if key.weight == 400 {
            return Some(font);
        }
        let attributes = asking(None, key.weight);
        let descriptor = font.font_descriptor().copy_with_attributes(attributes.as_opaque());
        Some(CTFont::with_font_descriptor(&descriptor, BASE, std::ptr::null()))
    }
}

/// The italic of a font when one is asked for and its family has one.
fn slanted(font: CFRetained<CTFont>, italic: bool) -> CFRetained<CTFont> {
    if !italic {
        return font;
    }
    let italic = CTFontSymbolicTraits::TraitItalic;
    // SAFETY: a copy of a valid font with a trait asked for; none when the family has none.
    unsafe { font.copy_with_symbolic_traits(BASE, std::ptr::null(), italic, italic) }.unwrap_or(font)
}

/// The loaded font of a key's family nearest the key: its slant first, then its weight.
fn loaded_font(loaded: &[Loaded], key: &FontKey) -> Option<CFRetained<CTFont>> {
    let weight = weight_of(key.weight);
    let far = |it: &Loaded| (it.weight - weight).abs() + if it.italic == key.italic { 0.0 } else { 10.0 };
    let family = loaded.iter().filter(|it| it.family.eq_ignore_ascii_case(&key.family));
    family.min_by(|a, b| far(a).total_cmp(&far(b))).map(|it| it.font.clone())
}

/// The font of a key: the system's own, a loaded one, or an installed one.
pub(super) fn matched(loaded: &[Loaded], key: &FontKey) -> Option<CFRetained<CTFont>> {
    if &*key.family == SYSTEM_FAMILY {
        return system(key).map(|font| slanted(font, key.italic));
    }
    loaded_font(loaded, key).or_else(|| installed_font(key).map(|font| slanted(font, key.italic)))
}

/// The fonts of a file in memory.
pub(super) fn load(file: &[u8]) -> Result<Vec<Loaded>, String> {
    if file.is_empty() {
        return Err("an empty font file".to_string());
    }
    let data = CFData::from_bytes(file);
    // SAFETY: Core Text describes the fonts of valid data, which the descriptors keep alive;
    // the array holds descriptors and nothing else.
    let fonts: Vec<Loaded> = unsafe {
        let descriptors = CTFontManagerCreateFontDescriptorsFromData(&data);
        let fonts = descriptors.cast_unchecked::<CTFontDescriptor>().iter().map(|descriptor| {
            let font = CTFont::with_font_descriptor(&descriptor, BASE, std::ptr::null());
            let family = font.family_name().to_string();
            Loaded { family, weight: weight_in(&font), italic: is_italic(&font), font }
        });
        fonts.collect()
    };
    if fonts.is_empty() {
        return Err("not a font file".to_string());
    }
    Ok(fonts)
}

/// The installed families.
pub(super) fn installed() -> Vec<String> {
    // SAFETY: the array holds the names, strings.
    unsafe {
        let names = CTFontManagerCopyAvailableFontFamilyNames();
        names.cast_unchecked::<CFString>().iter().map(|name| name.to_string()).collect()
    }
}

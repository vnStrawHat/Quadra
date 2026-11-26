//! Bidirectional levels (specs/08): the Unicode algorithm as the system's ICU runs it. Core
//! Text orders a line by it and tells a run's direction, not its level, which is what a
//! paragraph is ordered by here.

use std::ffi::c_void;

// The system's ICU, a library of every Mac. Its names carry no version there.
#[link(name = "icucore")]
unsafe extern "C" {
    fn ubidi_open() -> *mut c_void;
    fn ubidi_close(bidi: *mut c_void);
    /// `levels` null: the algorithm finds them. `error` is zero or below for no error.
    fn ubidi_setPara(
        bidi: *mut c_void,
        text: *const u16,
        length: i32,
        paragraph_level: u8,
        levels: *mut u8,
        error: *mut i32,
    );
    /// One level a UTF-16 unit of the text, owned by `bidi`.
    fn ubidi_getLevels(bidi: *mut c_void, error: *mut i32) -> *const u8;
}

/// The level of each byte of `text`, read right to left when `rtl`.
pub(super) fn levels(text: &str, rtl: bool) -> Option<Vec<u8>> {
    if text.is_empty() {
        return Some(Vec::new());
    }
    let units: Vec<u16> = text.encode_utf16().collect();
    let length = i32::try_from(units.len()).ok()?;
    // SAFETY: the text is `length` units long and outlives the calls; the levels ICU gives
    // back are as many, and are copied before the object that owns them is closed.
    let by_unit = unsafe {
        let bidi = ubidi_open();
        if bidi.is_null() {
            return None;
        }
        let mut error = 0;
        ubidi_setPara(bidi, units.as_ptr(), length, u8::from(rtl), std::ptr::null_mut(), &mut error);
        let found = if error <= 0 { ubidi_getLevels(bidi, &mut error) } else { std::ptr::null() };
        let copied =
            (error <= 0 && !found.is_null()).then(|| std::slice::from_raw_parts(found, units.len()).to_vec());
        ubidi_close(bidi);
        copied?
    };
    let (mut out, mut unit) = (Vec::with_capacity(text.len()), 0);
    for c in text.chars() {
        out.extend(std::iter::repeat_n(by_unit[unit], c.len_utf8()));
        unit += c.len_utf16();
    }
    Some(out)
}

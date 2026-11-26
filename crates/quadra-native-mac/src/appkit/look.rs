//! What the system looks like (specs/10): dark or light, more contrast, the accent colour;
//! and the pointer's shapes.

use std::ffi::CStr;

use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::{ClassType, msg_send};
use objc2_app_kit::{
    NSAppearance, NSAppearanceNameAccessibilityHighContrastAqua,
    NSAppearanceNameAccessibilityHighContrastDarkAqua, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSColor, NSColorSpace, NSCursor,
};
use objc2_foundation::NSArray;
use quadra_platform::{Appearance, Cursor};
use quadra_prim::Color;

/// What an appearance of AppKit asks of an app. The system's own colours and a size of
/// text are not said: macOS has no contrast themes and no text size for all apps.
pub(crate) fn appearance(of: &NSAppearance) -> Appearance {
    // SAFETY: AppKit's constants, read and never written.
    let names = unsafe {
        NSArray::from_slice(&[
            NSAppearanceNameAqua,
            NSAppearanceNameDarkAqua,
            NSAppearanceNameAccessibilityHighContrastAqua,
            NSAppearanceNameAccessibilityHighContrastDarkAqua,
        ])
    };
    let name = of.bestMatchFromAppearancesWithNames(&names).map(|it| it.to_string()).unwrap_or_default();
    Appearance {
        dark: name.contains("Dark"),
        high_contrast: name.contains("Accessibility"),
        accent: accent(),
        ..Appearance::default()
    }
}

fn accent() -> Option<Color> {
    let color = NSColor::controlAccentColor().colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
    let (r, g, b) = (color.redComponent(), color.greenComponent(), color.blueComponent());
    Some(Color { r: r as f32, g: g as f32, b: b as f32, a: 1.0 })
}

/// AppKit's cursor of a shape.
#[allow(deprecated)] // the two resize cursors: what replaces them needs macOS 15
pub(crate) fn cursor(shape: Cursor) -> Retained<NSCursor> {
    match shape {
        Cursor::Arrow => NSCursor::arrowCursor(),
        Cursor::Text => NSCursor::IBeamCursor(),
        Cursor::Pointer => NSCursor::pointingHandCursor(),
        Cursor::Crosshair => NSCursor::crosshairCursor(),
        Cursor::Move => unnamed(c"_moveCursor"),
        Cursor::NotAllowed => NSCursor::operationNotAllowedCursor(),
        Cursor::ResizeHorizontal => NSCursor::resizeLeftRightCursor(),
        Cursor::ResizeVertical => NSCursor::resizeUpDownCursor(),
        Cursor::ResizeNwse => unnamed(c"_windowResizeNorthWestSouthEastCursor"),
        Cursor::ResizeNesw => unnamed(c"_windowResizeNorthEastSouthWestCursor"),
        Cursor::Wait => unnamed(c"_waitCursor"),
        Cursor::Progress => unnamed(c"busyButClickableCursor"),
        Cursor::Help => unnamed(c"_helpCursor"),
    }
}

/// A cursor AppKit has and does not name in its headers (the system's own apps show it), or
/// the arrow on a system that has it no more.
fn unnamed(name: &CStr) -> Retained<NSCursor> {
    let (class, name) = (NSCursor::class(), Sel::register(name));
    if class.class_method(name).is_none() {
        return NSCursor::arrowCursor();
    }
    // SAFETY: the class answers the selector, which takes nothing and gives a cursor.
    let cursor: Option<Retained<NSCursor>> = unsafe { msg_send![class, performSelector: name] };
    cursor.unwrap_or_else(NSCursor::arrowCursor)
}

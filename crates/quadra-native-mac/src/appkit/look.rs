//! What the system looks like (specs/10): dark or light, more contrast, the accent colour;
//! and the pointer's shapes.

use std::ffi::CStr;

use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::{ClassType, msg_send, sel};
use objc2_app_kit::{
    NSAppearance, NSAppearanceNameAccessibilityHighContrastAqua,
    NSAppearanceNameAccessibilityHighContrastDarkAqua, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
    NSColor, NSColorSpace, NSCursor, NSCursorFrameResizeDirections, NSCursorFrameResizePosition,
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
    let arrow = NSCursor::arrowCursor;
    match shape {
        Cursor::Arrow => arrow(),
        Cursor::Text => NSCursor::IBeamCursor(),
        Cursor::Pointer => NSCursor::pointingHandCursor(),
        Cursor::Crosshair => NSCursor::crosshairCursor(),
        Cursor::Move => unnamed(c"_moveCursor").unwrap_or_else(NSCursor::openHandCursor),
        Cursor::NotAllowed => NSCursor::operationNotAllowedCursor(),
        Cursor::ResizeHorizontal => NSCursor::resizeLeftRightCursor(),
        Cursor::ResizeVertical => NSCursor::resizeUpDownCursor(),
        Cursor::ResizeNwse => {
            corner(NSCursorFrameResizePosition::TopLeft, c"_windowResizeNorthWestSouthEastCursor")
        }
        Cursor::ResizeNesw => {
            corner(NSCursorFrameResizePosition::TopRight, c"_windowResizeNorthEastSouthWestCursor")
        }
        Cursor::Wait => unnamed(c"_waitCursor").unwrap_or_else(arrow),
        Cursor::Progress => unnamed(c"busyButClickableCursor").unwrap_or_else(arrow),
        Cursor::Help => unnamed(c"_helpCursor").unwrap_or_else(arrow),
    }
}

/// Whether `NSCursor` itself answers a selector, as the runtime's `respondsToSelector`
/// says it: `class_getClassMethod` found a method for a cursor the class then did not
/// answer (`_moveCursor`, on macOS 26).
fn answers(name: Sel) -> bool {
    NSCursor::class().metaclass().responds_to(name)
}

/// The cursor of a frame's corner that is dragged: AppKit's own since macOS 15, before it
/// the one it does not name.
fn corner(at: NSCursorFrameResizePosition, before: &CStr) -> Retained<NSCursor> {
    if answers(sel!(frameResizeCursorFromPosition:inDirections:)) {
        return NSCursor::frameResizeCursorFromPosition_inDirections(at, NSCursorFrameResizeDirections::All);
    }
    unnamed(before).unwrap_or_else(NSCursor::arrowCursor)
}

/// A cursor AppKit has and does not name in its headers (the system's own apps show it);
/// none on a system that has it no more.
fn unnamed(name: &CStr) -> Option<Retained<NSCursor>> {
    let name = Sel::register(name);
    if !answers(name) {
        return None;
    }
    // SAFETY: the class answers the selector, which takes nothing and gives a cursor.
    unsafe { msg_send![NSCursor::class(), performSelector: name] }
}

//! What the system looks like (specs/10): dark or light, more contrast, the accent colour;
//! and the pointer's shapes.

use objc2::rc::Retained;
use objc2::{ClassType, sel};
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

/// AppKit's cursor of a shape, of those it names. It names none for waiting, progress and
/// help (the system shows its own wheel over an app that does not answer): the arrow. The
/// ones it does not name are not asked for: macOS 26 says it answers `_moveCursor`, and
/// raises an exception when asked for it.
#[allow(deprecated)] // the two resize cursors: what replaces them needs macOS 15
pub(crate) fn cursor(shape: Cursor) -> Retained<NSCursor> {
    match shape {
        Cursor::Arrow | Cursor::Wait | Cursor::Progress | Cursor::Help => NSCursor::arrowCursor(),
        Cursor::Text => NSCursor::IBeamCursor(),
        Cursor::Pointer => NSCursor::pointingHandCursor(),
        Cursor::Crosshair => NSCursor::crosshairCursor(),
        Cursor::Move => NSCursor::openHandCursor(),
        Cursor::NotAllowed => NSCursor::operationNotAllowedCursor(),
        Cursor::ResizeHorizontal => NSCursor::resizeLeftRightCursor(),
        Cursor::ResizeVertical => NSCursor::resizeUpDownCursor(),
        Cursor::ResizeNwse => corner(NSCursorFrameResizePosition::TopLeft),
        Cursor::ResizeNesw => corner(NSCursorFrameResizePosition::TopRight),
    }
}

/// The cursor of a frame's corner that is dragged.
// ponytail: the arrow before macOS 15, which names no cursor for a corner.
fn corner(at: NSCursorFrameResizePosition) -> Retained<NSCursor> {
    if !NSCursor::class().metaclass().responds_to(sel!(frameResizeCursorFromPosition:inDirections:)) {
        return NSCursor::arrowCursor();
    }
    NSCursor::frameResizeCursorFromPosition_inDirections(at, NSCursorFrameResizeDirections::All)
}

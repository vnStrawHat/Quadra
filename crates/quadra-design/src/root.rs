//! What an app does to use the kit: its fonts and a theme for the app, and the box at the
//! root of each window that gives the window the look of the design language.

use quadra_app::{App, AppError, Feature};
use quadra_render::Renderer;
use quadra_view::{Div, column};

use crate::theme::{KitTheme, system_theme};
use crate::{fonts, size, token};

/// Gives an app the kit's fonts, its key bindings and a theme. A view changes the theme later by writing the
/// theme signal of its context with another [`KitTheme::theme`].
pub fn install<R: Renderer>(mut app: App<R>, theme: &KitTheme) -> Result<App<R>, AppError> {
    app.load_fonts(&fonts::files())?;
    // The face of the UI is shaped without its contextual alternates (behaviour/15, "Type").
    app.font_features(fonts::UI, &[Feature::off(b"calt")]);
    // What shows before a window's first frame, and at its edge while it is resized.
    let app = app.clear_color(theme.color(token::CANVAS)).theme(theme.theme());
    Ok(app.keymap(crate::keymap()))
}

/// As [`install`], with the theme the system asks for (`system_theme`: its high contrast
/// when that is on, otherwise the dark theme or the light one), and another when the user
/// changes it.
pub fn install_system<R: Renderer>(app: App<R>) -> Result<App<R>, AppError> {
    let theme = system_theme(&app.appearance());
    let app = install(app, &theme)?;
    Ok(app.theme_by(|appearance| system_theme(appearance).theme()))
}

/// The box at the root of a window: the canvas behind everything, and the text of the UI
/// (its face, its size, its colour) for all that the window holds, in a column that fills
/// the window.
pub fn root() -> Div {
    let page = column().grow(1.0).bg(token::CANVAS).text_color(token::TEXT);
    let page = page.font_family(fonts::UI).font_weight(size::WEIGHT_NORMAL);
    // Lines are 1.618 of the size of their text, in whole pixels: 23 px for the 14 of the UI.
    let page = page.text_size(size::TEXT_MD.size).line_height(size::LINE);
    // In fields: what stands where text goes, the selection, and the caret.
    let page = page.placeholder_color(token::TEXT_3).selection_color(token::SELECTION);
    page.caret_color(token::ACCENT)
}

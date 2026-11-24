//! Themes of what the system is set to (specs/12; specs/design, "Themes"): its high
//! contrast, where the user chose a few colours that read well together and every role
//! takes one of them (written as a theme file over the built-in theme of its lightness);
//! and its accent, for an app that takes it.

use quadra_platform::{Appearance, SystemColors};
use quadra_prim::Color;

use super::{KitTheme, Mode, dark, light};
use crate::token;

/// The theme the system asks for: its high contrast when that is on, otherwise the dark
/// theme or the light one. The system's accent is not taken (the design language has its
/// own); `KitTheme::with_accent` takes it for an app that wants it.
pub fn system_theme(appearance: &Appearance) -> KitTheme {
    match appearance.colors.filter(|_| appearance.high_contrast) {
        Some(colors) => high_contrast(&colors),
        None if appearance.dark => dark(),
        None => light(),
    }
}

impl KitTheme {
    /// The theme with another accent (the system's, `Appearance::accent`): the roles that
    /// follow the accent take it, its hover a little lighter on a dark theme (darker on a
    /// light one), its press the other way, its tints and the selection translucent.
    pub fn with_accent(mut self, accent: Color) -> KitTheme {
        use token::*;
        let (lift, sink) = match self.mode {
            Mode::Light => (Color::BLACK, Color::WHITE),
            _ => (Color::WHITE, Color::BLACK),
        };
        let roles = [
            (ACCENT, accent),
            (ACCENT_HOVER, accent.lerp(lift, 0.15)),
            (ACCENT_PRESS, accent.lerp(sink, 0.15)),
            (ACCENT_TINT, accent.alpha(0.16)),
            (ACCENT_TINT_HOVER, accent.alpha(0.2)),
            (ACCENT_TEXT, accent),
            (BORDER_FOCUSED, accent),
            (SELECTION, accent.alpha(0.3)),
            (EDITOR_CURSOR, accent),
            (EDITOR_SELECTION, accent.alpha(0.3)),
        ];
        for (role, color) in roles {
            self.set(role, color);
        }
        self
    }
}

/// The theme of a high contrast the system is set to: the window's colours for planes and
/// text, the highlight for what is chosen, has the focus or is the accent, the link's for
/// links, the disabled text's for what cannot be used and for the quiet text. Lines are
/// the text's colour; tints are the highlight, translucent. The colours of the terminal and
/// of the series of charts stay the built-in theme's.
pub fn high_contrast(colors: &SystemColors) -> KitTheme {
    let SystemColors { window, text, highlight, highlight_text, link, disabled, button, button_text } =
        *colors;
    let hex = |color: Color| color.hex();
    let tint = |color: Color, alpha: f32| color.alpha(alpha).hex();
    let palette: Vec<(&str, String)> = vec![
        ("canvas", hex(window)),
        ("panel", hex(window)),
        ("panel_hover", tint(highlight, 0.25)),
        ("raised", hex(button)),
        ("raised_hover", tint(highlight, 0.25)),
        ("press", tint(highlight, 0.4)),
        ("overlay_bg", hex(window)),
        ("line", hex(text)),
        ("line_strong", hex(text)),
        ("border_focused", hex(highlight)),
        ("chrome", hex(window)),
        ("status_bg", hex(window)),
        ("tab_bg", hex(window)),
        ("scroll_thumb", hex(disabled)),
        ("input_bg", hex(window)),
        ("selection", tint(highlight, 0.5)),
        ("tab_fg", hex(text)),
        ("tab_active_fg", hex(text)),
        ("table_head_bg", hex(button)),
        ("table_head_fg", hex(button_text)),
        ("table_row_border", hex(text)),
        ("list_even", hex(window)),
        ("scroll_thumb_hover", hex(text)),
        ("text", hex(text)),
        ("text_2", hex(text)),
        ("text_3", hex(text)),
        ("text_disabled", hex(disabled)),
        ("text_on_accent", hex(highlight_text)),
        ("text_on_status", hex(window)),
        ("accent", hex(highlight)),
        ("accent_hover", hex(highlight)),
        ("accent_press", hex(highlight)),
        ("accent_tint", tint(highlight, 0.3)),
        ("accent_tint_hover", tint(highlight, 0.4)),
        ("accent_text", hex(link)),
        ("info", hex(link)),
    ];
    let code = [
        "keyword",
        "control",
        "str",
        "number",
        "constant",
        "typ",
        "function",
        "variable",
        "property",
        "operator",
        "punctuation",
        "comment",
        "tag",
        "attribute",
        "regexp",
        "escape",
        "invalid",
    ];
    let editor = [
        ("foreground", hex(text)),
        ("cursor", hex(text)),
        ("line_highlight", hex(window)),
        ("gutter", hex(disabled)),
        ("gutter_active", hex(text)),
        ("selection", tint(highlight, 0.5)),
        ("find_match", tint(highlight, 0.3)),
        ("find_current", tint(highlight, 0.6)),
        ("bracket_match", tint(highlight, 0.3)),
    ];
    let list = |items: &mut dyn Iterator<Item = (String, String)>| {
        items.map(|(key, value)| format!("\"{key}\": \"{value}\"")).collect::<Vec<_>>().join(", ")
    };
    let file = format!(
        "{{\"name\": \"High contrast\", \"mode\": \"high_contrast\", \"palette\": {{{}}}, \"syntax\": {{{}}}, \
         \"editor\": {{{}}}}}",
        list(&mut palette.into_iter().map(|(key, value)| (key.to_string(), value))),
        list(&mut code.iter().map(|key| (key.to_string(), hex(text)))),
        list(&mut editor.into_iter().map(|(key, value)| (key.to_string(), value))),
    );
    // Planes as light as a light theme's start from it, so what is left (shadows, the
    // terminal) is drawn for that lightness.
    let lightness = 0.2126 * window.r + 0.7152 * window.g + 0.0722 * window.b;
    let base = if lightness > 0.5 { light() } else { dark() };
    base.merged(&file).unwrap_or_else(|e| panic!("the theme of high contrast: {e}"))
}

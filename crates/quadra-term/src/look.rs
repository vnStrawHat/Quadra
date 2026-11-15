//! How a terminal looks (specs/18, "The view"): its face, size, lines, room around the grid
//! and colours over the theme's; a signal the view follows without being mounted again.

use std::rc::Rc;

use oneterm_vt::Palette;
use quadra_prim::Color;
use quadra_tree::TextValues;

use crate::widget::rgb;

#[derive(Clone, Debug, PartialEq)]
pub struct TermLook {
    /// A family of the system; empty, the kit's mono face.
    pub family: String,
    /// In pixels.
    pub size: f32,
    /// A line's height, times the size.
    pub line_height: f32,
    pub weight: u16,
    /// Room around the grid, in logical pixels.
    pub padding: f32,
    /// The gutter at the left: when each row was written and its line number.
    pub gutter: bool,
    pub colors: TermColors,
}

impl Default for TermLook {
    fn default() -> TermLook {
        TermLook {
            family: String::new(),
            size: 13.0,
            line_height: 20.0 / 13.0,
            weight: 400,
            padding: 8.0,
            gutter: false,
            colors: TermColors::default(),
        }
    }
}

/// Colours that stand over the theme's; `None` keeps the theme's.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TermColors {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub cursor: Option<Color>,
    pub selection: Option<Color>,
    /// The sixteen ANSI colours.
    pub ansi: [Option<Color>; 16],
    /// The gutter's text; else the foreground mixed half with the background.
    pub gutter_fg: Option<Color>,
    /// Else the terminal's background.
    pub gutter_bg: Option<Color>,
    /// The time and the line number; else `gutter_fg`.
    pub clock_fg: Option<Color>,
    pub line_number_fg: Option<Color>,
}

impl TermLook {
    pub(crate) fn text(&self) -> TextValues {
        let family = match self.family.trim() {
            "" => quadra_design::fonts::MONO,
            named => named,
        };
        TextValues {
            family: Some(Rc::from(family)),
            size: Some(self.size),
            line_height: Some(self.line_height),
            weight: Some(self.weight),
            ..TextValues::default()
        }
    }
}

impl TermColors {
    /// The theme's palette with these over it.
    pub(crate) fn over(&self, base: &Palette) -> Palette {
        let mut palette = *base;
        for (slot, color) in palette.indexed.iter_mut().zip(self.ansi) {
            if let Some(color) = color {
                *slot = rgb(color);
            }
        }
        let set = |slot: &mut oneterm_vt::Rgb, color: Option<Color>| {
            if let Some(color) = color {
                *slot = rgb(color);
            }
        };
        set(&mut palette.foreground, self.foreground);
        set(&mut palette.background, self.background);
        set(&mut palette.cursor, self.cursor);
        palette
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_stand_over_the_theme_where_they_are_said() {
        let mut colors = TermColors { foreground: Some(Color::rgb(0xEFEFEF)), ..TermColors::default() };
        colors.ansi[1] = Some(Color::rgb(0xFF0000));
        let (base, over) = (Palette::new(), colors.over(&Palette::new()));
        assert_eq!((over.foreground.r, over.foreground.g), (0xEF, 0xEF));
        assert_eq!((over.indexed[1].r, over.indexed[1].g), (0xFF, 0));
        assert_eq!(over.indexed[2], base.indexed[2]);
        assert_eq!(over.background, base.background);
    }
}

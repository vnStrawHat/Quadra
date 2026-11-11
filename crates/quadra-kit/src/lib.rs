//! Quadra's kit (specs/13; the design language is specs/behaviour/15): the roles a theme
//! gives colours to, the themes, and the components built on them.

mod theme;
pub mod token;

pub use theme::{KitTheme, Mode, ThemeError, dark, light, theme_from_json};

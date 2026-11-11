//! Quadra's kit (specs/13; the design language is specs/behaviour/15): the roles a theme
//! gives colours to, the themes, and the components built on them.

pub mod fonts;
mod root;
pub mod size;
mod theme;
pub mod token;

pub use root::{install, root};
pub use theme::{KitTheme, Mode, ThemeError, dark, light, theme_from_json};

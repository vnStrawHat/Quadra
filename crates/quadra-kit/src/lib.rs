//! Quadra's kit (specs/13; the design language is specs/behaviour/15): the roles a theme
//! gives colours to, the themes, and the components built on them.

pub mod action;
mod button;
mod choice;
mod dialog;
pub mod fonts;
mod icon;
pub mod icons;
mod input;
mod list;
mod menu;
mod popup;
mod root;
mod select;
pub mod size;
mod tabs;
mod theme;
mod toggle;
pub mod token;
mod tooltip;

pub use action::keymap;
pub use button::{Button, Size, Variant, button, icon_button};
pub use choice::{Choice, radio_group, segmented};
pub use dialog::{Dialog, dialog};
pub use icon::icon;
pub use input::{Input, input};
pub use list::{List, list};
pub use menu::{MenuItem, menu};
pub use popup::popover;
pub use root::{install, root};
pub use select::{Select, select};
pub use tabs::{Tabs, tabs};
pub use theme::{KitTheme, Mode, ThemeError, dark, light, theme_from_json};
pub use toggle::{Toggle, checkbox, switch};
pub use tooltip::{Tooltip, tooltip};

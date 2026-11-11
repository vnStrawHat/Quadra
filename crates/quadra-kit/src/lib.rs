//! Quadra's kit (specs/13; the design language is specs/behaviour/15): the roles a theme
//! gives colours to, the themes, and the components built on them.

mod accordion;
pub mod action;
mod button;
mod choice;
mod dialog;
mod display;
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
mod split;
mod tabs;
mod theme;
mod toggle;
pub mod token;
mod tooltip;

pub use accordion::{Accordion, accordion};
pub use action::keymap;
pub use button::{Button, Size, Variant, button, icon_button};
pub use choice::{Choice, radio_group, segmented};
pub use dialog::{Dialog, dialog};
pub use display::{
    Tone, avatar, badge, card, card_head, divider, heading, kbd, key_label, label, tag, vdivider,
};
pub use icon::icon;
pub use input::{Input, input};
pub use list::{List, list};
pub use menu::{MenuItem, menu};
pub use popup::popover;
pub use root::{install, root};
pub use select::{Select, select};
pub use split::{Split, split};
pub use tabs::{Tabs, tabs};
pub use theme::{KitTheme, Mode, ThemeError, dark, light, theme_from_json};
pub use toggle::{Toggle, checkbox, switch};
pub use tooltip::{Tooltip, tooltip};

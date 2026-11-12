//! Quadra's kit (specs/13; the design language is specs/behaviour/15): the roles a theme
//! gives colours to, the themes, and the components built on them.

mod accordion;
pub mod action;
mod bars;
mod button;
mod choice;
mod combobox;
mod date;
mod dialog;
mod display;
mod field;
pub mod fonts;
mod hover_card;
mod icon;
pub mod icons;
mod input;
mod list;
mod menu;
mod notes;
mod notice;
mod number;
mod palette;
mod popup;
mod progress;
mod range;
mod root;
mod select;
mod settings;
mod sidebar;
pub mod size;
mod slider;
mod split;
mod status;
mod table;
mod tabs;
mod theme;
mod time;
mod titlebar;
mod toggle;
pub mod token;
mod tooltip;
mod tree;
pub mod work;

pub use accordion::{Accordion, accordion};
pub use action::keymap;
pub use bars::{
    Breadcrumb, Statusbar, breadcrumb, status_button, status_divider, status_item, statusbar, toolbar,
};
pub use button::{Button, Size, Variant, button, icon_button};
pub use choice::{Choice, radio_group, segmented};
pub use combobox::{Combobox, combobox};
pub use date::{Date, DatePicker, calendar, date_picker};
pub use dialog::{Dialog, Drawer, confirm, dialog, drawer};
pub use display::{
    Tone, avatar, badge, card, card_head, divider, heading, kbd, key_hint, key_label, label, tag, vdivider,
};
pub use field::{Field, Rule, Textarea, field, textarea, validate};
pub use hover_card::{HoverCard, hover_card};
pub use icon::icon;
pub use input::{Input, input};
pub use list::{List, list};
pub use menu::{
    ContextMenu, MenuButton, MenuItem, context_menu, menu, menu_button, menu_icon_button, menubar,
    popover_button,
};
pub use notes::{alert, description_list, empty_state};
pub use notice::{Notice, Notices, notices};
pub use number::{NumberInput, number_input};
pub use palette::{Command, command_palette, fuzzy, fuzzy_hits, marked};
pub use popup::{Edge, popover};
pub use progress::{Busy, Skeleton, Spinner, progress, progress_busy, skeleton, spinner};
pub use range::{RangeSlider, range_slider};
pub use root::{install, root};
pub use select::{Select, select};
pub use settings::{Settings, SettingsGroup, SettingsPage, settings};
pub use sidebar::{NavItem, Sidebar, sidebar};
pub use slider::{Slider, slider};
pub use split::{Split, split};
pub use status::{Status, status_label, status_mark};
pub use table::{CellKind, Column, Table, age_text, table};
pub use tabs::{Tabs, tabs};
pub use theme::{KitTheme, Mode, ThemeError, dark, light, theme_from_json};
pub use time::{TimeInput, TimeOfDay, time_input};
pub use titlebar::{Titlebar, titlebar};
pub use toggle::{Toggle, checkbox, switch};
pub use tooltip::{Tooltip, tooltip};
pub use tree::{Tree, TreeNode, tree};

//! The pages: one a component, grouped as the sidebar shows them.

use quadra::Div;
use quadra_kit::{dark, icons, light};

use crate::pages::{actions, overview};
use crate::shell::Gallery;

pub struct PageInfo {
    pub id: &'static str,
    pub title: &'static str,
    pub group: &'static str,
    pub icon: &'static str,
}

const fn info(id: &'static str, title: &'static str, group: &'static str, icon: &'static str) -> PageInfo {
    PageInfo { id, title, group, icon }
}

pub const PAGES: &[PageInfo] = &[
    info("start", "Getting started", "Overview", icons::HOUSE),
    info("typography", "Typography", "Overview", icons::TYPE),
    info("colors", "Colors", "Overview", icons::PALETTE),
    info("icons", "Icons", "Overview", icons::STAR),
    info("button", "Button", "Actions", icons::SQUARE_CHECK),
    info("icon-button", "Icon button", "Actions", icons::SETTINGS),
    info("toggle-button", "Toggle button", "Actions", icons::BOLD),
];

/// Builds the page of an id; an id that is not a page's gives the first page. A page that
/// shows the theme reads which one the gallery has, and is built again when it changes.
pub fn make(id: &str, gallery: Gallery) -> Div {
    match id {
        "typography" => overview::typography_page(),
        "colors" => overview::colors_page(&if gallery.dark.get() { dark() } else { light() }),
        "icons" => overview::icons_page(),
        "button" => actions::button_page(),
        "icon-button" => actions::icon_button_page(),
        "toggle-button" => actions::toggle_button_page(),
        _ => overview::start_page(),
    }
}

//! The pages: one a component, grouped as the sidebar shows them.

use quadra::Div;
use quadra_kit::icons;

use crate::pages::{actions, overview};

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
    info("button", "Button", "Actions", icons::SQUARE_CHECK),
    info("icon-button", "Icon button", "Actions", icons::SETTINGS),
    info("toggle-button", "Toggle button", "Actions", icons::BOLD),
];

/// Builds the page of an id; an id that is not a page's gives the first page.
pub fn make(id: &str) -> Div {
    match id {
        "button" => actions::button_page(),
        "icon-button" => actions::icon_button_page(),
        "toggle-button" => actions::toggle_button_page(),
        _ => overview::start_page(),
    }
}

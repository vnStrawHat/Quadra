//! The pages: one a component, grouped as the sidebar shows them.

use quadra::Div;
use quadra_kit::{dark, icons, light};

use crate::pages::{actions, choices, display, fields, overview};
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
    info("checkbox", "Checkbox", "Forms", icons::SQUARE_CHECK),
    info("switch", "Switch", "Forms", icons::TOGGLE_LEFT),
    info("radio", "Radio group", "Forms", icons::CIRCLE_DOT),
    info("segmented", "Segmented control", "Forms", icons::LAYOUT_GRID),
    info("slider", "Slider", "Forms", icons::SLIDERS_HORIZONTAL),
    info("text-input", "Text input", "Forms", icons::TYPE),
    info("password", "Password input", "Forms", icons::LOCK),
    info("number", "Number input", "Forms", icons::PLUS),
    info("textarea", "Textarea", "Forms", icons::PENCIL),
    info("combobox", "Combobox", "Forms", icons::SEARCH),
    info("select", "Select", "Forms", icons::CHEVRON_DOWN),
    info("date-picker", "Date and time", "Forms", icons::CALENDAR),
    info("field", "Form field", "Forms", icons::CIRCLE_CHECK),
    info("status", "Status", "Display", icons::CIRCLE_DOT),
    info("badge", "Badge", "Display", icons::BOOKMARK),
    info("tag", "Tag", "Display", icons::TAG),
    info("kbd", "Kbd", "Display", icons::COMMAND),
    info("alert", "Alert", "Display", icons::INFO),
    info("empty-state", "Empty state", "Display", icons::SEARCH),
    info("description-list", "Description list", "Display", icons::LIST),
    info("avatar", "Avatar", "Display", icons::USER),
    info("progress", "Progress bar", "Display", icons::GAUGE),
    info("spinner", "Spinner", "Display", icons::LOADER_CIRCLE),
    info("skeleton", "Skeleton", "Display", icons::MINUS),
    info("card", "Card", "Display", icons::LAYERS),
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
        "checkbox" => choices::checkbox_page(),
        "switch" => choices::switch_page(gallery),
        "radio" => choices::radio_page(),
        "segmented" => choices::segmented_page(),
        "slider" => choices::slider_page(),
        "text-input" => fields::text_input_page(),
        "password" => fields::password_page(),
        "number" => fields::number_page(),
        "textarea" => fields::textarea_page(),
        "combobox" => fields::combobox_page(),
        "select" => fields::select_page(),
        "date-picker" => fields::date_page(),
        "field" => fields::field_page(gallery),
        "status" => display::status_page(),
        "badge" => display::badge_page(),
        "tag" => display::tag_page(),
        "kbd" => display::kbd_page(),
        "alert" => display::alert_page(),
        "empty-state" => display::empty_state_page(),
        "description-list" => display::description_list_page(),
        "avatar" => display::avatar_page(),
        "progress" => display::progress_page(),
        "spinner" => display::spinner_page(),
        "skeleton" => display::skeleton_page(),
        "card" => display::card_page(),
        _ => overview::start_page(),
    }
}

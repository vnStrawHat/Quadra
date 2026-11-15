//! The rail folded (specs/19, "Rail"): 44 px of the tabs' icons, the one that shows marked,
//! New terminal as a `+`, and what opens the rail again.

use quadra::{Align, Display, Role, Signal, View, column, div, each, row};
use quadra_design::{icon_button, icons, token, tooltip};

use crate::marks;
use crate::model::{Model, Tab};

pub(crate) fn narrow(model: &Model, folded: Signal<bool>) -> impl View + use<> {
    let (tabs, shown, adds) = (model.tabs(), model.clone(), model.clone());
    let rows =
        each(move || tabs.get(), |tab| tab.id, move |tab: Signal<Tab>| face(&shown, tab.get_untracked()));
    column()
        .width(44.0)
        .shrink(0.0)
        .align_items(Align::Center)
        .gap(6.0)
        .padding_y(8.0)
        .bg(token::PANEL)
        .border_right(1.0)
        .border_color(token::LINE)
        .child(tooltip(
            "Unfold the tabs",
            icon_button(icons::PANEL_LEFT, "Unfold the tabs").on_click(move || folded.set(false)),
        ))
        .child(rows.container(column().gap(4.0).role(Role::TabList).label("Tabs")))
        .child(tooltip(
            "New terminal",
            icon_button(icons::PLUS, "New terminal").on_click(move || adds.new_tab()),
        ))
}

/// A tab as its icon in a box of 32 px, filled while it shows, its title told to a reader.
fn face(model: &Model, tab: Tab) -> impl View + use<> {
    let (active, shows) = (model.active(), model.clone());
    let on = move || active.get() == tab.id;
    let bar = div()
        .absolute()
        .left(-6.0)
        .top(8.0)
        .bottom(8.0)
        .width(2.0)
        .radius(1.0)
        .bg(token::ACCENT)
        .display(move || if on() { Display::Flex } else { Display::None });
    let mark =
        row().align_items(Align::Center).justify(quadra::Justify::Center).child(marks::tab_icon(tab, on));
    tooltip(
        move || tab.title(),
        div()
            .size(32.0, 32.0)
            .radius(6.0)
            .align_items(Align::Center)
            .justify(quadra::Justify::Center)
            .bg(move || if on() { token::RAISED } else { token::PANEL })
            .hover(|s| s.bg(token::PANEL_HOVER))
            .role(Role::Tab)
            .label(move || tab.title())
            .selected(on)
            .on_click(move |_| shows.show(tab.id))
            .child(bar)
            .child(mark),
    )
}

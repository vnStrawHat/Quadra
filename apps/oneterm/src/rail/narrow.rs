//! The rail collapsed (specs/19, "Rail"): 56 px, Expand tabs, a box of 46 px a tab (its icon,
//! its signal as a coloured word under it, the one that shows on the editor colour with the
//! accent bar), then New terminal as a `+`.

use quadra::{Align, NodeRef, Role, Side, Signal, View, by_text, column, each, row, signal};
use quadra_design::{icon, icon_button, icons, menu_beside, token, tooltip};

use crate::marks;
use crate::model::{Model, Tab};

pub(crate) fn narrow(model: &Model, folded: Signal<bool>) -> impl View + use<> {
    let (tabs, shown) = (model.tabs(), model.clone());
    let rows =
        each(move || tabs.get(), |tab| tab.id, move |tab: Signal<Tab>| face(&shown, tab.get_untracked()));
    let expand =
        icon_button(icons::PANEL_LEFT_OPEN, "Expand tabs").muted().on_click(move || folded.set(false));
    let top = row()
        .height(by_text(28.0))
        .shrink(0.0)
        .align_items(Align::Center)
        .justify(quadra::Justify::Center)
        .child(tooltip("Expand tabs", expand));
    let (open, opener, making) = (signal(false), NodeRef::new(), model.clone());
    let new = tooltip(
        "New terminal  Ctrl-T",
        boxed()
            .text_color(token::TEXT_3)
            .hover(|s| s.bg(token::PANEL_HOVER))
            .role(Role::Button)
            .label("New terminal")
            .node_ref(&opener)
            .on_click(move |_| open.update(|open| *open = !*open))
            .child(icon(icons::PLUS).size(16.0)),
    );
    column()
        .width(56.0)
        .shrink(0.0)
        .bg(token::PANEL)
        .border_right(1.0)
        .border_color(token::LINE)
        .child(top)
        .child(
            column()
                .border_top(1.0)
                .border_color(token::LINE_STRONG)
                .child(rows.container(column().role(Role::TabList).label("Tabs")))
                .child(new),
        )
        .child(menu_beside(open, &opener, Side::Right, 4.0, move || crate::rail::new_menu(&making, "")))
}

/// A box of 40 px the rail's width, a line under it, what it holds in its middle.
fn boxed() -> quadra::Div {
    row()
        .height(40.0)
        .shrink(0.0)
        .align_items(Align::Center)
        .justify(quadra::Justify::Center)
        .border_bottom(1.0)
        .border_color(token::LINE_STRONG)
}

/// A tab as its icon in a box of 46 px with its signal as a word under it, the one that shows
/// on the pressed editor colour with the accent bar; its title and signal are its name and
/// tooltip.
fn face(model: &Model, tab: Tab) -> impl View + use<> {
    let (active, shows) = (model.active(), model.clone());
    let on = move || active.get() == tab.id;
    let column =
        column().height(46.0).shrink(0.0).align_items(Align::Center).justify(quadra::Justify::Center);
    tooltip(
        move || marks::tab_said(tab),
        column
            .gap(2.0)
            .border_bottom(1.0)
            .border_color(token::LINE_STRONG)
            .role(Role::Tab)
            .label(move || marks::tab_said(tab))
            .selected(on)
            .on_click(move |_| shows.show(tab.id))
            .child(crate::rail::active_fill(on, 1.0, false))
            .child(marks::tab_icon(tab, 16.0, on, token::TEXT_2))
            .child(marks::tab_word(tab))
            .child(marks::progress_bar(tab, -1.0))
            .child(marks::busy_bar(tab, -1.0, 55.0)),
    )
    .side(Side::Right)
    .meta(move || marks::tab_where(tab))
}

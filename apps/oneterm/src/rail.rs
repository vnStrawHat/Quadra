//! The rail (specs/19, "Rail"): 248 px of tabs on the panel colour, a heading with the
//! count, a row a tab, and "New terminal". A click shows a tab, the middle button closes it.

use quadra::{
    Align, Display, MouseButton, PointerKind, Role, Signal, View, by_text, column, div, each, row, text,
};
use quadra_design::{fonts, icon, icons, token};

use crate::model::{Model, Tab};

pub(crate) fn rail(model: &Model) -> impl View + use<> {
    let tabs = model.tabs();
    let heading = row()
        .align_items(Align::Center)
        .padding_x(14.0)
        .height(by_text(36.0))
        .text_size(11.0)
        .text_color(token::TEXT_3)
        .child(div().grow(1.0).child(text("TABS")))
        .child(div().font_family(fonts::MONO).child(text(move || tabs.with(Vec::len).to_string())));
    let shown = model.clone();
    let rows =
        each(move || tabs.get(), |tab| tab.id, move |tab: Signal<Tab>| tab_row(&shown, tab.get_untracked()));
    let making = model.clone();
    let new = row()
        .align_items(Align::Center)
        .gap(10.0)
        .padding_x(8.0)
        .height(by_text(32.0))
        .radius(6.0)
        .text_color(token::TEXT_2)
        .hover(|s| s.bg(token::PANEL_HOVER).text_color(token::TEXT))
        .role(Role::Button)
        .label("New terminal")
        .on_click(move |_| making.new_tab())
        .child(icon(icons::PLUS).size(14.0))
        .child(div().grow(1.0).child(text("New terminal")))
        .child(
            div().font_family(fonts::MONO).text_size(11.0).text_color(token::TEXT_3).child(text("Ctrl+T")),
        );
    column()
        .width(248.0)
        .shrink(0.0)
        .bg(token::PANEL)
        .border_right(1.0)
        .border_color(token::LINE)
        .role(Role::TabList)
        .label("Tabs")
        .child(heading)
        .child(rows.container(column().gap(2.0).padding_x(8.0)))
        .child(div().padding_x(8.0).padding_y(4.0).child(new))
}

/// A tab's row: the accent bar and fill while it shows, its icon, its title and a line on
/// what it holds.
fn tab_row(model: &Model, tab: Tab) -> impl View + use<> {
    let active = model.active();
    let on = move || active.get() == tab.id;
    let spaces = move || {
        let n = tab.spaces.with(Vec::len);
        format!("{n} local \u{B7} {n} {}", if n == 1 { "space" } else { "spaces" })
    };
    let (shows, closes) = (model.clone(), model.clone());
    let bar = div()
        .absolute()
        .left(0.0)
        .top(8.0)
        .bottom(8.0)
        .width(2.0)
        .bg(token::ACCENT)
        .display(move || if on() { Display::Flex } else { Display::None });
    let words = column()
        .grow(1.0)
        .min_width(0.0)
        .child(div().text_color(token::TEXT).child(text(move || tab.title()).ellipsis()))
        .child(
            div()
                .text_size(11.0)
                .font_family(fonts::MONO)
                .text_color(token::TEXT_2)
                .child(text(spaces).ellipsis()),
        );
    row()
        .align_items(Align::Center)
        .gap(10.0)
        .padding_x(10.0)
        .height(by_text(46.0))
        .radius(6.0)
        .bg(move || if on() { token::RAISED } else { token::PANEL })
        .hover(|s| s.bg(token::PANEL_HOVER))
        .role(Role::Tab)
        .label(move || tab.title())
        .selected(on)
        .on_click(move |_| shows.active().set(tab.id))
        .on_pointer(PointerKind::Click, move |e| {
            if e.button == MouseButton::Middle {
                closes.close_tab(tab.id);
            }
        })
        .child(bar)
        .child(icon(icons::TERMINAL).size(14.0).color(token::TEXT_2))
        .child(words)
}

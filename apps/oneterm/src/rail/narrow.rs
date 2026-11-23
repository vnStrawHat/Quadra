//! The rail collapsed (specs/19, "Rail"): 56 px, Expand tabs, a box of 46 px a tab (its icon,
//! its signal as a coloured word under it, the one that shows on the editor colour with the
//! accent bar; the middle button closes it), then New terminal as a `+`.

use quadra::{
    Align, MouseButton, NodeRef, PointerKind, Role, Side, Signal, View, by_text, column, each, row, signal,
};
use quadra_design::{icon, icon_button, icons, searched_menu, token, tooltip};

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
    // The expanded rail's dashed box, 4 px from the row's seams and 6 from its sides.
    let dashed = row()
        .grow(1.0)
        .align_items(Align::Center)
        .justify(quadra::Justify::Center)
        .radius(6.0)
        .border(1.0)
        .border_style(quadra::BorderStyle::Dashed)
        .border_color(token::LINE_STRONG)
        .hover(|s| s.bg(token::PANEL_HOVER))
        .role(Role::Button)
        .label("New terminal")
        .node_ref(&opener)
        .on_click(move |_| open.update(|open| *open = !*open))
        .child(icon(icons::PLUS).size(16.0));
    let new = tooltip(
        "New terminal  Ctrl-T",
        boxed()
            .align_items(Align::Stretch)
            .padding_x(6.0)
            .padding_y(4.0)
            .text_color(token::TEXT_3)
            .child(dashed),
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
        .child(searched_menu(open, &opener, Side::Right, crate::rail::ASK, move |typed: &str| {
            crate::rail::new_menu(&making, typed)
        }))
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
/// on the pressed editor colour with the accent bar; its title and signal are its name. Its
/// tooltip is two lines: where its shell is, and what it runs (read when it shows).
fn face(model: &Model, tab: Tab) -> impl View + use<> {
    let (active, shows, closes) = (model.active(), model.clone(), model.clone());
    let on = move || active.get() == tab.id;
    let column =
        column().height(46.0).shrink(0.0).align_items(Align::Center).justify(quadra::Justify::Center);
    // While the shell has said no directory, the program it was started as.
    let first = move || match marks::tab_dir(tab) {
        dir if dir.is_empty() => marks::tab_program(tab),
        dir => dir,
    };
    let second = move || marks::tab_runs(tab);
    tooltip(
        first,
        column
            .gap(2.0)
            .hover(|s| s.bg(token::PANEL_HOVER.alpha(0.5)))
            .role(Role::Tab)
            .label(move || marks::tab_said(tab))
            .selected(on)
            .on_click(move |_| shows.show(tab.id))
            .on_pointer(PointerKind::Click, move |e| {
                if e.button == MouseButton::Middle {
                    closes.close_tab(tab.id);
                }
            })
            .child(crate::rail::active_fill(on, 1.0, false))
            .child(marks::tab_icon(tab, 16.0, on, token::TEXT_2))
            .child(marks::tab_word(tab))
            .child(marks::progress_bar(tab, 0.0))
            .child(marks::busy_bar(tab, 0.0, 55.0)),
    )
    .side(Side::Right)
    .meta(second)
}

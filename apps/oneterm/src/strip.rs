//! The horizontal variant of the tabs (specs/19, "Rail"): a strip of 36 px under the title
//! bar, a button a tab (the one that shows on the editor colour with an accent line under
//! it), `+` after the last, and "Vertical" at the end to go back to the rail.

use quadra::{Align, Display, MouseButton, PointerKind, Role, Signal, View, by_text, div, each, row};
use quadra_design::{button, context_menu, icon, icon_button, icons, token};

use crate::model::{Model, Tab};
use crate::rail::{Dragged, tab_menu, title};

pub(crate) fn strip(model: &Model) -> impl View + use<> {
    let (tabs, vertical, making) = (model.tabs(), model.vertical(), model.clone());
    let shown = model.clone();
    let buttons = each(
        move || tabs.get(),
        |tab| tab.id,
        move |tab: Signal<Tab>| tab_button(&shown, tab.get_untracked()),
    );
    row()
        .align_items(Align::Center)
        .height(by_text(36.0))
        .shrink(0.0)
        .bg(token::TAB_BG)
        .border_bottom(1.0)
        .border_color(token::LINE)
        .role(Role::TabList)
        .label("Tabs")
        .child(buttons.container(row().align_items(Align::Stretch).min_width(0.0)))
        .child(
            div()
                .padding_x(4.0)
                .child(icon_button(icons::PLUS, "New terminal").on_click(move || making.new_tab())),
        )
        .child(div().grow(1.0))
        .child(div().padding_x(6.0).child(button("Vertical").on_click(move || vertical.set(true))))
}

fn tab_button(model: &Model, tab: Tab) -> impl View + use<> {
    let active = model.active();
    let on = move || active.get() == tab.id;
    let (shows, closes, moves, menu) = (model.clone(), model.clone(), model.clone(), model.clone());
    let line = div()
        .absolute()
        .left(0.0)
        .right(0.0)
        .bottom(0.0)
        .height(2.0)
        .bg(token::ACCENT)
        .display(move || if on() { Display::Flex } else { Display::None });
    let face = row()
        .align_items(Align::Center)
        .gap(8.0)
        .padding_x(12.0)
        .max_width(220.0)
        .min_width(0.0)
        .bg(move || if on() { token::CANVAS } else { token::TAB_BG })
        .text_color(move || if on() { token::TAB_ACTIVE_FG } else { token::TAB_FG })
        .border_right(1.0)
        .border_color(token::LINE)
        .hover(|s| s.bg(token::PANEL_HOVER))
        .drop_target(|s| s.bg(token::ACCENT_TINT))
        .role(Role::Tab)
        .label(move || tab.title())
        .selected(on)
        .on_click(move |e| {
            shows.active().set(tab.id);
            if e.clicks == 2 {
                shows.renaming().set(Some(tab.id));
            }
        })
        .on_pointer(PointerKind::Click, move |e| {
            if e.button == MouseButton::Middle {
                closes.close_tab(tab.id);
            }
        })
        .draggable(move || Dragged(tab.id))
        .on_drop::<Dragged>(move |dragged, _| {
            let to = moves.tabs().with_untracked(|tabs| tabs.iter().position(|it| it.id == tab.id));
            moves.move_tab(dragged.0, to.unwrap_or(usize::MAX));
        })
        .child(icon(icons::TERMINAL).size(14.0))
        .child(div().min_width(0.0).child(title(model, tab)))
        .child(line);
    context_menu(face, move || tab_menu(&menu, tab.id))
}

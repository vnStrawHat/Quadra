//! The window's title bar (specs/19, "Window"): 38 px with the app's mark ("Locked" after it
//! while locked) and the window's buttons: Search, SFTP, Notifications, Lock, Settings; in the
//! settings, Done.

use quadra::{Align, Display, NodeRef, Side, View, div, row, signal, text};
use quadra_design::{button, icon, icon_button, icons, searched_menu, titlebar, token, tooltip};

use crate::locked::shown;
use crate::model::Model;

pub(crate) fn title_bar(model: &Model) -> impl View + use<> {
    let (model, seen, lock) = (model.clone(), model.clone(), model.lock());
    let (expanded, settings) = (model.ui().expanded, model.ui().settings);
    let locked = div()
        .text_size(12.0)
        .text_color(token::TEXT_2)
        .display(shown(move || lock.locked.get()))
        .child(text("Locked"));
    let mark = row()
        .align_items(Align::Center)
        .gap(8.0)
        .padding_left(12.0)
        .child(icon(icons::TERMINAL).size(16.0).color(token::ACCENT))
        .child(div().font_weight(600).child(text("1Term")))
        .child(locked);
    let (dock, open, hosts, vertical) =
        (seen.ui().dock, seen.ui().settings, seen.ui().hosts, seen.vertical());
    // With the tabs in a strip, the hosts are a panel this opens.
    let panel = div().display(shown(move || !vertical.get())).child(
        button("Hosts")
            .icon(icons::APP_WINDOW)
            .variant(quadra_design::Variant::Secondary)
            .size(quadra_design::Size::Sm)
            .on_click(move || hosts.update(|on| *on = !*on)),
    );
    // The settings open in a window of their own; where there are no windows to open (the
    // harness), in this one.
    let (windows, opens) = (quadra::use_context::<quadra::Windows>(), seen.clone());
    let gear = tooltip(
        "Settings",
        icon_button(icons::SETTINGS, "Settings").on_click(move || match &windows {
            Some(windows) => crate::page::open_window(windows, &opens),
            None => open.update(|on| *on = !*on),
        }),
    );
    let collapse = div()
        .display(shown(move || expanded.get()))
        .child(button("Collapse SFTP").icon(icons::WINDOW_RESTORE).on_click(move || expanded.set(false)));
    let files =
        tooltip("SFTP", icon_button(icons::FOLDER, "SFTP").on_click(move || dock.update(|on| *on = !*on)));
    let locks = div()
        .display(shown(move || lock.can_lock()))
        .child(tooltip("Lock 1Term now", icon_button(icons::LOCK, "Lock").on_click(move || lock.lock_now())));
    // Locked, what would open something is not displayed; in the settings, only Done.
    let buttons = row()
        .align_items(Align::Center)
        .gap(4.0)
        .display(shown(move || !lock.locked.get() && !settings.get()))
        .child(collapse)
        .child(panel)
        .child(search(&seen))
        .child(files)
        .child(notifications(&seen))
        .child(locks)
        .child(gear);
    let done =
        div().display(shown(move || settings.get())).child(button("Done").on_click(move || open.set(false)));
    titlebar("").height(38.0).control(mark).control(div().grow(1.0)).control(buttons).control(done)
}

/// Search: the new terminal menu under the title bar's button, with the tabs that match.
fn search(model: &Model) -> impl View + use<> {
    let (open, opener, making) = (signal(false), NodeRef::new(), model.clone());
    let face = icon_button(icons::SEARCH, "Search sessions and hosts")
        .on_click(move || open.update(|on| *on = !*on));
    div().child(div().node_ref(&opener).child(tooltip("Search sessions and hosts", face))).child(
        searched_menu(open, &opener, Side::Below, crate::rail::ASK, move |typed: &str| {
            crate::rail::new_menu(&making, typed)
        }),
    )
}

/// The button of notifications, with how many came unseen; a press marks them seen.
fn notifications(model: &Model) -> impl View + use<> {
    let unread = model.unread();
    let count = row()
        .absolute()
        .top(3.0)
        .right(2.0)
        .min_width(14.0)
        .height(14.0)
        .padding_x(4.0)
        .radius(7.0)
        .align_items(Align::Center)
        .justify(quadra::Justify::Center)
        .bg(token::ERROR)
        .text_color(token::TEXT_ON_ACCENT)
        .text_size(10.0)
        .font_weight(600)
        .display(move || if unread.get() > 0 { Display::Flex } else { Display::None })
        .child(text(move || unread.get().to_string()));
    let button = icon_button(icons::BELL, "Notifications").on_click(move || unread.set(0));
    div().child(tooltip("Notifications", button)).child(count)
}

//! The window's title bar (specs/19, "Window"): 38 px with the app's mark ("Locked" after it
//! while locked), icon buttons of 22 px (Search, SSH hosts with the tabs in a strip, the bells
//! and notices with their count, Lock, Settings), then the window's buttons; in the settings,
//! Done.

use quadra::{Align, NodeRef, Role, Side, View, div, row, signal, text};
use quadra_design::{button, fonts, icon, icon_button, icons, searched_menu, titlebar, token, tooltip};

use crate::locked::shown;
use crate::model::Model;

/// 1Term's mark: a terminal's prompt in a window.
const MARK: &str =
    r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M7 9l3 3-3 3"/><path d="M13 15h4"/>"#;

pub(crate) fn title_bar(model: &Model) -> impl View + use<> {
    let (seen, lock) = (model.clone(), model.lock());
    let (expanded, settings) = (model.ui().expanded, model.ui().settings);
    let locked =
        div().text_color(token::TEXT_2).display(shown(move || lock.locked.get())).child(text("Locked"));
    let mark = row()
        .align_items(Align::Center)
        .gap(4.0)
        .padding_left(12.0)
        .child(icon(MARK).size(16.0).color(token::ACCENT))
        .child(div().padding_x(4.0).text_size(13.0).font_weight(600).child(text("1Term")))
        .child(locked);
    let (open, hosts, vertical) = (seen.ui().settings, seen.ui().hosts, seen.vertical());
    // With the tabs in a strip, the hosts are a panel this opens.
    let panel = div().display(shown(move || !vertical.get())).child(tooltip(
        "SSH hosts",
        icon_button(icons::NETWORK, "SSH hosts").muted().on_click(move || hosts.update(|on| *on = !*on)),
    ));
    // The settings open in a window of their own; where there are no windows to open (the
    // harness), in this one.
    let (windows, opens) = (quadra::use_context::<quadra::Windows>(), seen.clone());
    let gear = tooltip(
        "Open settings",
        icon_button(icons::SETTINGS, "Settings").muted().on_click(move || match &windows {
            Some(windows) => crate::page::open_window(windows, &opens),
            None => open.update(|on| *on = !*on),
        }),
    );
    let collapse = div()
        .display(shown(move || expanded.get()))
        .child(button("Collapse SFTP").icon(icons::WINDOW_RESTORE).on_click(move || expanded.set(false)));
    let locks = div().display(shown(move || lock.can_lock())).child(tooltip(
        "Lock 1Term now",
        icon_button(icons::LOCK, "Lock").muted().on_click(move || lock.lock_now()),
    ));
    // Locked, what would open something is not displayed; in the settings, only Done.
    let buttons = row()
        .align_items(Align::Center)
        .gap(4.0)
        .padding_right(8.0)
        .display(shown(move || !lock.locked.get() && !settings.get()))
        .child(collapse)
        .child(search(&seen))
        .child(panel)
        .child(bells(&seen))
        .child(locks)
        .child(gear);
    let done = div()
        .padding_right(8.0)
        .display(shown(move || settings.get()))
        .child(button("Done").on_click(move || open.set(false)));
    titlebar("").height(38.0).control(mark).control(div().grow(1.0)).control(buttons).control(done)
}

/// Search: the `+` menu under the title bar's button, with a field that narrows it.
fn search(model: &Model) -> impl View + use<> {
    let (open, opener, making) = (signal(false), NodeRef::new(), model.clone());
    let face = icon_button(icons::SEARCH, "Search tabs, hosts and commands")
        .muted()
        .on_click(move || open.update(|on| *on = !*on));
    div().child(div().node_ref(&opener).child(tooltip("Search tabs, hosts and commands", face))).child(
        searched_menu(open, &opener, Side::Below, crate::rail::ASK, move |typed: &str| {
            crate::rail::new_menu(&making, typed)
        }),
    )
}

/// The bells not heard and the notices not seen, one count after a bell, in the error colour
/// while there are any; a click marks the notices seen.
fn bells(model: &Model) -> impl View + use<> {
    let (tabs, unread) = (model.tabs(), model.unread());
    let count = move || crate::signals::all_bells(tabs) + unread.get();
    let tone = move || if count() > 0 { token::ERROR } else { token::TEXT_2 };
    let face = row()
        .align_items(Align::Center)
        .gap(4.0)
        .padding_x(4.0)
        .height(22.0)
        .radius(4.0)
        .hover(|s| s.bg(token::PANEL_HOVER))
        .role(Role::Button)
        .label(move || format!("{} bells and notices", count()))
        .on_click(move |_| unread.set(0))
        .child(div().text_color(tone).child(icon(icons::BELL).size(14.0)))
        .child(
            div()
                .font_family(fonts::MONO)
                .text_size(12.0)
                .text_color(tone)
                .child(text(move || count().to_string())),
        );
    tooltip("Bells and notices", face)
}

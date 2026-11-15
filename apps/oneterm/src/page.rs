//! The settings page (specs/19, "Settings"): it takes the window's body while it is open, in
//! the kit's layout of cards: the pages with their icons and a search in the column, each
//! page's title and what it is about, its groups. Each control writes a setting, which saves it.

use quadra::{Align, column, div, effect, row, signal, text};
use quadra_design::{
    SettingsGroup, SettingsPage, Variant, button, icon, icons, input, key_hint, select, settings, switch,
    token,
};

use crate::model::Model;

mod look;
use crate::prefs::{self, Prefs};

/// Each page: its name, its icon and what it is about.
const PAGES_ABOUT: [(&str, &str, &str); 10] = [
    ("Appearance", icons::PALETTE, "The theme, and the terminal's text and colours."),
    (
        "Terminal",
        icons::SQUARE_TERMINAL,
        "The cursor, the history and the shell. Changes apply to new spaces.",
    ),
    ("Tabs & spaces", icons::LAYERS, "Where the tabs stand."),
    ("SSH & hosts", icons::APP_WINDOW, "The hosts saved in the rail and the targets connected to lately."),
    ("SFTP", icons::FOLDER, "Where downloads go."),
    ("Notifications & signals", icons::BELL, "What programs say with escape sequences, and how it shows."),
    ("Status bar", icons::GAUGE, "What the status bar shows."),
    ("Security & lock", icons::LOCK, "The master password and the lock when idle."),
    ("Keybindings", icons::COMMAND, "The app's keys."),
    ("Advanced", icons::CODE, "The settings file."),
];

/// The app's keys as the Keybindings page lists them.
const KEYS: [(&str, &str); 8] = [
    ("ctrl-t", "New tab"),
    ("ctrl-d", "Split the space"),
    ("ctrl-w", "Close the space"),
    ("ctrl-alt-right", "Next space"),
    ("ctrl-alt-left", "Previous space"),
    ("ctrl-shift-c", "Copy"),
    ("ctrl-shift-v", "Paste"),
    ("ctrl-shift-l", "Lock"),
];

/// Opens the settings in a window of their own, or brings it to the front: its own title bar
/// and the page. It closes when 1Term locks, and with the main window.
pub(crate) fn open_window(windows: &quadra::Windows, model: &Model) {
    let model = model.clone();
    let options =
        quadra::WindowOptions::new("1Term Settings").size(980.0, 700.0).custom_frame().remember("settings");
    windows.open(options, move || {
        let (locked, me) = (model.lock().locked, quadra::window());
        effect(move || {
            if locked.get() {
                me.close_now();
            }
        });
        let mark = row()
            .align_items(Align::Center)
            .gap(8.0)
            .padding_left(12.0)
            .child(icon(icons::SETTINGS).size(16.0).color(token::ACCENT))
            .child(div().font_weight(600).child(text("1Term Settings")));
        // Using the settings is using 1Term: idle starts again.
        crate::locked::watches(quadra_design::root(), model.lock())
            .child(quadra_design::titlebar("").height(38.0).control(mark).control(div().grow(1.0)))
            .child(page(&model))
    });
}

pub(crate) fn page(model: &Model) -> quadra::Div {
    // The title bar says the page and has Done.
    let (p, current, query) = (model.prefs(), model.ui().page, signal(String::new()));
    let mut view = settings(current).search(query).cards();
    for ((name, body, about), groups) in PAGES_ABOUT.into_iter().zip(groups(p, model)) {
        let page = SettingsPage::new(name).icon(body).about(about);
        view = view.page(groups.into_iter().fold(page, SettingsPage::group));
    }
    column().grow(1.0).min_height(0.0).child(view)
}

fn groups(p: Prefs, model: &Model) -> Vec<Vec<SettingsGroup>> {
    let tabs = SettingsGroup::new("Tabs & spaces").row(
        "Vertical tabs",
        "Tabs in a rail at the left; off, in a strip above the spaces.",
        switch(p.vertical),
    );
    let signals = SettingsGroup::new("Signals")
        .row("Program states", "OSC 7501.", select(prefs::STATUS_SHOWN, p.status_shown).width(240.0))
        .row("Card when blocked", "A card when a program waits or failed.", switch(p.toast_blocked))
        .row("Progress on the tab", "OSC 9;4 under the tab.", switch(p.progress_tab))
        .row("Progress in the status bar", "OSC 9;4 as a meter.", switch(p.progress_bar))
        .row("Bell", "A bell in a space not looked at.", select(prefs::BELLS, p.bell).width(240.0))
        .row("Notifications", "OSC 9.", select(prefs::NOTICES, p.notices).width(240.0));
    let status = SettingsGroup::new("Status bar")
        .row("Git", "The branch and the files changed.", switch(p.show_git))
        .row("CPU and memory", "What 1Term uses.", switch(p.show_usage))
        .row("Clock", "The local time.", switch(p.show_clock));
    let lock = crate::locked::security(model.lock(), p.lock_after);
    let keys = KEYS
        .iter()
        .fold(SettingsGroup::new("Keys"), |group, (keys, does)| group.row(does, "", key_hint(keys, "")));
    let file = quadra::AppDirs::for_app("1Term")
        .map(|dirs| dirs.config.join("settings.json").display().to_string())
        .unwrap_or_default();
    let advanced = SettingsGroup::new("Files").row("Settings file", &file, div());
    let note = row()
        .align_items(Align::Center)
        .gap(10.0)
        .text_color(token::TEXT_2)
        .child(icon(icons::INFO).size(14.0))
        .child(text(format!("Settings are saved to {file} as you change them.")));
    vec![
        look::appearance(p),
        look::terminal(p),
        vec![tabs],
        vec![hosts(p, model)],
        vec![SettingsGroup::new("Downloads").row(
            "Downloads folder",
            "Where a file downloaded from the dock goes; empty, the user's Downloads.",
            input(p.downloads).placeholder("Downloads").width(240.0),
        )],
        vec![signals],
        vec![status],
        vec![lock],
        vec![keys],
        vec![advanced, SettingsGroup::new("").wide(note)],
    ]
}

/// The saved hosts and the recent targets, with what adds one and what forgets them.
fn hosts(p: Prefs, model: &Model) -> SettingsGroup {
    let (hosts, recent, adds, closes) = (p.hosts, p.recent, model.ui().adding, model.ui().settings);
    let count = move |n: usize, one: &str| format!("{n} {one}{}", if n == 1 { "" } else { "s" });
    let saved = row()
        .align_items(Align::Center)
        .gap(12.0)
        .child(text(move || count(hosts.with(Vec::len), "host")))
        .child(button("Add SSH host\u{2026}").variant(Variant::Secondary).on_click(move || {
            closes.set(false);
            adds.set(true);
        }));
    let lately = row()
        .align_items(Align::Center)
        .gap(12.0)
        .child(text(move || count(recent.with(Vec::len), "target")))
        .child(button("Forget").variant(Variant::Secondary).on_click(move || recent.set(Vec::new())));
    SettingsGroup::new("Hosts")
        .row("Saved hosts", "Edited from the rail: a host's menu has Edit and Remove.", saved)
        .row("Recent", "The last targets connected to, in the + menu.", lately)
}

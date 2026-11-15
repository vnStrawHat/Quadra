//! The settings page (specs/19, "Settings"): it takes the window's body while it is open,
//! `1Term Settings · <page>` above the page, a search in the column. Each control writes a
//! setting, which saves it.

use quadra::{Align, Signal, View, column, div, effect, row, signal, text, untrack};
use quadra_design::{
    SettingsGroup, SettingsPage, button, icons, key_hint, number_input, segmented, select, settings, switch,
};

use crate::model::Model;
use crate::prefs::{self, Prefs};

const PAGES: [&str; 8] = [
    "Appearance",
    "Terminal",
    "Tabs & spaces",
    "Notifications & signals",
    "Status bar",
    "Security & lock",
    "Keybindings",
    "Advanced",
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
    ("ctrl-shift-p", "Command palette"),
];

pub(crate) fn page(model: &Model) -> quadra::Div {
    let (p, open) = (model.prefs(), model.settings_open());
    let (current, query) = (signal(0usize), signal(String::new()));
    let head = row()
        .align_items(Align::Center)
        .gap(8.0)
        .child(div().grow(1.0).child(text(move || format!("1Term Settings \u{B7} {}", PAGES[current.get()]))))
        .child(button("Back to the terminal").on_click(move || open.set(false)));
    let mut view = settings(current).search(query).header(head);
    for (at, group) in groups(p).into_iter().enumerate() {
        let icon = [icons::PALETTE, icons::TERMINAL, icons::LAYERS, icons::BELL, icons::LIST, icons::LOCK]
            .get(at)
            .copied()
            .unwrap_or(icons::SETTINGS);
        view = view.page(SettingsPage::new(PAGES[at]).icon(icon).group(group));
    }
    column().grow(1.0).min_height(0.0).child(view)
}

fn groups(p: Prefs) -> Vec<SettingsGroup> {
    let appearance = SettingsGroup::new("Colours and text")
        .row("Theme", "System follows the OS's dark or light mode.", segmented(prefs::THEMES, p.theme))
        .row("Font size", "The terminal's text, in pixels.", number_input(p.font_size).range(8.0, 32.0));
    let terminal = SettingsGroup::new("Terminal")
        .row("Cursor shape", "Until a program sets its own.", segmented(prefs::CURSORS, p.cursor))
        .row("Cursor blink", "Whether the cursor blinks.", switch(p.blink))
        .row(
            "Scrollback lines",
            "The history each space keeps.",
            number_input(p.scrollback).range(0.0, 100_000.0).step(1000.0),
        )
        .row("Default shell", "What Ctrl+T and New tab start.", shell_choice(p.shell))
        .row(
            "New spaces start in",
            "Where a new tab's shell starts.",
            select(prefs::STARTS, p.start).width(240.0),
        );
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
    let lock = SettingsGroup::new("Master password").about("Comes with the lock screen.");
    let keys = KEYS
        .iter()
        .fold(SettingsGroup::new("Keys"), |group, (keys, does)| group.row(does, "", key_hint(keys, "")));
    let file = quadra::AppDirs::for_app("1Term")
        .map(|dirs| dirs.config.join("settings.json").display().to_string())
        .unwrap_or_default();
    let advanced = SettingsGroup::new("Files").row("Settings file", &file, div());
    vec![appearance, terminal, tabs, signals, status, lock, keys, advanced]
}

/// The default shell as a choice of the shells found, written as its name; the first is
/// the OS's own.
fn shell_choice(shell: Signal<String>) -> impl View {
    let names: Vec<String> = std::iter::once("The OS's".to_string())
        .chain(crate::shells::found().into_iter().map(|it| it.name.into()))
        .collect();
    let at = untrack(|| names.iter().position(|it| *it == shell.get()).unwrap_or(0));
    let chosen = signal(at);
    let words = names.clone();
    effect(move || {
        let name = if chosen.get() == 0 { String::new() } else { words[chosen.get()].clone() };
        if untrack(|| shell.get()) != name {
            shell.set(name);
        }
    });
    select(names, chosen).width(240.0)
}

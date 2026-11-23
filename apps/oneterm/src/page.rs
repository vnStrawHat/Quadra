//! The settings page (specs/19, "Settings"): it takes the window's body while it is open, in
//! the kit's layout of cards: the pages with their icons and a search in the column, each
//! page's title and what it is about, its groups. Each control writes a setting, which saves it.

use quadra::{Align, Signal, column, div, effect, row, signal, text};
use quadra_design::{
    SettingsGroup, SettingsPage, Variant, button, icon, icons, input, number_input, segmented, select,
    settings, switch, token,
};

use crate::model::Model;

mod complete;
pub(crate) mod keys;
mod look;
use crate::prefs::{self, Prefs};

/// Each page: its name, its icon and what it is about.
const PAGES_ABOUT: [(&str, &str, &str); 11] = [
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
    ("About", icons::INFO, "Which 1Term this is."),
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

/// A row that goes back to the default of its setting: `default` is the same setting where
/// nothing was written (`Prefs::defaults`).
pub(crate) trait Back {
    fn back<T: PartialEq + Clone + 'static>(self, value: Signal<T>, default: Signal<T>) -> Self;
}

impl Back for SettingsGroup {
    fn back<T: PartialEq + Clone + 'static>(self, value: Signal<T>, default: Signal<T>) -> Self {
        self.resets(value, default.get_untracked())
    }
}

pub(crate) fn page(model: &Model) -> quadra::Div {
    // The title bar says the page and has Done.
    let (p, current, query) = (model.prefs(), model.ui().page, signal(String::new()));
    let mut view = settings(current).search(query).cards();
    for ((name, body, about), groups) in PAGES_ABOUT.into_iter().zip(groups(p, Prefs::defaults(), model)) {
        let page = SettingsPage::new(name).icon(body).about(about);
        view = view.page(groups.into_iter().fold(page, SettingsPage::group));
    }
    column().grow(1.0).min_height(0.0).child(view)
}

fn groups(p: Prefs, d: Prefs, model: &Model) -> Vec<Vec<SettingsGroup>> {
    let tabs = SettingsGroup::new("Tabs & spaces")
        .row(
            "Vertical tabs",
            "Tabs in a rail at the left; off, in a strip above the spaces.",
            switch(p.vertical),
        )
        .back(p.vertical, d.vertical)
        .row(
            "Rail width",
            "The vertical tabs' width, from 140 to 400 px.",
            number_input(p.rail_width).stepper().range(140.0, 400.0).step(10.0).label("Rail width"),
        )
        .back(p.rail_width, d.rail_width)
        .row("Fold the rail", "Only the tabs' icons, in 48 px.", switch(p.rail_folded))
        .back(p.rail_folded, d.rail_folded)
        .row("Tab title", "What names a tab not renamed.", select(prefs::TITLES, p.tab_title).width(240.0))
        .back(p.tab_title, d.tab_title);
    let signals = SettingsGroup::new("Signals")
        .row("Program states", "OSC 7501.", select(prefs::STATUS_SHOWN, p.status_shown).width(240.0))
        .back(p.status_shown, d.status_shown)
        .row("Progress on the tab", "OSC 9;4 under the tab.", switch(p.progress_tab))
        .back(p.progress_tab, d.progress_tab)
        .row("Progress in the status bar", "OSC 9;4 as a meter.", switch(p.progress_bar))
        .back(p.progress_bar, d.progress_bar)
        .row("Bell", "A bell in a space not looked at.", select(prefs::BELLS, p.bell).width(240.0))
        .back(p.bell, d.bell)
        .row("Notifications", "OSC 9.", select(prefs::NOTICES, p.notices).width(240.0))
        .back(p.notices, d.notices);
    let status = SettingsGroup::new("Status bar")
        .row("Git", "The branch and the files changed.", switch(p.show_git))
        .back(p.show_git, d.show_git)
        .row("CPU and memory", "What 1Term uses.", switch(p.show_usage))
        .back(p.show_usage, d.show_usage)
        .row("Clock", "The local time.", switch(p.show_clock))
        .back(p.show_clock, d.show_clock)
        .row("Frames a second", "What the window draws, counted each second.", switch(p.show_fps))
        .back(p.show_fps, d.show_fps);
    let lock = crate::locked::security(model.lock(), p.lock_after, d.lock_after);
    let keys = keys::group();
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
        look::appearance(p, d),
        look::terminal(p, d).into_iter().chain([complete::completion(model), logging(p, d)]).collect(),
        vec![tabs],
        vec![
            hosts(p, model),
            crate::sshkeys::group(model),
            SettingsGroup::new("Connections")
                .row("Keepalive", "Asked of an idle server so the connection stays.", switch(p.keepalive_on))
                .back(p.keepalive_on, d.keepalive_on)
                .row(
                    "Every",
                    "Seconds between two, from 5 to 3600.",
                    number_input(p.keepalive_secs).stepper().range(5.0, 3600.0).step(5.0).label("Every"),
                )
                .back(p.keepalive_secs, d.keepalive_secs)
                .row(
                    "Unanswered",
                    "How many go unanswered before the connection is given up.",
                    number_input(p.keepalive_max).stepper().range(1.0, 20.0).label("Unanswered"),
                )
                .back(p.keepalive_max, d.keepalive_max),
        ],
        vec![
            SettingsGroup::new("Downloads")
                .row(
                    "Downloads folder",
                    "Where a file downloaded from the dock goes; empty, the user's Downloads.",
                    input(p.downloads).placeholder("Downloads").label("Downloads folder").width(240.0),
                )
                .back(p.downloads, d.downloads),
            SettingsGroup::new("Editing")
                .row(
                    "Editor",
                    "What a file's Edit opens it in.",
                    select(crate::sftp::EDITORS, p.editor).width(240.0),
                )
                .back(p.editor, d.editor)
                .row(
                    "Program",
                    "The editor's program, when it is one.",
                    input(p.editor_program).placeholder("notepad.exe").label("Program").width(240.0),
                )
                .back(p.editor_program, d.editor_program)
                .row(
                    "Arguments",
                    "Before the file, split at spaces.",
                    input(p.editor_args).label("Arguments").width(240.0),
                )
                .back(p.editor_args, d.editor_args)
                .row(
                    "Asked above",
                    "A file larger, in KB, is edited only when asked; 0, any.",
                    number_input(p.edit_limit_kb).stepper().range(0.0, 1_048_576.0).step(256.0).label("Asked above"),
                )
                .back(p.edit_limit_kb, d.edit_limit_kb),
        ],
        vec![signals],
        vec![status],
        vec![
            lock,
            SettingsGroup::new("Clipboard")
                .row(
                    "Programs read the clipboard",
                    "OSC 52 asked with `?`, by a local or a remote program. Writing it is always let through.",
                    switch(p.clipboard_read),
                )
                .back(p.clipboard_read, d.clipboard_read),
        ],
        vec![keys],
        vec![advanced, SettingsGroup::new("").wide(note)],
        vec![
            SettingsGroup::new("1Term")
                .row("Version", "", text(env!("CARGO_PKG_VERSION")))
                .row("Terminal engine", "", text("oneterm-vt 0.2"))
                .row("Built with", "", text("Quadra, a native UI framework on Direct3D 11 and DirectWrite"))
                .row("License", "", text(env!("CARGO_PKG_LICENSE"))),
        ],
    ]
}

/// Which spaces are logged from their start, where, and how a file there is written.
fn logging(p: Prefs, d: Prefs) -> SettingsGroup {
    SettingsGroup::new("Logging")
        .row("Log local shells", "Each new local shell writes what it shows to a file.", switch(p.log_local))
        .back(p.log_local, d.log_local)
        .row("Log SSH spaces", "Each new SSH space does.", switch(p.log_ssh))
        .back(p.log_ssh, d.log_ssh)
        .row(
            "Folder",
            "Where logs go; empty, session-logs in 1Term's data folder.",
            folder_field(p.log_folder),
        )
        .back(p.log_folder, d.log_folder)
        .row("File name", "%n is the shell or the host.", text("%n_%Y-%m-%d_%H-%M-%S.log"))
        .row("A line", "%msg is a line of what the space showed.", text("[%Y-%m-%d %H:%M:%S] %msg"))
        .row(
            "A file there",
            "Written after what it holds, or over it.",
            segmented(crate::logs::MODES, p.log_mode),
        )
        .back(p.log_mode, d.log_mode)
}

/// A folder typed, or chosen in the system's dialog.
fn folder_field(folder: quadra::Signal<String>) -> quadra::Div {
    let dialogs = quadra::use_context::<std::rc::Rc<dyn quadra_shell::Dialogs>>();
    let choose = button("Choose\u{2026}").variant(Variant::Secondary).on_click(move || {
        let asked = quadra_shell::FileDialog::new("The folder of logs");
        if let Some(chosen) = dialogs.as_ref().and_then(|it| it.pick_folder(&asked)) {
            folder.set(chosen.display().to_string());
        }
    });
    row().gap(8.0).child(input(folder).placeholder("session-logs").label("Folder").width(240.0)).child(choose)
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

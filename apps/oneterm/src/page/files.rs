//! The settings' files (specs/19, "Settings"): the settings file and its folder on the
//! Advanced page, and the logging of spaces, a group of the Terminal page.

use quadra::{row, text};
use quadra_design::{SettingsGroup, Variant, button, input, segmented, switch, token};

use super::Back;
use crate::prefs::Prefs;

/// The settings file, where this run saves it (a trial run's folder too), and its folder
/// shown in the file manager.
pub(super) fn files() -> SettingsGroup {
    let dirs = quadra::use_context::<std::rc::Rc<quadra::AppDirs>>().map(|it| (*it).clone());
    let file = dirs.or_else(|| quadra::AppDirs::for_app("1Term")).map(|it| it.config.join("settings.json"));
    let said = file.as_ref().map(|it| it.display().to_string()).unwrap_or_default();
    let opener = quadra::use_context::<std::rc::Rc<dyn quadra_shell::Opener>>();
    let show = button("Show in folder").variant(Variant::Secondary).on_click(move || {
        if let (Some(opener), Some(file)) = (&opener, &file) {
            opener.reveal(file);
        }
    });
    SettingsGroup::new("Files")
        .about("Saved as you change them; the file can be edited by hand while 1Term is closed.")
        .row("Settings file", &said, show)
}

/// Which spaces are logged from their start, where, and how a file there is written.
pub(super) fn logging(p: Prefs, d: Prefs) -> SettingsGroup {
    SettingsGroup::new("Logging")
        .row("Log local shells", "Write everything a new local shell shows to a file.", switch(p.log_local))
        .back(p.log_local, d.log_local)
        .row("Log SSH spaces", "The same for each new SSH space.", switch(p.log_ssh))
        .back(p.log_ssh, d.log_ssh)
        .row(
            "Folder",
            "Where logs are saved. Leave empty for session-logs in 1Term's data folder.",
            folder_field(p.log_folder),
        )
        .back(p.log_folder, d.log_folder)
        .row("File name", "%n is the shell's or the host's name.", text("%n_%Y-%m-%d_%H-%M-%S.log"))
        .row("Line format", "%msg is one line of what the space showed.", text("[%Y-%m-%d %H:%M:%S] %msg"))
        .row(
            "An existing file",
            "Add to a log that is already there, or replace it.",
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
    row().gap(token::SPACE_MD).child(input(folder).placeholder("session-logs").label("Folder")).child(choose)
}

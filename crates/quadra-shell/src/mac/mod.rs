//! The macOS shell (specs/16; specs/23, M3): AppKit's panels and alert, opening by the
//! system's `open`, one instance over a socket, an item in the menu bar, and what the
//! system's own libraries seal, hash, fetch and measure. What holds the thread until the
//! user answers is never run by a test: those are manual checks.

mod assoc;
pub mod console;
mod crash;
mod fetch;
mod instance;
mod process;
mod seal;
mod tray;
mod usage;

pub use crash::crash_reports;
pub use fetch::http_get;
pub use instance::single_instance;
pub use process::{shell_process, user_environment};
pub use seal::{derive_key, open, random_bytes, random_key, seal, sha256};
pub use usage::{cycles_per_second, private_memory, process_cycles, process_usage, window_shown};

use std::path::{Path, PathBuf};
use std::process::Command;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertSecondButtonReturn, NSAlertStyle, NSBeep, NSModalResponseOK,
    NSOpenPanel, NSSavePanel,
};
use objc2_foundation::{NSArray, NSString, NSURL};

use crate::{
    Answer, Dialogs, FileDialog, Message, MessageKind, Meter, Opener, Sound, Sounds, Tray, TrayEvent,
    TrayIcon, TrayImage, Usage,
};
use tray::MacTray;

/// The shell of macOS. Its dialogs are AppKit's, on the main thread, and hold it until they
/// are answered.
pub struct MacShell;

/// What a dialog asks, said to a panel (an open panel is a save panel).
fn prepare(panel: &NSSavePanel, of: &FileDialog, kinds: bool) {
    // A panel shows no title of its own since macOS 11: the words go above the files.
    panel.setMessage(Some(&NSString::from_str(&of.title)));
    if let Some(folder) = of.folder.as_deref().and_then(NSURL::from_file_path) {
        panel.setDirectoryURL(Some(&folder));
    }
    if let Some(name) = &of.start {
        panel.setNameFieldStringValue(&NSString::from_str(name));
    }
    // The kinds are a filter's extensions: `*.png;*.jpg`. One that takes any file (`*.*`)
    // leaves the panel open to all.
    let patterns = || of.filters.iter().flat_map(|(_, patterns)| patterns.split(';')).map(str::trim);
    let any = patterns().any(|pattern| pattern == "*" || pattern.ends_with(".*"));
    let named: Vec<Retained<NSString>> =
        patterns().filter_map(|pattern| pattern.strip_prefix("*.")).map(NSString::from_str).collect();
    if kinds && !any && !named.is_empty() {
        // ponytail: by extension, which AppKit calls deprecated; uniform type identifiers
        // (a framework more) when a kind is no extension.
        #[allow(deprecated)]
        panel.setAllowedFileTypes(Some(&NSArray::from_retained_slice(&named)));
    }
}

fn open_dialog(of: &FileDialog, folders: bool) -> Vec<PathBuf> {
    let Some(mtm) = MainThreadMarker::new() else { return Vec::new() };
    let panel = NSOpenPanel::openPanel(mtm);
    panel.setCanChooseFiles(!folders);
    panel.setCanChooseDirectories(folders);
    panel.setAllowsMultipleSelection(of.multiple);
    prepare(&panel, of, !folders);
    if panel.runModal() != NSModalResponseOK {
        return Vec::new();
    }
    panel.URLs().iter().filter_map(|url| quadra_native_mac::file_path(&url)).collect()
}

impl Dialogs for MacShell {
    fn open_files(&self, dialog: &FileDialog) -> Vec<PathBuf> {
        open_dialog(dialog, false)
    }

    fn save_file(&self, of: &FileDialog) -> Option<PathBuf> {
        let panel = NSSavePanel::savePanel(MainThreadMarker::new()?);
        prepare(&panel, of, true);
        if panel.runModal() != NSModalResponseOK {
            return None;
        }
        quadra_native_mac::file_path(&*panel.URL()?)
    }

    fn pick_folder(&self, dialog: &FileDialog) -> Option<PathBuf> {
        open_dialog(dialog, true).into_iter().next()
    }

    // ponytail: the buttons in English; in the app's language when the menu bar is.
    fn message(&self, message: &Message) -> Answer {
        let Some(mtm) = MainThreadMarker::new() else { return Answer::Cancel };
        let question = message.kind == MessageKind::Question;
        let alert = NSAlert::new(mtm);
        alert.setMessageText(&NSString::from_str(&message.title));
        alert.setInformativeText(&NSString::from_str(&message.text));
        alert.setAlertStyle(match message.kind {
            MessageKind::Error => NSAlertStyle::Critical,
            MessageKind::Warning => NSAlertStyle::Warning,
            MessageKind::Info | MessageKind::Question => NSAlertStyle::Informational,
        });
        let buttons: &[&str] = if question { &["Yes", "No", "Cancel"] } else { &["OK"] };
        for title in buttons {
            alert.addButtonWithTitle(&NSString::from_str(title));
        }
        let answer = alert.runModal();
        match question {
            false => Answer::Ok,
            true if answer == NSAlertFirstButtonReturn => Answer::Yes,
            true if answer == NSAlertSecondButtonReturn => Answer::No,
            true => Answer::Cancel,
        }
    }
}

/// Runs the system's `open`, which knows the user's apps: true when it could.
fn system_open(command: &mut Command) -> bool {
    command.status().is_ok_and(|status| status.success())
}

impl Opener for MacShell {
    fn open(&self, target: &str) -> bool {
        system_open(Command::new("/usr/bin/open").arg(target))
    }

    fn reveal(&self, path: &Path) -> bool {
        system_open(Command::new("/usr/bin/open").arg("-R").arg(path))
    }

    /// An app (a bundle, `Name.app`) is opened as the Finder opens it; any other program is
    /// run.
    fn open_with(&self, program: &Path, args: &[String], file: &Path) -> bool {
        if program.extension().is_some_and(|kind| kind == "app") {
            let mut open = Command::new("/usr/bin/open");
            return system_open(open.arg("-a").arg(program).arg(file).arg("--args").args(args));
        }
        Command::new(program).args(args).arg(file).spawn().is_ok()
    }
}

impl Sounds for MacShell {
    // ponytail: the one alert sound the user set, for every kind: macOS has no sound a kind.
    fn play(&self, _sound: Sound) {
        NSBeep();
    }
}

impl Meter for MacShell {
    fn usage(&self) -> Option<Usage> {
        process_usage()
    }
}

impl Tray for MacShell {
    fn add(
        &self,
        image: Option<&TrayImage>,
        tooltip: &str,
        menu: &[&str],
        events: quadra_async::Sender<TrayEvent>,
    ) -> Option<Box<dyn TrayIcon>> {
        Some(Box::new(MacTray::add(image, tooltip, menu, events)?))
    }
}

impl TrayIcon for MacTray {
    fn notify(&self, title: &str, text: &str) {
        MacTray::notify(self, title, text);
    }

    fn set_tooltip(&mut self, tooltip: &str) {
        MacTray::set_tooltip(self, tooltip);
    }
}

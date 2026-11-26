//! The macOS shell (specs/16; specs/23, M3): AppKit's panels and alert, opening by the
//! system's `open`, one instance over a socket. What shows something to the user is never
//! run by a test: those are manual checks.

mod instance;

pub use instance::single_instance;

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
    Answer, Associations, Dialogs, FileDialog, FileTypes, Message, MessageKind, Meter, Opener, Sound, Sounds,
    Tray, TrayEvent, TrayIcon, TrayImage, Usage,
};

/// The shell of macOS. Its dialogs are AppKit's, on the main thread, and hold it until they
/// are answered.
pub struct MacShell;

/// Crashes that are no panic: macOS writes a report of each itself, which the user finds in
/// Console (`~/Library/Logs/DiagnosticReports`). Nothing is written to `folder`.
pub fn crash_reports(_folder: &Path) {}

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
    // ponytail: not read yet (the task's info, by Mach); a view that shows it shows nothing.
    fn usage(&self) -> Option<Usage> {
        None
    }
}

impl Tray for MacShell {
    // ponytail: no item in the menu bar yet (`NSStatusItem`): refused, as the trait allows.
    fn add(
        &self,
        _image: Option<&TrayImage>,
        _tooltip: &str,
        _menu: &[&str],
        _events: quadra_async::Sender<TrayEvent>,
    ) -> Option<Box<dyn TrayIcon>> {
        None
    }
}

/// On macOS an app's kinds of file are written in its bundle (`Info.plist`), which the
/// system reads when the app is put in place: nothing is registered while it runs.
impl Associations for MacShell {
    fn register(&self, _types: &FileTypes) -> Result<(), String> {
        Err("on macOS an app's kinds of file are those its bundle names".to_string())
    }

    fn opener_of(&self, _extension: &str) -> Option<PathBuf> {
        None
    }

    fn choose_default(&self, _types: &FileTypes) -> bool {
        false
    }
}

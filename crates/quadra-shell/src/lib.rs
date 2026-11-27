//! What an app asks of the OS beyond windows (specs/16): dialogs, opening files and links,
//! the tray and notifications, one instance, reports of crashes that are no panic. Each
//! service is a trait with an implementation for each OS and a fake that tests drive
//! ([`FakeShell`]); views find the app's in their context. What is here outside an OS's
//! module is the same on every OS; an OS's module (`win`, `mac`) is built only for it.

mod fake;
mod files;
#[cfg(target_os = "macos")]
mod mac;
mod net;
#[cfg(any(windows, target_os = "macos"))]
mod secret;
#[cfg(windows)]
mod win;

use std::path::PathBuf;
use std::rc::Rc;

#[cfg(any(windows, target_os = "macos"))]
use quadra_app::App;
use quadra_async::{Inbox, Sender};
use quadra_reactive::expect;
#[cfg(any(windows, target_os = "macos"))]
use quadra_render::Renderer;

pub use fake::FakeShell;
pub use files::{Associations, FileTypes, RegValue, default_apps_page, registration, same_program};

/// What the app's process has used (specs/16, "Process usage"): processor time since it
/// started, its memory in bytes, its threads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    /// The processor time of all its threads.
    pub cpu: std::time::Duration,
    /// Since the process was created.
    pub uptime: std::time::Duration,
    /// The private working set: what the task manager shows as its memory.
    pub memory: u64,
    pub working_set: u64,
    pub peak_working_set: u64,
    /// The commit: memory the process alone may use, in RAM or not.
    pub private_bytes: u64,
    pub threads: u32,
}

/// A shell's process and what it runs (specs/16): its full path, and the command line of the
/// newest process it started, none while it runs nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShellProcess {
    pub path: String,
    pub running: Option<String>,
}

/// Reads what the app's process uses: the OS's ([`install`]) or a test's ([`FakeShell`]).
pub trait Meter {
    fn usage(&self) -> Option<Usage>;
}
#[cfg(target_os = "macos")]
use mac as os;
#[cfg(target_os = "macos")]
pub use mac::{MacShell, console};
pub use net::Network;
#[cfg(any(windows, target_os = "macos"))]
pub use os::{crash_reports, process_usage, shell_process, single_instance, user_environment};
#[cfg(any(windows, target_os = "macos"))]
pub use os::{cycles_per_second, private_memory, process_cycles, window_shown};
#[cfg(any(windows, target_os = "macos"))]
pub use os::{derive_key, http_get, open, random_bytes, random_key, seal, sha256};
#[cfg(any(windows, target_os = "macos"))]
pub use secret::{hash_password, verify_password};
#[cfg(windows)]
use win as os;
#[cfg(windows)]
pub use win::{WinShell, console};
pub use zeroize::{Zeroize, Zeroizing};

/// What a dialog of files asks: its title, the kinds of file it shows (a name and patterns,
/// `("Images", "*.png;*.jpg")`), whether several may be chosen, and where it starts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileDialog {
    pub title: String,
    pub filters: Vec<(String, String)>,
    pub multiple: bool,
    /// The folder it opens in, and the name a save starts with.
    pub folder: Option<PathBuf>,
    pub start: Option<String>,
}

impl FileDialog {
    pub fn new(title: &str) -> FileDialog {
        FileDialog { title: title.to_string(), ..FileDialog::default() }
    }

    pub fn filter(mut self, name: &str, patterns: &str) -> FileDialog {
        self.filters.push((name.to_string(), patterns.to_string()));
        self
    }

    pub fn multiple(mut self) -> FileDialog {
        self.multiple = true;
        self
    }

    pub fn folder(mut self, folder: impl Into<PathBuf>) -> FileDialog {
        self.folder = Some(folder.into());
        self
    }

    pub fn start(mut self, name: &str) -> FileDialog {
        self.start = Some(name.to_string());
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MessageKind {
    #[default]
    Info,
    Warning,
    Error,
    /// Yes, No and Cancel: "Save the changes?"
    Question,
}

/// A box of a message: `Answer::Ok` for one that is no question.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Message {
    pub title: String,
    pub text: String,
    pub kind: MessageKind,
}

impl Message {
    pub fn new(kind: MessageKind, title: &str, text: &str) -> Message {
        Message { title: title.to_string(), text: text.to_string(), kind }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Ok,
    Yes,
    No,
    Cancel,
}

/// The OS's dialogs. Each is modal over the app's active window and holds the UI thread until
/// it is answered; the app's windows keep their last frame meanwhile.
pub trait Dialogs {
    /// The files chosen; none when the user gave up.
    fn open_files(&self, dialog: &FileDialog) -> Vec<PathBuf>;
    fn save_file(&self, dialog: &FileDialog) -> Option<PathBuf>;
    fn pick_folder(&self, dialog: &FileDialog) -> Option<PathBuf>;
    fn message(&self, message: &Message) -> Answer;
}

/// Opening what is not the app's.
pub trait Opener {
    /// A file in the app the user chose for its kind, or a link in the browser. False when
    /// the OS could not.
    fn open(&self, target: &str) -> bool;
    /// The folder of a file in the file manager, the file selected.
    fn reveal(&self, path: &std::path::Path) -> bool;
    /// A program the user named, with `args` and then the file. False when it could not start.
    fn open_with(&self, program: &std::path::Path, args: &[String], file: &std::path::Path) -> bool;
}

/// One of the system's sounds: the one the user set for that kind of event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    Default,
    Info,
    Warning,
    Error,
    Question,
}

/// The system's sounds.
pub trait Sounds {
    /// Plays `sound` and returns at once.
    fn play(&self, sound: Sound);
}

/// What the icon in the tray tells the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrayEvent {
    Click,
    DoubleClick,
    /// An item of its menu was chosen: its index.
    Item(usize),
}

/// The picture of an icon in the tray: straight RGBA, `width` by `height` (16 or 32).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrayImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// The notification area. An app of the tray says `quit_on_last_window(false)` (specs/15).
pub trait Tray {
    /// Puts an icon there, the app's picture or the system's: its clicks and the items of its
    /// menu (right click) go to `events`. None when the OS refused it.
    fn add(
        &self,
        image: Option<&TrayImage>,
        tooltip: &str,
        menu: &[&str],
        events: Sender<TrayEvent>,
    ) -> Option<Box<dyn TrayIcon>>;
}

/// An icon in the tray. Dropping it takes it away; it is added again when the taskbar starts
/// again.
pub trait TrayIcon {
    /// A notification from the icon.
    fn notify(&self, title: &str, text: &str);
    fn set_tooltip(&mut self, tooltip: &str);
}

/// What a later start of the app asked: the folder it was started in, and its arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Launch {
    pub folder: PathBuf,
    pub args: Vec<String>,
}

pub enum Instance {
    /// This process is the app's first: later starts arrive here, on the UI thread
    /// (`.on_data(..)` in a view).
    First(Inbox<Launch>),
    /// Another process is: it was given this one's arguments, and this one ends.
    Second,
}

/// Gives the app's views the OS's dialogs, opener, tray, meter, sounds and file types
/// ([`dialogs`], [`opener`], [`tray`], `Rc<dyn Meter>`, `Rc<dyn Sounds>`,
/// `Rc<dyn Associations>`).
#[cfg(any(windows, target_os = "macos"))]
pub fn install<R: Renderer>(app: App<R>) -> App<R> {
    #[cfg(windows)]
    let shell = Rc::new(WinShell);
    #[cfg(target_os = "macos")]
    let shell = Rc::new(MacShell);
    let app = app.provide::<Rc<dyn Dialogs>>(shell.clone()).provide::<Rc<dyn Opener>>(shell.clone());
    let app = app.provide::<Rc<dyn Tray>>(shell.clone()).provide::<Rc<dyn Meter>>(shell.clone());
    app.provide::<Rc<dyn Sounds>>(shell.clone()).provide::<Rc<dyn Associations>>(shell)
}

/// The app's tray. Call it while a view is built.
#[track_caller]
pub fn tray() -> Rc<dyn Tray> {
    expect::<Rc<dyn Tray>>()
}

/// The app's dialogs. Call it while a view is built, and keep it for its handlers.
#[track_caller]
pub fn dialogs() -> Rc<dyn Dialogs> {
    expect::<Rc<dyn Dialogs>>()
}

/// The app's opener. Call it while a view is built, and keep it for its handlers.
#[track_caller]
pub fn opener() -> Rc<dyn Opener> {
    expect::<Rc<dyn Opener>>()
}

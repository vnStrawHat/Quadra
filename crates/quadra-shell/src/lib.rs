//! What an app asks of the OS beyond windows (specs/16): dialogs, opening files and links,
//! the tray and notifications, one instance, reports of crashes that are no panic. Each
//! service is a trait with a Windows implementation and a fake that tests drive
//! ([`FakeShell`]); views find the app's in their context.

mod crash;
mod fake;
mod instance;
mod tray;
mod win;

use std::path::PathBuf;
use std::rc::Rc;

use quadra_app::App;
use quadra_async::Sender;
use quadra_reactive::expect;
use quadra_render::Renderer;

pub use crash::crash_reports;
pub use fake::FakeShell;
pub use instance::{Instance, Launch, single_instance};
pub use win::WinShell;

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

/// An icon in the tray. Dropping it takes it away.
// ponytail: an icon is not added again when the taskbar restarts (TaskbarCreated).
pub trait TrayIcon {
    /// A notification from the icon.
    fn notify(&self, title: &str, text: &str);
    fn set_tooltip(&mut self, tooltip: &str);
}

/// Gives the app's views the OS's dialogs, opener and tray ([`dialogs`], [`opener`],
/// [`tray`]).
pub fn install<R: Renderer>(app: App<R>) -> App<R> {
    let shell = Rc::new(WinShell);
    let app = app.provide::<Rc<dyn Dialogs>>(shell.clone()).provide::<Rc<dyn Opener>>(shell.clone());
    app.provide::<Rc<dyn Tray>>(shell)
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

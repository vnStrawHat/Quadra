//! The shell of tests (specs/16, "Traits and fakes"): it answers what the test queued and
//! keeps what it was asked, and nothing reaches the OS.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use quadra_reactive::provide;

use quadra_async::Sender;

use crate::{Answer, Dialogs, FileDialog, Message, Opener, Tray, TrayEvent, TrayIcon, TrayImage};

#[derive(Default)]
pub struct FakeShell {
    files: RefCell<VecDeque<Vec<PathBuf>>>,
    answers: RefCell<VecDeque<Answer>>,
    asked: Rc<RefCell<Vec<String>>>,
    tray: RefCell<Option<Sender<TrayEvent>>>,
}

impl FakeShell {
    pub fn new() -> Rc<FakeShell> {
        Rc::new(FakeShell::default())
    }

    /// Gives the views mounted in the current scope this shell as their dialogs and opener.
    pub fn provide(self: &Rc<Self>) {
        provide::<Rc<dyn Dialogs>>(self.clone());
        provide::<Rc<dyn Opener>>(self.clone());
        provide::<Rc<dyn Tray>>(self.clone());
    }

    /// The user does something to the icon the app last put in the tray.
    pub fn click_tray(&self, event: TrayEvent) {
        if let Some(events) = &*self.tray.borrow() {
            events.push(event);
        }
    }

    /// What the next dialog of files answers: the files chosen; none when not queued.
    pub fn choose(&self, files: &[&str]) {
        self.files.borrow_mut().push_back(files.iter().map(PathBuf::from).collect());
    }

    /// What the next message answers; `Ok` or `Cancel` when not queued.
    pub fn answer(&self, answer: Answer) {
        self.answers.borrow_mut().push_back(answer);
    }

    /// What was asked so far, in order: `"message: Save?"`, `"open: https://…"`.
    pub fn asked(&self) -> Vec<String> {
        self.asked.borrow().clone()
    }

    fn ask(&self, what: &str, of: &str) {
        self.asked.borrow_mut().push(format!("{what}: {of}"));
    }

    fn chosen(&self) -> Vec<PathBuf> {
        self.files.borrow_mut().pop_front().unwrap_or_default()
    }
}

impl Dialogs for FakeShell {
    fn open_files(&self, dialog: &FileDialog) -> Vec<PathBuf> {
        self.ask("open_files", &dialog.title);
        self.chosen()
    }

    fn save_file(&self, dialog: &FileDialog) -> Option<PathBuf> {
        self.ask("save_file", &dialog.title);
        self.chosen().into_iter().next()
    }

    fn pick_folder(&self, dialog: &FileDialog) -> Option<PathBuf> {
        self.ask("pick_folder", &dialog.title);
        self.chosen().into_iter().next()
    }

    fn message(&self, message: &Message) -> Answer {
        self.ask("message", &message.title);
        let none = if message.kind == crate::MessageKind::Question { Answer::Cancel } else { Answer::Ok };
        self.answers.borrow_mut().pop_front().unwrap_or(none)
    }
}

impl Opener for FakeShell {
    fn open(&self, target: &str) -> bool {
        self.ask("open", target);
        true
    }

    fn reveal(&self, path: &Path) -> bool {
        self.ask("reveal", &path.display().to_string());
        true
    }

    fn open_with(&self, program: &Path, args: &[String], file: &Path) -> bool {
        let line = std::iter::once(program.display().to_string()).chain(args.iter().cloned());
        let line: Vec<String> = line.chain([file.display().to_string()]).collect();
        self.ask("open with", &line.join(" "));
        true
    }
}

struct FakeIcon(Rc<RefCell<Vec<String>>>);

impl TrayIcon for FakeIcon {
    fn notify(&self, title: &str, text: &str) {
        self.0.borrow_mut().push(format!("notify: {title}: {text}"));
    }

    fn set_tooltip(&mut self, tooltip: &str) {
        self.0.borrow_mut().push(format!("tooltip: {tooltip}"));
    }
}

impl Tray for FakeShell {
    fn add(
        &self,
        _image: Option<&TrayImage>,
        tooltip: &str,
        menu: &[&str],
        events: Sender<TrayEvent>,
    ) -> Option<Box<dyn TrayIcon>> {
        self.ask("tray", &format!("{tooltip} [{}]", menu.join(", ")));
        *self.tray.borrow_mut() = Some(events);
        Some(Box::new(FakeIcon(self.asked.clone())))
    }
}

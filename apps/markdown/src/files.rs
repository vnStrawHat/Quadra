//! Files (specs/21, "Files" and "Tabs"): new, open, save, save as, a file dropped, closing a
//! tab or the window, and the question before unsaved work is lost.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use quadra::{Action, use_context};
use quadra_design::Tone;
use quadra_shell::{Answer, Dialogs, FileDialog, Message, MessageKind};

use crate::model::Model;
use crate::tabs::Tabs;

macro_rules! actions {
    ($($name:ident $id:literal),*) => {$(
        #[derive(Clone, Copy, Debug)]
        pub struct $name;
        impl Action for $name {
            const NAME: &'static str = $id;
        }
    )*};
}

actions!(
    NewFile "markdown::New", OpenFile "markdown::Open", Save "markdown::Save", SaveAs "markdown::SaveAs",
    CloseTab "markdown::CloseTab", NextTab "markdown::NextTab", PreviousTab "markdown::PreviousTab"
);

/// The files the app reads, by their extensions.
pub(crate) fn readable(path: &Path) -> bool {
    let ext = path.extension().map(|it| it.to_string_lossy().to_lowercase());
    matches!(ext.as_deref(), Some("md" | "markdown" | "txt"))
}

/// What does the files' work: the tabs and the OS's dialogs, when the app has them.
#[derive(Clone)]
pub(crate) struct Files {
    pub tabs: Tabs,
    dialogs: Option<Rc<dyn Dialogs>>,
}

impl Files {
    /// Call it while the view is built.
    pub fn new(tabs: &Tabs) -> Files {
        Files { tabs: tabs.clone(), dialogs: use_context::<Rc<dyn Dialogs>>() }
    }

    fn m(&self) -> Model {
        self.tabs.current_untracked()
    }

    pub fn new_file(&self) {
        self.tabs.new_tab();
    }

    /// Asks for files, and opens each in a tab.
    pub fn open(&self) {
        let Some(dialogs) = self.dialogs.clone() else { return };
        let mut asked =
            FileDialog::new("Open").filter("Markdown", "*.md;*.markdown").filter("Text", "*.txt").multiple();
        if let Some(folder) = self.m().folder() {
            asked = asked.folder(folder);
        }
        for path in dialogs.open_files(&asked) {
            self.tabs.open(&path);
        }
    }

    /// Opens a file in a tab.
    pub fn open_path(&self, path: PathBuf) {
        self.tabs.open(&path);
    }

    /// Saves the active document.
    pub fn save(&self, then: impl FnOnce() + 'static) {
        self.save_of(&self.m(), then);
    }

    pub fn save_as(&self, then: impl FnOnce() + 'static) {
        self.save_as_of(&self.m(), then);
    }

    /// Saves to the file, or asks where for an untitled document; `then` runs once saved.
    fn save_of(&self, m: &Model, then: impl FnOnce() + 'static) {
        match m.path.get_untracked() {
            Some(path) => write(m, path, then),
            None => self.save_as_of(m, then),
        }
    }

    fn save_as_of(&self, m: &Model, then: impl FnOnce() + 'static) {
        let Some(dialogs) = self.dialogs.clone() else { return };
        let name = m.path.get_untracked().map_or_else(|| "Untitled.md".to_string(), |_| m.name());
        let mut asked = FileDialog::new("Save as").filter("Markdown", "*.md").start(&name);
        if let Some(folder) = m.folder() {
            asked = asked.folder(folder);
        }
        if let Some(path) = dialogs.save_file(&asked) {
            write(m, path, then);
        }
    }

    /// Closes a tab, asking first when it has unsaved work.
    pub fn close_tab(&self, id: &str) {
        let Some(m) = self.tabs.find(id) else { return };
        let (tabs, id) = (self.tabs.clone(), id.to_string());
        self.then_unsaved(&m, move || tabs.remove(&id));
    }

    /// Some tab has unsaved work.
    pub fn unsaved(&self) -> bool {
        self.tabs.all().iter().any(|m| m.changed.get_untracked())
    }

    /// Runs `then` once the unsaved work of every tab is saved or let go, asking for each in
    /// turn; Cancel on any runs nothing.
    pub fn then_all_unsaved(&self, then: impl FnOnce() + 'static) {
        let left: Vec<Model> = self.tabs.all().into_iter().filter(|m| m.changed.get_untracked()).collect();
        self.ask_each(left, Box::new(then));
    }

    fn ask_each(&self, mut left: Vec<Model>, then: Box<dyn FnOnce()>) {
        let Some(m) = left.pop() else { return then() };
        // The tab asked about shows.
        self.tabs.active.set(m.id.clone());
        let files = self.clone();
        self.then_unsaved(&m, move || files.ask_each(left, then));
    }

    /// Runs `then` once the unsaved work of `m` is saved or let go: at once without any; after
    /// "Save changes to <name>?" with it. Cancel runs nothing.
    fn then_unsaved(&self, m: &Model, then: impl FnOnce() + 'static) {
        if !m.changed.get_untracked() {
            return then();
        }
        let Some(dialogs) = self.dialogs.clone() else { return };
        let words = format!("Save changes to {}?", m.name());
        let message = Message::new(MessageKind::Question, "Markdown", &words);
        match dialogs.message(&message) {
            Answer::Yes => self.save_of(m, then),
            Answer::No => then(),
            Answer::Ok | Answer::Cancel => {}
        }
    }
}

/// Writes the text as the editor holds it beside `path`, then renames it over `path`.
fn write(m: &Model, path: PathBuf, then: impl FnOnce() + 'static) {
    let m = m.clone();
    m.clone().file_text(move |text| {
        let name = format!(".{}.saving", path.file_name().map(|it| it.to_string_lossy()).unwrap_or_default());
        let beside = path.with_file_name(name);
        let written = std::fs::write(&beside, &text).and_then(|_| std::fs::rename(&beside, &path));
        match written {
            Ok(()) => {
                m.saved_as(path, text);
                then();
            }
            Err(e) => {
                let _ = std::fs::remove_file(&beside);
                m.notices.toast(Tone::Danger, format!("Could not save {}: {e}", path.display()));
            }
        }
    });
}

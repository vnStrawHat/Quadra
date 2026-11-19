//! Files (specs/21, "Files"): new, open, save, save as, a file dropped, and the question
//! before unsaved work is lost.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use quadra::{Action, use_context};
use quadra_design::Tone;
use quadra_shell::{Answer, Dialogs, FileDialog, Message, MessageKind};

use crate::model::Model;

macro_rules! actions {
    ($($name:ident $id:literal),*) => {$(
        #[derive(Clone, Copy, Debug)]
        pub struct $name;
        impl Action for $name {
            const NAME: &'static str = $id;
        }
    )*};
}

actions!(NewFile "markdown::New", OpenFile "markdown::Open", Save "markdown::Save", SaveAs "markdown::SaveAs");

/// The files the app reads, by their extensions.
pub(crate) fn readable(path: &Path) -> bool {
    let ext = path.extension().map(|it| it.to_string_lossy().to_lowercase());
    matches!(ext.as_deref(), Some("md" | "markdown" | "txt"))
}

/// What does the files' work: the model and the OS's dialogs, when the app has them.
#[derive(Clone)]
pub(crate) struct Files {
    m: Model,
    dialogs: Option<Rc<dyn Dialogs>>,
}

impl Files {
    /// Call it while the view is built.
    pub fn new(m: &Model) -> Files {
        Files { m: m.clone(), dialogs: use_context::<Rc<dyn Dialogs>>() }
    }

    pub fn new_file(&self) {
        let m = self.m.clone();
        self.then_unsaved(move || m.show(None, String::new()));
    }

    /// Asks about unsaved work, then for a file, then opens it.
    pub fn open(&self) {
        let Some(dialogs) = self.dialogs.clone() else { return };
        let mut asked = FileDialog::new("Open").filter("Markdown", "*.md;*.markdown").filter("Text", "*.txt");
        if let Some(folder) = self.m.folder() {
            asked = asked.folder(folder);
        }
        let m = self.m.clone();
        self.then_unsaved(move || {
            if let Some(path) = dialogs.open_files(&asked).into_iter().next() {
                m.open(&path);
            }
        });
    }

    /// Opens a file, asking first when there is unsaved work.
    pub fn open_path(&self, path: PathBuf) {
        let m = self.m.clone();
        self.then_unsaved(move || m.open(&path));
    }

    /// Saves to the file, or asks where for an untitled document; `then` runs once saved.
    pub fn save(&self, then: impl FnOnce() + 'static) {
        match self.m.path.get_untracked() {
            Some(path) => self.write(path, then),
            None => self.save_as(then),
        }
    }

    pub fn save_as(&self, then: impl FnOnce() + 'static) {
        let Some(dialogs) = self.dialogs.clone() else { return };
        let name = self.m.path.get_untracked().map_or_else(|| "Untitled.md".to_string(), |_| self.m.name());
        let mut asked = FileDialog::new("Save as").filter("Markdown", "*.md").start(&name);
        if let Some(folder) = self.m.folder() {
            asked = asked.folder(folder);
        }
        if let Some(path) = dialogs.save_file(&asked) {
            self.write(path, then);
        }
    }

    /// Writes the text as the editor holds it beside `path`, then renames it over `path`.
    fn write(&self, path: PathBuf, then: impl FnOnce() + 'static) {
        let m = self.m.clone();
        m.clone().file_text(move |text| {
            let name =
                format!(".{}.saving", path.file_name().map(|it| it.to_string_lossy()).unwrap_or_default());
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

    /// There is unsaved work.
    pub fn unsaved(&self) -> bool {
        self.m.changed.get_untracked()
    }

    /// Runs `then` once unsaved work is saved or let go: at once without any; after "Save
    /// changes to <name>?" with it. Cancel runs nothing.
    pub fn then_unsaved(&self, then: impl FnOnce() + 'static) {
        if !self.m.changed.get_untracked() {
            return then();
        }
        let Some(dialogs) = self.dialogs.clone() else { return };
        let words = format!("Save changes to {}?", self.m.name());
        let message = Message::new(MessageKind::Question, "Markdown", &words);
        match dialogs.message(&message) {
            Answer::Yes => self.save(then),
            Answer::No => then(),
            Answer::Ok | Answer::Cancel => {}
        }
    }
}

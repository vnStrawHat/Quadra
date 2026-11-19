//! What the window holds (specs/21): the file, its text, the layout, and the editor's
//! document while it is not shown.

use std::path::{Path, PathBuf};

use quadra::{Signal, batch, signal};
use quadra_code::{EditorHandle, KeptDoc};
use quadra_design::{Notices, Tone, notices};
use quadra_md::Place;

use crate::sync::Driver;

/// The layouts, in the order of the bar's segmented control.
pub const RAW: usize = 0;
pub const SIDE_BY_SIDE: usize = 1;
pub const PREVIEW: usize = 2;

/// The window's state. Copies are the same one.
#[derive(Clone)]
pub struct Model {
    /// The file, `None` while untitled.
    pub path: Signal<Option<PathBuf>>,
    /// The text the preview shows: as read, then as edited.
    pub text: Signal<String>,
    /// The text as last read or saved: what the editor's marks and `changed` compare with.
    pub saved: Signal<String>,
    /// Counts the documents opened: the editor is built again for each.
    pub opened: Signal<u64>,
    pub layout: Signal<usize>,
    /// How much of the width the editor has, side by side.
    pub share: Signal<f32>,
    /// The text is not what was last read or saved.
    pub changed: Signal<bool>,
    /// The editor's document's version, after each change of its text.
    pub edited: Signal<u64>,
    /// The source line the preview brings into view (a link to a heading).
    pub reveal: Signal<Option<usize>>,
    /// Where the editor stands, as a fractional line, and where the preview is scrolled to:
    /// kept while a pane is not shown.
    pub pos: Signal<f64>,
    pub preview_y: Signal<f32>,
    /// Where the preview's blocks are, the editor's caret line, and the pane that drives.
    pub places: Signal<Vec<Place>>,
    pub caret: Signal<usize>,
    pub driver: Signal<Driver>,
    /// The line of the heading tinted in the preview, a moment after it was chosen.
    pub flash: Signal<Option<usize>>,
    /// The outline shows.
    pub outline: Signal<bool>,
    /// The editor while it shows, and its document while it does not.
    pub editor: EditorHandle,
    pub doc: KeptDoc,
    pub notices: Notices,
}

impl Model {
    /// An untitled, empty document in Preview.
    pub fn new() -> Model {
        Model {
            path: signal(None),
            text: signal(String::new()),
            saved: signal(String::new()),
            opened: signal(0),
            layout: signal(PREVIEW),
            share: signal(0.5),
            changed: signal(false),
            edited: signal(0),
            reveal: signal(None),
            pos: signal(0.0),
            preview_y: signal(0.0),
            places: signal(Vec::new()),
            caret: signal(0),
            driver: signal(Driver::Editor),
            flash: signal(None),
            outline: signal(true),
            editor: EditorHandle::default(),
            doc: KeptDoc::default(),
            notices: notices().above(0.0),
        }
    }

    /// Opens a file in Preview; one that cannot be read says why and leaves the document as
    /// it was.
    pub fn open(&self, path: &Path) {
        match std::fs::read_to_string(path) {
            // ponytail: a byte order mark is dropped, and not written back on a save.
            Ok(text) => self.show(
                Some(path.to_path_buf()),
                text.strip_prefix('\u{feff}').map_or(text.clone(), str::to_string),
            ),
            Err(e) => {
                let words = format!("Could not open {}: {e}", path.display());
                self.notices.toast(Tone::Danger, words);
            }
        }
    }

    /// Shows a document: its file and its text, in Preview, with a new editor.
    pub fn show(&self, path: Option<PathBuf>, text: String) {
        self.doc.borrow_mut().take();
        batch(|| {
            self.path.set(path);
            self.saved.set(text.clone());
            self.text.set(text);
            self.changed.set(false);
            self.layout.set(PREVIEW);
            self.pos.set(0.0);
            self.preview_y.set(0.0);
            self.reveal.set(None);
            self.flash.set(None);
            self.caret.set(0);
            self.driver.set(Driver::Editor);
            self.opened.update(|n| *n += 1);
        });
    }

    /// Calls `f` with the text as the file is to have it (its line ends as read): the
    /// editor's, its kept document's, or the text as read when it was never edited. The
    /// editor's is read in the tree's next pass.
    pub fn file_text(&self, f: impl FnOnce(String) + 'static) {
        if self.editor.is_mounted() {
            return self.editor.read(move |w| f(w.doc.buffer.file_text()));
        }
        let kept = self.doc.borrow().as_ref().map(|doc| doc.buffer.file_text());
        f(kept.unwrap_or_else(|| self.text.get_untracked()));
    }

    /// The text the editor holds now, its lines ended by `\n`, for the preview.
    pub fn refresh(&self) {
        let text = self.text;
        if self.editor.is_mounted() {
            return self.editor.read(move |w| text.set(w.doc.text()));
        }
        if let Some(now) = self.doc.borrow().as_ref().map(|doc| doc.text()) {
            text.set(now);
        }
    }

    /// The text was written to `path`: it is the file now, and nothing is unsaved.
    pub fn saved_as(&self, path: PathBuf, text: String) {
        let base = text.replace("\r\n", "\n");
        self.editor.read(move |w| w.set_base(&base));
        batch(|| {
            self.path.set(Some(path));
            self.saved.set(text);
            self.changed.set(false);
        });
    }

    /// The file's name, or `Untitled`.
    pub fn name(&self) -> String {
        let name = self.path.with(|path| path.as_ref().and_then(|it| it.file_name()).map(|it| it.to_owned()));
        name.map_or_else(|| "Untitled".to_string(), |it| it.to_string_lossy().into_owned())
    }

    /// The folder of the file, where its images are looked for.
    pub fn folder(&self) -> Option<PathBuf> {
        self.path.with(|path| path.as_ref().and_then(|it| it.parent()).map(Path::to_path_buf))
    }
}

impl Default for Model {
    fn default() -> Self {
        Model::new()
    }
}

//! The SFTP menus for the other button (specs/19, "M9 in detail"), shared by the dock and the
//! expanded view, and the dialogs they open: a new folder's name, a new name, a delete asked.

use std::rc::Rc;

use oneterm_ssh::{Entry, join};
use quadra::{Clipboard, Signal, View, column, div, signal, text, use_context};
use quadra_design::{MenuItem, Variant, dialog, field, icons, input};

use crate::model::Model;
use crate::panes::Local;
use crate::sftp::Dock;

/// What the dialogs of a dock ask about.
#[derive(Clone, Copy)]
pub(crate) struct Ask {
    pub folder: Signal<bool>,
    pub rename: Signal<bool>,
    pub delete: Signal<bool>,
    /// The entry renamed or deleted, and the name typed.
    pub entry: Signal<Option<Entry>>,
    pub name: Signal<String>,
}

impl Ask {
    pub fn new() -> Ask {
        let no = || signal(false);
        Ask { folder: no(), rename: no(), delete: no(), entry: signal(None), name: signal(String::new()) }
    }

    pub fn folder(&self) {
        self.name.set(String::new());
        self.folder.set(true);
    }
}

fn clipboard() -> Option<Rc<dyn Clipboard>> {
    use_context::<Rc<dyn Clipboard>>()
}

fn copies(path: String) -> impl Fn() + 'static {
    let clipboard = clipboard();
    move || {
        if let Some(clipboard) = &clipboard {
            clipboard.write_text(&path);
        }
    }
}

/// A remote entry's menu: open or download it, rename, copy its path, delete.
pub(crate) fn remote(model: &Model, d: Dock, entry: &Entry) -> impl Fn() -> Vec<MenuItem> + use<> {
    let (model, entry, copy) = (model.clone(), entry.clone(), clipboard());
    move || {
        let path = join(&d.dir.get_untracked(), &entry.name);
        let (opens, (a, b, c)) = (model.clone(), (entry.clone(), entry.clone(), entry.clone()));
        let first = match entry.dir {
            true => MenuItem::new("Open").icon(icons::FOLDER).on_select({
                let path = path.clone();
                move || d.load(&opens, Some(path.clone()))
            }),
            false => {
                MenuItem::new("Download").icon(icons::ARROW_DOWN).on_select(move || d.download(&opens, &a))
            }
        };
        let copy = copy.clone();
        vec![
            first,
            MenuItem::new("Rename\u{2026}").on_select(move || {
                d.ask.name.set(b.name.clone());
                d.ask.entry.set(Some(b.clone()));
                d.ask.rename.set(true);
            }),
            MenuItem::new("Copy path").icon(icons::COPY).on_select(move || {
                if let Some(clipboard) = &copy {
                    clipboard.write_text(&path);
                }
            }),
            MenuItem::separator(),
            MenuItem::new("Delete\u{2026}").danger().on_select(move || {
                d.ask.entry.set(Some(c.clone()));
                d.ask.delete.set(true);
            }),
        ]
    }
}

/// The menu of the remote list where no entry is: a new folder, the list again.
pub(crate) fn remote_area(model: &Model, d: Dock) -> impl Fn() -> Vec<MenuItem> + use<> {
    let model = model.clone();
    move || {
        let again = model.clone();
        vec![
            MenuItem::new("New folder\u{2026}").icon(icons::FOLDER).on_select(move || d.ask.folder()),
            MenuItem::new("Refresh").on_select(move || d.refresh(&again)),
        ]
    }
}

/// A local entry's menu in the expanded view: open or upload it, copy its path.
pub(crate) fn local(model: &Model, d: Dock, entry: &Entry) -> impl Fn() -> Vec<MenuItem> + use<> {
    let (model, entry, local): (Model, Entry, Local) = (model.clone(), entry.clone(), d.local);
    move || {
        let path = local.dir.get_untracked().join(&entry.name);
        let (sends, at) = (model.clone(), path.clone());
        let first = match entry.dir {
            true => MenuItem::new("Open").icon(icons::FOLDER).on_select(move || local.load(at.clone())),
            false => MenuItem::new("Upload")
                .icon(icons::ARROW_UP)
                .on_select(move || d.upload(&sends, vec![at.clone()])),
        };
        let copy = copies(path.display().to_string());
        vec![
            first,
            MenuItem::new("Copy path").icon(icons::COPY).on_select(copy),
            MenuItem::new("Refresh").on_select(move || local.refresh()),
        ]
    }
}

/// The dialogs the menus open.
pub(crate) fn dialogs(model: &Model, d: Dock) -> impl View + use<> {
    let ask = d.ask;
    let named = move || {
        let name = ask.name.get_untracked();
        let name = name.trim();
        (!name.is_empty() && name != "." && name != ".." && !name.contains('/')).then(|| name.to_string())
    };
    let body = move || {
        column()
            .width(320.0)
            .padding_x(12.0)
            .child(field("Name", input(ask.name).width(296.0).test_id("sftp-name")))
    };
    let (makes, renames, deletes) = (model.clone(), model.clone(), model.clone());
    let folder = dialog(ask.folder, "New folder", body).action("Cancel", Variant::Secondary, || {}).action(
        "Create",
        Variant::Primary,
        move || {
            if let Some(name) = named() {
                d.mkdir(&makes, &name);
            }
        },
    );
    let rename = dialog(ask.rename, "Rename", body).action("Cancel", Variant::Secondary, || {}).action(
        "Rename",
        Variant::Primary,
        move || {
            if let (Some(name), Some(entry)) = (named(), ask.entry.get_untracked()) {
                d.rename(&renames, &entry.name, &name);
            }
        },
    );
    let asked = move || {
        let entry = ask.entry.get();
        let name = entry.as_ref().map(|it| it.name.as_str()).unwrap_or_default();
        let what = if entry.as_ref().is_some_and(|it| it.dir) { "the empty folder" } else { "the file" };
        format!("Delete {what} {name} from the server? It cannot be undone.")
    };
    let delete = dialog(ask.delete, "Delete", move || div().width(320.0).padding_x(12.0).child(text(asked)))
        .action("Cancel", Variant::Secondary, || {})
        .action("Delete", Variant::Primary, move || {
            if let Some(entry) = ask.entry.get_untracked() {
                d.delete(&deletes, &entry);
            }
        });
    div().child(folder).child(rename).child(delete)
}

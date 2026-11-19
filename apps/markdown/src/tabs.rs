//! Tabs (specs/21, "Tabs"): the window's documents, one showing; where a file opens, and the
//! strip's names and marks as the documents change.

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use quadra::{Signal, effect, signal, untrack};
use quadra_design::work::Tab;

use crate::model::Model;

/// The window's documents. Copies are the same tabs.
#[derive(Clone)]
pub struct Tabs {
    docs: Rc<RefCell<Vec<Model>>>,
    /// Counts the documents added and taken away: what the strip's names follow.
    count: Signal<u64>,
    /// The strip's tabs in their order, and the id of the one showing.
    pub strip: Signal<Vec<Tab>>,
    pub active: Signal<String>,
}

impl Tabs {
    /// The tabs of one document. Call it where the window's view is built.
    pub fn new(first: Model) -> Tabs {
        let tabs = Tabs {
            strip: signal(vec![Tab::new(first.id.clone(), first.name())]),
            active: signal(first.id.clone()),
            docs: Rc::new(RefCell::new(vec![first])),
            count: signal(0),
        };
        // Each tab is its file's name, marked while it has unsaved work.
        let (docs, count, strip) = (tabs.docs.clone(), tabs.count, tabs.strip);
        effect(move || {
            count.get();
            let now: Vec<(String, String, bool)> =
                docs.borrow().iter().map(|m| (m.id.clone(), m.name(), m.changed.get())).collect();
            let old = untrack(|| strip.get());
            let new: Vec<Tab> = old
                .iter()
                .filter_map(|tab| now.iter().find(|(id, ..)| *id == tab.id))
                .map(|(id, name, dirty)| Tab { dirty: *dirty, ..Tab::new(id.clone(), name.clone()) })
                .collect();
            if new != old {
                strip.set(new);
            }
        });
        tabs
    }

    /// The document showing: a read that what it is in runs again for.
    pub fn current(&self) -> Model {
        let id = self.active.get();
        self.find(&id).unwrap_or_else(|| self.docs.borrow()[0].clone())
    }

    pub fn current_untracked(&self) -> Model {
        untrack(|| self.current())
    }

    pub fn find(&self, id: &str) -> Option<Model> {
        self.docs.borrow().iter().find(|m| m.id == id).cloned()
    }

    /// All the documents, in no order.
    pub fn all(&self) -> Vec<Model> {
        self.docs.borrow().clone()
    }

    /// The tabs' ids in the strip's order.
    pub fn order(&self) -> Vec<String> {
        self.strip.with_untracked(|tabs| tabs.iter().map(|tab| tab.id.clone()).collect())
    }

    /// Adds a document after the active tab, and shows it.
    pub fn add(&self, m: Model) {
        let id = m.id.clone();
        let at = self.order().iter().position(|it| *it == self.active.get_untracked()).map_or(0, |at| at + 1);
        self.strip.update(|tabs| tabs.insert(at.min(tabs.len()), Tab::new(id.clone(), m.name())));
        self.docs.borrow_mut().push(m);
        self.count.update(|n| *n += 1);
        self.active.set(id);
    }

    /// A new untitled document, sharing the window's settings and notices.
    pub fn fresh(&self) -> Model {
        let first = self.docs.borrow()[0].clone();
        Model::with(first.prefs, first.notices)
    }

    /// A new untitled tab.
    pub fn new_tab(&self) {
        self.add(self.fresh());
    }

    /// Opens a file: in the tab that has it, else in the active tab when it is untitled,
    /// empty and unchanged, else in a new tab. One that cannot be read says why and adds none.
    pub fn open(&self, path: &Path) {
        let same =
            |m: &Model| m.path.with_untracked(|it| it.as_deref().is_some_and(|it| same_file(it, path)));
        if let Some(there) = self.docs.borrow().iter().find(|m| same(m)) {
            self.active.set(there.id.clone());
            return;
        }
        let now = self.current_untracked();
        let blank = now.path.get_untracked().is_none()
            && !now.changed.get_untracked()
            && now.text.with_untracked(String::is_empty);
        if blank {
            return now.open(path);
        }
        let m = self.fresh();
        m.open(path);
        if m.path.get_untracked().is_some() {
            self.add(m);
        }
    }

    /// Takes a tab away; the last leaves an untitled one. The tab next to it shows.
    pub fn remove(&self, id: &str) {
        let order = self.order();
        let Some(at) = order.iter().position(|it| it == id) else { return };
        if order.len() == 1 {
            self.new_tab();
        }
        self.strip.update(|tabs| tabs.retain(|tab| tab.id != id));
        self.docs.borrow_mut().retain(|m| m.id != id);
        self.count.update(|n| *n += 1);
        if self.active.get_untracked() == id {
            let order = self.order();
            self.active.set(order[at.min(order.len() - 1)].clone());
        }
    }

    /// The tab `by` places after the active one, round.
    pub fn step(&self, by: isize) {
        let order = self.order();
        let at = order.iter().position(|it| *it == self.active.get_untracked()).unwrap_or(0);
        let next = (at as isize + by).rem_euclid(order.len() as isize) as usize;
        self.active.set(order[next].clone());
    }
}

/// The same file, whatever the case of its letters or the form of its path.
fn same_file(a: &Path, b: &Path) -> bool {
    let plain = |it: &Path| std::fs::canonicalize(it).unwrap_or_else(|_| it.to_path_buf());
    plain(a).to_string_lossy().to_lowercase() == plain(b).to_string_lossy().to_lowercase()
}

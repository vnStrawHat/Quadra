//! What the SFTP dock shows and does (specs/19, "M9 in detail"): the folder of the focused
//! space's connection, its entries, the transfers. Every call of the engine runs on the pool;
//! its answer comes back to the UI thread.

use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use oneterm_ssh::{Entry, Files, Ssh, join, safe_name};
use quadra::{Scope, Signal, background, interval, signal, spawn, untrack};

use crate::model::Model;
use crate::panes::{Local, free_name};

/// A file going one way or the other.
/// What the dock says of a space that is not remote.
pub(crate) const LOCAL: &str =
    "The active space is a local shell. Select an SSH space to browse its files here.";

pub(crate) struct Transfer {
    pub name: String,
    /// To the server.
    pub up: bool,
    /// The folder it goes to.
    pub to: String,
    pub done: Arc<AtomicU64>,
    pub whole: Arc<AtomicU64>,
    pub cancel: Arc<AtomicBool>,
    /// None while it runs; then how it ended.
    pub end: Signal<Option<Result<(), String>>>,
}

impl Transfer {
    /// `name → remote 48%`, `name ← local`, or why it failed.
    pub fn said(&self) -> String {
        let arrow = if self.up { "\u{2192} remote" } else { "\u{2190} local" };
        match self.end.get() {
            Some(Ok(())) => format!("{} {arrow} done", self.name),
            Some(Err(why)) => format!("{}: {why}", self.name),
            None => {
                let (done, whole) = (self.done.load(Ordering::Relaxed), self.whole.load(Ordering::Relaxed));
                format!("{} {arrow} {}%", self.name, (done * 100).checked_div(whole).unwrap_or(0))
            }
        }
    }

    /// How much is done, from 0 to 1.
    pub fn part(&self) -> f32 {
        let (done, whole) = (self.done.load(Ordering::Relaxed), self.whole.load(Ordering::Relaxed));
        match self.end.get() {
            Some(Ok(())) => 1.0,
            _ if whole == 0 => 0.0,
            _ => done as f32 / whole as f32,
        }
    }

    /// `48%`, `done` or `failed`.
    pub fn state(&self) -> String {
        match self.end.get() {
            Some(Ok(())) => "done".into(),
            Some(Err(_)) => "failed".into(),
            None => format!("{:.0}%", self.part() * 100.0),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Dock {
    pub dir: Signal<String>,
    pub entries: Signal<Vec<Entry>>,
    /// What stands for the list: why it is empty, or what failed.
    pub said: Signal<String>,
    pub transfers: Signal<Vec<Rc<Transfer>>>,
    /// Moves while transfers run, so their percentages show.
    pub tick: Signal<u32>,
    /// The remote entry chosen in the expanded view.
    pub selected: Signal<Option<String>>,
    /// Whether names that start with `.` show.
    pub hidden: Signal<bool>,
    /// The expanded view, whose Local pane downloads go to.
    pub expanded: Signal<bool>,
    pub local: Local,
    owner: Scope,
}

/// The focused space's connection, if it is remote.
pub(crate) fn connection(model: &Model) -> Option<Arc<Ssh>> {
    let space = untrack(|| model.active_tab().and_then(|tab| tab.focused()))?;
    space.session.kept::<Arc<Ssh>>().map(|it| (*it).clone())
}

impl Dock {
    /// The dock's state; while it is open and the connection is not up yet, it asks again
    /// every second.
    pub fn new(model: &Model, open: Signal<bool>) -> Dock {
        let dock = Dock {
            dir: signal(String::new()),
            entries: signal(Vec::new()),
            said: signal(String::new()),
            transfers: signal(Vec::new()),
            tick: signal(0),
            selected: signal(None),
            hidden: signal(false),
            expanded: model.ui().expanded,
            local: Local::new(downloads(model)),
            owner: Scope::current(),
        };
        let (model, waits) = (model.clone(), std::cell::Cell::new(0u32));
        let _ = interval(Duration::from_millis(250), move || {
            if dock.transfers.with_untracked(|all| all.iter().any(|it| it.end.get_untracked().is_none())) {
                dock.tick.update(|n| *n += 1);
            }
            let early = dock.said.with_untracked(|it| it == oneterm_ssh::NOT_YET);
            if open.get_untracked() && early && waits.replace((waits.get() + 1) % 4) == 3 {
                dock.load(&model, None);
            }
        });
        dock
    }

    /// Runs `work` with the connection's files on the pool, then `then` with its answer.
    fn with_files<T: Send + 'static>(
        &self,
        model: &Model,
        work: impl FnOnce(Files) -> Result<T, String> + Send + 'static,
        then: impl FnOnce(Dock, Result<T, String>) + 'static,
    ) {
        let Some(ssh) = connection(model) else {
            self.said.set(LOCAL.into());
            self.entries.set(Vec::new());
            return;
        };
        let dock = *self;
        self.owner.run(|| {
            spawn(async move {
                let got = background(move || work(ssh.files()?)).await;
                then(dock, got.unwrap_or_else(|_| Err("the work failed".into())));
            })
            .detach()
        });
    }

    /// Shows a folder, the home folder when none is given.
    pub fn load(&self, model: &Model, dir: Option<String>) {
        self.with_files(
            model,
            move |files| {
                let dir = match dir {
                    Some(dir) => dir,
                    None => files.home()?,
                };
                let entries = files.list(&dir)?;
                Ok((dir, entries))
            },
            |dock, got| match got {
                Ok((dir, entries)) => {
                    dock.selected.set(None);
                    dock.dir.set(dir);
                    dock.entries.set(entries);
                    dock.said.set(String::new());
                }
                Err(why) => dock.said.set(why),
            },
        );
    }

    /// The folder shown, again.
    pub fn refresh(&self, model: &Model) {
        let dir = self.dir.get_untracked();
        self.load(model, (!dir.is_empty()).then_some(dir));
    }

    pub fn mkdir(&self, model: &Model, name: &str) {
        let (path, again) = (join(&self.dir.get_untracked(), name), model.clone());
        self.with_files(model, move |files| files.mkdir(&path), move |dock, got| dock.after(&again, got));
    }

    pub fn delete(&self, model: &Model, entry: &Entry) {
        let (path, dir, again) = (join(&self.dir.get_untracked(), &entry.name), entry.dir, model.clone());
        self.with_files(
            model,
            move |files| files.remove(&path, dir),
            move |dock, got| dock.after(&again, got),
        );
    }

    fn after(&self, model: &Model, got: Result<(), String>) {
        match got {
            Ok(()) => self.refresh(model),
            Err(why) => self.said.set(why),
        }
    }

    /// Downloads a file under a name not taken: to the Local pane's folder in the expanded
    /// view, else to the downloads folder.
    pub fn download(&self, model: &Model, entry: &Entry) {
        if !safe_name(&entry.name) {
            self.said.set(format!("Not a name for a local file: {}", entry.name));
            return;
        }
        let remote = join(&self.dir.get_untracked(), &entry.name);
        let folder =
            if self.expanded.get_untracked() { self.local.dir.get_untracked() } else { downloads(model) };
        let local = free_name(&folder, &entry.name);
        let transfer = self.start(&entry.name, false, folder.display().to_string());
        let (done, whole, cancel) = (transfer.done.clone(), transfer.whole.clone(), transfer.cancel.clone());
        self.with_files(
            model,
            move |files| files.download(&remote, &local, |d, w| store(&done, &whole, d, w), &cancel),
            move |dock, got| {
                if got.is_ok() && dock.local.dir.get_untracked() == folder {
                    dock.local.refresh();
                }
                transfer.end.set(Some(got));
            },
        );
    }

    /// Uploads local files to the folder shown.
    pub fn upload(&self, model: &Model, paths: Vec<PathBuf>) {
        for path in paths.into_iter().filter(|it| it.is_file()) {
            let name = path.file_name().map(|it| it.to_string_lossy().into_owned()).unwrap_or_default();
            let remote = join(&self.dir.get_untracked(), &name);
            let transfer = self.start(&name, true, self.dir.get_untracked());
            let (done, whole, cancel) =
                (transfer.done.clone(), transfer.whole.clone(), transfer.cancel.clone());
            let again = model.clone();
            self.with_files(
                model,
                move |files| files.upload(&path, &remote, |d, w| store(&done, &whole, d, w), &cancel),
                move |dock, got| {
                    if got.is_ok() {
                        dock.refresh(&again);
                    }
                    transfer.end.set(Some(got));
                },
            );
        }
    }

    fn start(&self, name: &str, up: bool, to: String) -> Rc<Transfer> {
        let transfer = Rc::new(Transfer {
            name: name.to_string(),
            up,
            to,
            done: Arc::default(),
            whole: Arc::default(),
            cancel: Arc::default(),
            end: signal(None),
        });
        self.transfers.update(|all| all.push(transfer.clone()));
        transfer
    }
}

fn store(done: &AtomicU64, whole: &AtomicU64, d: u64, w: u64) {
    done.store(d, Ordering::Relaxed);
    whole.store(w, Ordering::Relaxed);
}

/// Where downloads go: the setting, else the user's Downloads folder.
fn downloads(model: &Model) -> PathBuf {
    let set = model.prefs().downloads.get_untracked();
    if !set.is_empty() {
        return PathBuf::from(set);
    }
    let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default();
    home.join("Downloads")
}

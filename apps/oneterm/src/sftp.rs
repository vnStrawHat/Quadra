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

/// A file going one way or the other.
pub(crate) struct Transfer {
    pub name: String,
    /// To the server.
    pub up: bool,
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
            self.said.set("Not an SSH space".into());
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

    /// Downloads a file to the user's Downloads folder, under a name not taken.
    pub fn download(&self, model: &Model, entry: &Entry) {
        if !safe_name(&entry.name) {
            self.said.set(format!("Not a name for a local file: {}", entry.name));
            return;
        }
        let remote = join(&self.dir.get_untracked(), &entry.name);
        let local = free_name(&downloads(model), &entry.name);
        let transfer = self.start(&entry.name, false);
        let (done, whole, cancel) = (transfer.done.clone(), transfer.whole.clone(), transfer.cancel.clone());
        self.with_files(
            model,
            move |files| files.download(&remote, &local, |d, w| store(&done, &whole, d, w), &cancel),
            move |_, got| transfer.end.set(Some(got)),
        );
    }

    /// Uploads local files to the folder shown.
    pub fn upload(&self, model: &Model, paths: Vec<PathBuf>) {
        for path in paths.into_iter().filter(|it| it.is_file()) {
            let name = path.file_name().map(|it| it.to_string_lossy().into_owned()).unwrap_or_default();
            let remote = join(&self.dir.get_untracked(), &name);
            let transfer = self.start(&name, true);
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

    fn start(&self, name: &str, up: bool) -> Rc<Transfer> {
        let transfer = Rc::new(Transfer {
            name: name.to_string(),
            up,
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

/// `name`, or `name (1).ext` and on when that is taken.
pub(crate) fn free_name(folder: &std::path::Path, name: &str) -> PathBuf {
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem, format!(".{ext}")),
        _ => (name, String::new()),
    };
    let mut path = folder.join(name);
    let mut n = 1;
    while path.exists() || path.with_file_name(format!("{}.part", file(&path))).exists() {
        path = folder.join(format!("{stem} ({n}){ext}"));
        n += 1;
    }
    path
}

fn file(path: &std::path::Path) -> String {
    path.file_name().map(|it| it.to_string_lossy().into_owned()).unwrap_or_default()
}

/// A size as people read it: `812 B`, `2.4 KB`, `41 MB`.
pub(crate) fn size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let (mut value, mut unit) = (bytes as f64 / 1024.0, 0);
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        (value, unit) = (value / 1024.0, unit + 1);
    }
    if value < 10.0 { format!("{value:.1} {}", UNITS[unit]) } else { format!("{value:.0} {}", UNITS[unit]) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_and_free_names() {
        assert_eq!(
            (size(812), size(2458), size(41 * 1024 * 1024)),
            ("812 B".into(), "2.4 KB".into(), "41 MB".into())
        );
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/free-name");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(free_name(&dir, "a.txt"), dir.join("a.txt"));
        std::fs::write(dir.join("a.txt"), b"").unwrap();
        assert_eq!(free_name(&dir, "a.txt"), dir.join("a (1).txt"));
        assert_eq!(free_name(&dir, ".bashrc"), dir.join(".bashrc"));
    }
}

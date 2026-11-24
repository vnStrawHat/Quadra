//! Settings (specs/15, "Settings"): one JSON object in one file, each key a signal. A key is
//! a path of names with dots between, `editor.font_size`, and the file holds objects in
//! objects: `size` in `editor`. Changes are written half a second after the last one, through
//! a file beside it that is renamed over it.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::time::Duration;

use quadra_async::{Timer, Timers, timeout};
use quadra_reactive::{Scope, Signal, effect, signal, untrack, use_context};
use serde_json::{Map, Value};

use crate::log;

mod stored;
pub use stored::Stored;

/// How long after the last change the file is written.
const QUIET: Duration = Duration::from_millis(500);

/// The value at `key` in `map`: `a.b` is `b` in the object `a`.
fn at<'a>(map: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    match key.split_once('.') {
        None => map.get(key),
        Some((first, rest)) => at(map.get(first)?.as_object()?, rest),
    }
}

/// Puts `value` at `key`, making the objects on the way there. What stood on the way and is
/// no object gives way: a key is not also the start of another.
fn place(map: &mut Map<String, Value>, key: &str, value: Value) {
    let Some((first, rest)) = key.split_once('.') else {
        map.insert(key.to_string(), value);
        return;
    };
    let inner = map.entry(first).or_insert(Value::Null);
    if !inner.is_object() {
        *inner = Value::Object(Map::new());
    }
    if let Value::Object(inner) = inner {
        place(inner, rest, value);
    }
}

/// A file of before named each setting whole at the top (`"editor.font_size": 15`): those
/// are put where they belong, after what is nested already.
fn nested(map: Map<String, Value>) -> Map<String, Value> {
    let (whole, mut out): (Map<_, _>, Map<_, _>) = map.into_iter().partition(|(key, _)| key.contains('.'));
    whole.into_iter().for_each(|(key, value)| place(&mut out, &key, value));
    out
}

pub struct Settings {
    /// None: kept in memory only.
    path: Option<PathBuf>,
    /// Objects in objects, as the file is.
    map: RefCell<Map<String, Value>>,
    /// The signal of each key asked for: two views of one key share it.
    signals: RefCell<HashMap<String, Box<dyn Any>>>,
    /// False for a file of a newer shape than the app knows: it is read, never written.
    writable: Cell<bool>,
    pending: RefCell<Option<Timer>>,
    /// Where the signals and the timers live: where the settings were opened.
    scope: Scope,
    me: Weak<Settings>,
}

impl Settings {
    /// Settings that are never written.
    pub fn memory() -> Rc<Settings> {
        Settings::new(None, Map::new())
    }

    /// The settings of a file; the defaults when there is none. A file that is no JSON object
    /// is moved aside to `<name>.corrupt`, and the log says so.
    pub fn open(path: impl Into<PathBuf>) -> Rc<Settings> {
        let path = path.into();
        let map = match fs::read_to_string(&path) {
            Err(_) => Map::new(),
            Ok(text) => match serde_json::from_str::<Value>(&text) {
                Ok(Value::Object(map)) => nested(map),
                _ => {
                    let aside = with_extension(&path, "corrupt");
                    let _ = fs::rename(&path, &aside);
                    log::warn(
                        "settings",
                        format!("{} is not a JSON object: moved to {}", path.display(), aside.display()),
                    );
                    Map::new()
                }
            },
        };
        Settings::new(Some(path), map)
    }

    fn new(path: Option<PathBuf>, map: Map<String, Value>) -> Rc<Settings> {
        Rc::new_cyclic(|me| Settings {
            path,
            map: RefCell::new(map),
            signals: RefCell::default(),
            writable: Cell::new(true),
            pending: RefCell::default(),
            scope: Scope::current(),
            me: me.clone(),
        })
    }

    /// A setting that is one of `options`, kept as its words (the file reads as a settings
    /// page does) and given as its index: the two follow each other. Where none is written,
    /// and for words the app does not know, it is the first option.
    pub fn choice(&self, key: &str, options: &'static [&'static str]) -> Signal<usize> {
        self.choice_or(key, options, 0)
    }

    /// As [`Settings::choice`], the option at `default` where none is written.
    pub fn choice_or(&self, key: &str, options: &'static [&'static str], default: usize) -> Signal<usize> {
        let default = default.min(options.len() - 1);
        let words = self.value(key, options[default].to_string());
        let at = move || options.iter().position(|o| words.with(|w| o == w)).unwrap_or(default);
        self.scope.run(|| {
            let index = signal(untrack(at));
            effect(move || {
                let now = at();
                if untrack(|| index.get()) != now {
                    index.set(now);
                }
            });
            effect(move || {
                let at = index.get();
                if untrack(|| words.get()) != options[at] {
                    words.set(options[at].to_string());
                }
            });
            index
        })
    }

    /// The setting of `key`: its value in the file, or `default`. Writing the signal writes
    /// the setting.
    pub fn value<T: Stored>(&self, key: &str, default: T) -> Signal<T> {
        if let Some(known) = self.signals.borrow().get(key).and_then(|s| s.downcast_ref::<Signal<T>>()) {
            return *known;
        }
        let start = at(&self.map.borrow(), key).and_then(T::from_json).unwrap_or(default);
        let (me, name) = (self.me.clone(), key.to_string());
        let value = self.scope.run(|| {
            let (value, first) = (signal(start), Cell::new(true));
            effect(move || {
                let now = value.get();
                // The value it starts with is not a change: a default is not written.
                if !first.replace(false)
                    && let Some(me) = me.upgrade()
                {
                    me.put(&name, now.to_json());
                }
            });
            value
        });
        self.signals.borrow_mut().insert(key.to_string(), Box::new(value));
        value
    }

    fn put(&self, key: &str, value: Value) {
        if at(&self.map.borrow(), key) == Some(&value) {
            return;
        }
        place(&mut self.map.borrow_mut(), key, value);
        self.schedule();
    }

    /// Writes the file half a second after the last change; at once where nothing keeps
    /// time (a bare tree).
    fn schedule(&self) {
        if self.path.is_none() {
            return;
        }
        if let Some(timer) = self.pending.borrow_mut().take() {
            timer.cancel();
        }
        let me = self.me.clone();
        let timer = self.scope.run(|| {
            use_context::<Rc<Timers>>().map(|_| {
                timeout(QUIET, move || {
                    if let Some(me) = me.upgrade() {
                        me.save_now();
                    }
                })
            })
        });
        match timer {
            Some(timer) => *self.pending.borrow_mut() = Some(timer),
            None => self.save_now(),
        }
    }

    /// Writes the file now, if anything waits to be written: to a file beside it, which is
    /// then renamed over it, so the file is whole whenever it is read.
    pub fn save_now(&self) {
        if let Some(timer) = self.pending.borrow_mut().take() {
            timer.cancel();
        }
        let Some(path) = self.path.as_ref().filter(|_| self.writable.get()) else { return };
        let text = serde_json::to_string_pretty(&*self.map.borrow()).unwrap_or_default();
        let fresh = with_extension(path, "tmp");
        if let Some(folder) = path.parent() {
            let _ = fs::create_dir_all(folder);
        }
        let written = fs::write(&fresh, text + "\n").and_then(|()| fs::rename(&fresh, path));
        if let Err(e) = written {
            log::error("settings", format!("{} could not be written: {e}", path.display()));
        }
    }

    /// Brings a file of an older shape to `version`: `f` is given the version the file says
    /// (0 when it says none) and its object, as the file holds it (objects in objects). A
    /// file of a newer version is left as it is, and never written. Call it before asking for
    /// values.
    pub fn migrate(&self, version: u64, f: impl FnOnce(u64, &mut Map<String, Value>)) {
        let from = self.map.borrow().get("version").and_then(Value::as_u64).unwrap_or(0);
        if from > version {
            self.writable.set(false);
            log::warn(
                "settings",
                format!("the file is of version {from}, newer than {version}: it is not written"),
            );
            return;
        }
        if from < version {
            f(from, &mut self.map.borrow_mut());
            self.map.borrow_mut().insert("version".into(), Value::from(version));
            self.save_now();
        }
    }
}

fn with_extension(path: &Path, more: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".");
    name.push(more);
    PathBuf::from(name)
}

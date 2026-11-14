//! The log (specs/15, "Log and crashes"): lines of four levels, kept in a ring and, once the
//! app has folders, appended to a file a day; until then errors and warnings go to the
//! standard error stream. Any thread may write it.

use std::collections::VecDeque;
use std::fmt::Display;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use crate::time::{DateTime, unix_now};

/// How many lines the ring keeps, how large a day's file grows, how long files are kept.
const RING: usize = 500;
const DAY_FILE: u64 = 8 << 20;
const KEPT: Duration = Duration::from_secs(14 * 86_400);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
}

impl Level {
    fn word(self) -> &'static str {
        ["ERROR", "WARN", "INFO", "DEBUG"][self as usize]
    }
}

struct Log {
    ring: VecDeque<String>,
    folder: Option<PathBuf>,
}

static LOG: Mutex<Log> = Mutex::new(Log { ring: VecDeque::new(), folder: None });

fn with<T>(f: impl FnOnce(&mut Log) -> T) -> T {
    // A thread that panicked while it wrote leaves the log as it was: it goes on.
    f(&mut LOG.lock().unwrap_or_else(|poisoned| poisoned.into_inner()))
}

/// Writes a line: the time in UTC, the level, what wrote it and what it says.
pub fn write(level: Level, target: &str, message: impl Display) {
    let now = DateTime::from_unix(unix_now(), 0);
    let line = format!("{now}Z {} {target}: {message}", level.word());
    with(|log| {
        if log.ring.len() == RING {
            log.ring.pop_front();
        }
        log.ring.push_back(line.clone());
        let Some(folder) = &log.folder else {
            // With no file, what went wrong is not lost: it goes to the standard error stream.
            if level <= Level::Warn {
                eprintln!("{line}");
            }
            return;
        };
        let path = folder.join(format!("{}.log", now.date));
        if fs::metadata(&path).is_ok_and(|file| file.len() >= DAY_FILE) {
            return; // a day that wrote this much has said what it had to
        }
        // ponytail: opened a line at a time; a writer kept open if logging gets busy.
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(file, "{line}");
        }
    });
}

pub fn error(target: &str, message: impl Display) {
    write(Level::Error, target, message);
}

pub fn warn(target: &str, message: impl Display) {
    write(Level::Warn, target, message);
}

pub fn info(target: &str, message: impl Display) {
    write(Level::Info, target, message);
}

pub fn debug(target: &str, message: impl Display) {
    write(Level::Debug, target, message);
}

/// The last `n` lines, the oldest first.
pub fn recent(n: usize) -> Vec<String> {
    with(|log| log.ring.iter().skip(log.ring.len().saturating_sub(n)).cloned().collect())
}

/// Appends the lines from now on to a file a day in `folder`, and deletes the files there
/// older than two weeks.
pub fn to_folder(folder: &Path) {
    let _ = fs::create_dir_all(folder);
    for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
        let old = entry.metadata().and_then(|m| m.modified()).ok().and_then(|at| at.elapsed().ok());
        let is_log = entry.path().extension().is_some_and(|ext| ext == "log");
        if is_log && old.is_some_and(|old| old > KEPT) {
            let _ = fs::remove_file(entry.path());
        }
    }
    with(|log| log.folder = Some(folder.to_path_buf()));
}

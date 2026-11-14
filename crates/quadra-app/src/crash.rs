//! Crash reports of panics (specs/15, "Log and crashes"): what panicked, where, the
//! backtrace and the end of the log, in a file of the app's crash folder.

use std::backtrace::Backtrace;
use std::fs;
use std::path::{Path, PathBuf};

use crate::log;
use crate::time::{DateTime, unix_now};

/// How many lines of the log a report ends with.
const TAIL: usize = 100;
/// The file in the crash folder that names the newest report an app was told of.
const SEEN: &str = "seen";

/// From now on a panic also writes a report to `folder`, then does what it did before.
pub fn report_panics(folder: &Path) {
    let folder = folder.to_path_buf();
    let before = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let now = DateTime::from_unix(unix_now(), 0);
        let name = format!("{}", now).replace([':', ' '], "-");
        let mut text = format!("{info}\n\n{}\n\nThe last lines of the log:\n", Backtrace::force_capture());
        for line in log::recent(TAIL) {
            text.push_str(&line);
            text.push('\n');
        }
        let _ = fs::create_dir_all(&folder);
        let _ = fs::write(folder.join(format!("{name}.txt")), text);
        log::error("panic", info);
        before(info);
    }));
}

/// The newest report in `folder` written since this was last asked, if any.
pub fn last_crash(folder: &Path) -> Option<PathBuf> {
    let reports = fs::read_dir(folder).ok()?.flatten().map(|entry| entry.path());
    let newest = reports.filter(|path| path.extension().is_some_and(|ext| ext == "txt")).max()?;
    let name = newest.file_name()?.to_string_lossy().into_owned();
    let seen = fs::read_to_string(folder.join(SEEN)).unwrap_or_default();
    if seen.trim() >= name.as_str() {
        return None;
    }
    let _ = fs::write(folder.join(SEEN), &name);
    Some(newest)
}

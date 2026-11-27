//! Crashes that are no panic (specs/16, "Crash reports"): an access to memory that is not
//! the process's, a stack that overflowed. macOS writes the report of the crash itself
//! (`~/Library/Logs/DiagnosticReports`); what it cannot know, the last lines of the app's log,
//! a handler of the signals writes beside the app's reports of panics.

use std::ffi::c_int;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use quadra_app::log;
use quadra_app::time::{DateTime, unix_now};

static FOLDER: OnceLock<PathBuf> = OnceLock::new();

/// The signals of a crash: an instruction that is none, a trap, an abort, arithmetic, and
/// the two of memory (`SIGILL`, `SIGTRAP`, `SIGABRT`, `SIGFPE`, `SIGBUS`, `SIGSEGV`).
const CRASHES: [c_int; 6] = [4, 5, 6, 8, 10, 11];
/// `SA_ONSTACK | SA_RESETHAND`: on the stack kept for signals (a stack that overflowed has
/// no room), and the system's own handler is back once ours is entered.
const ONCE_ON_ITS_STACK: c_int = 0x0001 | 0x0004;

/// `struct sigaction`.
#[repr(C)]
struct Action {
    handler: extern "C" fn(c_int),
    mask: u32,
    flags: c_int,
}

unsafe extern "C" {
    fn sigaction(signal: c_int, new: *const Action, old: *mut Action) -> c_int;
    fn alarm(seconds: u32) -> u32;
}

/// Writes what the app knew when it crashed: the signal and the log's last lines.
fn write_report(folder: &Path, signal: c_int) -> bool {
    let name = format!("{}", DateTime::from_unix(unix_now(), 0)).replace([':', ' '], "-");
    let said = format!(
        "signal {signal}; the system's report of the crash is in ~/Library/Logs/DiagnosticReports\n{}",
        log::recent(100).join("\n")
    );
    std::fs::create_dir_all(folder).is_ok()
        && std::fs::write(folder.join(format!("{name}.txt")), said).is_ok()
}

/// Returning from it, what crashed crashes again under the system's handler, which writes
/// its report and ends the process.
// ponytail: it allocates and writes a file, which a handler of signals is not promised it
// can: a process that crashed inside the allocator waits here for itself, and the alarm
// ends it with no report of ours. A pipe to a thread that writes, when one is seen.
extern "C" fn crashed(signal: c_int) {
    // A panic that aborts is told by the app's report of panics.
    if let Some(folder) = FOLDER.get().filter(|_| !std::thread::panicking()) {
        // SAFETY: asks the system for a signal later, and then for none.
        unsafe { alarm(5) };
        write_report(folder, signal);
        // SAFETY: as above.
        unsafe { alarm(0) };
    }
}

fn watch(signals: &[c_int]) {
    let action = Action { handler: crashed, mask: 0, flags: ONCE_ON_ITS_STACK };
    for &signal in signals {
        // SAFETY: the action is a local of the system's layout; the handler lives as long as
        // the program.
        unsafe { sigaction(signal, &action, std::ptr::null_mut()) };
    }
}

/// From now on a crash that is no panic leaves the log's last 100 lines in `folder`, beside
/// the report the system writes.
pub fn crash_reports(folder: &Path) {
    let _ = FOLDER.set(folder.to_path_buf());
    watch(&CRASHES);
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" {
        fn raise(signal: c_int) -> c_int;
    }

    /// A signal that ends nothing when it is handled (`SIGUSR2`) stands for a crash.
    #[test]
    fn a_signal_leaves_the_log_s_last_lines() {
        let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/tests/crash-mac");
        let _ = std::fs::remove_dir_all(&folder);
        FOLDER.set(folder.clone()).unwrap();
        watch(&[31]);
        // SAFETY: a signal to this thread, which the handler above takes.
        unsafe { raise(31) };
        let written: Vec<PathBuf> =
            std::fs::read_dir(&folder).unwrap().map(|it| it.unwrap().path()).collect();
        assert_eq!(written.len(), 1, "{written:?}");
        let said = std::fs::read_to_string(&written[0]).unwrap();
        assert!(
            written[0].extension().is_some_and(|it| it == "txt") && said.starts_with("signal 31;"),
            "{said}"
        );
    }
}

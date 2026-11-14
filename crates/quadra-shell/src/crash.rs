//! Crashes that are no panic (specs/16, "Crash reports"): an access violation, a stack that
//! overflowed. A handler of unhandled exceptions writes a minidump and the last lines of the
//! log beside the app's reports of panics.

use std::fs::File;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use quadra_app::log;
use quadra_app::time::{DateTime, unix_now};
use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Diagnostics::Debug::{
    EXCEPTION_POINTERS, MINIDUMP_EXCEPTION_INFORMATION, MiniDumpNormal, MiniDumpWriteDump,
    SetUnhandledExceptionFilter,
};
use windows::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId};

static FOLDER: OnceLock<PathBuf> = OnceLock::new();

/// Lets the process go on to end, as it would have without the handler.
const CONTINUE_SEARCH: i32 = 0;

/// Writes a minidump of this process to `path`: of the exception, when there is one.
fn write_dump(path: &Path, exception: Option<*const EXCEPTION_POINTERS>) -> bool {
    let Ok(file) = File::create(path) else { return false };
    let info = exception.map(|pointers| MINIDUMP_EXCEPTION_INFORMATION {
        ThreadId: unsafe { GetCurrentThreadId() },
        ExceptionPointers: pointers as *mut _,
        ClientPointers: false.into(),
    });
    // SAFETY: the file is open for writing through the call, and the exception's pointers,
    // when given, are the OS's for the exception being handled.
    unsafe {
        MiniDumpWriteDump(
            GetCurrentProcess(),
            GetCurrentProcessId(),
            HANDLE(file.as_raw_handle()),
            MiniDumpNormal,
            info.as_ref().map(|i| i as *const _),
            None,
            None,
        )
        .is_ok()
    }
}

unsafe extern "system" fn unhandled(pointers: *const EXCEPTION_POINTERS) -> i32 {
    if let Some(folder) = FOLDER.get() {
        let now = DateTime::from_unix(unix_now(), 0);
        let name = format!("{}", now).replace([':', ' '], "-");
        let _ = std::fs::create_dir_all(folder);
        write_dump(&folder.join(format!("{name}.dmp")), Some(pointers));
        let _ = std::fs::write(folder.join(format!("{name}.txt")), log::recent(100).join("\n"));
    }
    CONTINUE_SEARCH
}

/// From now on a crash that is no panic writes `<date-time>.dmp` and `.txt` (the last 100
/// lines of the log) to `folder`: the app's `crashes` folder (specs/15).
pub fn crash_reports(folder: &Path) {
    if FOLDER.set(folder.to_path_buf()).is_ok() {
        // SAFETY: the handler is a function of the right signature that lives forever.
        unsafe { SetUnhandledExceptionFilter(Some(unhandled)) };
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_dump_of_the_process_is_written() {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.tmp/tests/dump");
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("now.dmp");
        assert!(super::write_dump(&path, None));
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"MDMP") && bytes.len() > 1000, "{}", bytes.len());
    }
}

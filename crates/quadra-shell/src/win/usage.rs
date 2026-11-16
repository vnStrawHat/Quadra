//! What the app's process has used (specs/16, "Process usage").

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use windows::Win32::Foundation::{CloseHandle, FILETIME};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::{
    GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX2,
};
use windows::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId, GetProcessTimes};

use crate::Usage;

/// Seconds from 1601-01-01, where FILETIMEs start, to 1970-01-01.
const SECONDS_BEFORE_1970: u64 = 11_644_473_600;

/// The processor time of the process's threads, how long it has lived, its memory and threads.
pub fn process_usage() -> Option<Usage> {
    let (mut created, mut exited, mut kernel, mut user) =
        (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
    let mut memory = PROCESS_MEMORY_COUNTERS_EX2 {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32,
        ..Default::default()
    };
    let counters = (&mut memory as *mut PROCESS_MEMORY_COUNTERS_EX2).cast::<PROCESS_MEMORY_COUNTERS>();
    // SAFETY: the pseudo handle of the current process needs no closing; every pointer is to
    // a local of the size the call is told, which tells it which structure it is.
    unsafe {
        let process = GetCurrentProcess();
        GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user).ok()?;
        GetProcessMemoryInfo(process, counters, memory.cb).ok()?;
    }
    // FILETIMEs count 100 ns.
    let time = |t: FILETIME| {
        Duration::from_nanos((u64::from(t.dwHighDateTime) << 32 | u64::from(t.dwLowDateTime)) * 100)
    };
    let born = time(created).saturating_sub(Duration::from_secs(SECONDS_BEFORE_1970));
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    Some(Usage {
        cpu: time(kernel) + time(user),
        uptime: now.saturating_sub(born),
        memory: memory.PrivateWorkingSetSize as u64,
        working_set: memory.WorkingSetSize as u64,
        peak_working_set: memory.PeakWorkingSetSize as u64,
        private_bytes: memory.PrivateUsage as u64,
        threads: threads(),
    })
}

/// The process's threads, from its entry in a snapshot of the processes; 0 when not found.
fn threads() -> u32 {
    // SAFETY: the snapshot is a handle of ours, closed on every path after the walk; the
    // entry is a local of the size it says.
    unsafe {
        let me = GetCurrentProcessId();
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return 0 };
        let mut entry =
            PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut found = 0;
        let mut more = Process32FirstW(snapshot, &mut entry).is_ok();
        while more {
            if entry.th32ProcessID == me {
                found = entry.cntThreads;
                break;
            }
            more = Process32NextW(snapshot, &mut entry).is_ok();
        }
        let _ = CloseHandle(snapshot);
        found
    }
}

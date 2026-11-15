//! What the app's process has used (specs/16, "Process usage").

use std::time::Duration;

use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

use crate::Usage;

/// The processor time of the process's threads and its working set.
pub fn process_usage() -> Option<Usage> {
    let (mut created, mut exited, mut kernel, mut user) =
        (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
    let mut memory = PROCESS_MEMORY_COUNTERS::default();
    let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // SAFETY: the pseudo handle of the current process needs no closing; every pointer is to
    // a local of the size the call is told.
    unsafe {
        let process = GetCurrentProcess();
        GetProcessTimes(process, &mut created, &mut exited, &mut kernel, &mut user).ok()?;
        GetProcessMemoryInfo(process, &mut memory, size).ok()?;
    }
    // FILETIMEs count 100 ns.
    let ticks = |t: FILETIME| u64::from(t.dwHighDateTime) << 32 | u64::from(t.dwLowDateTime);
    let cpu = Duration::from_nanos((ticks(kernel) + ticks(user)) * 100);
    Some(Usage { cpu, memory: memory.WorkingSetSize as u64 })
}

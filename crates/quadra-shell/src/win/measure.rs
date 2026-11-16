//! What another process uses, read from outside: the comparison bench's runner (specs/16,
//! "Process usage").

use std::os::windows::io::AsRawHandle;
use std::process::Child;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HANDLE, HWND, LPARAM};
use windows::Win32::System::ProcessStatus::{
    GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX2,
};
use windows::Win32::System::Threading::GetCurrentThread;
use windows::Win32::System::WindowsProgramming::{QueryProcessCycleTime, QueryThreadCycleTime};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
};
use windows::core::BOOL;

fn handle(child: &Child) -> HANDLE {
    HANDLE(child.as_raw_handle())
}

/// The cycle count of every thread of the child since it started.
pub fn process_cycles(child: &Child) -> Option<u64> {
    let mut cycles = 0;
    // SAFETY: the child's handle is open while it is borrowed; the count goes to a local.
    unsafe { QueryProcessCycleTime(handle(child), &mut cycles).ok()? };
    Some(cycles)
}

/// How many cycles the cycle counter advances in a second: measured once, by spinning this
/// thread for 200 ms, then kept.
pub fn cycles_per_second() -> f64 {
    static RATE: OnceLock<f64> = OnceLock::new();
    *RATE.get_or_init(|| {
        let (mut c0, mut c1) = (0u64, 0u64);
        // SAFETY: the pseudo handle of the current thread needs no closing; the counts go to
        // locals.
        unsafe {
            let thread = GetCurrentThread();
            let began = Instant::now();
            let _ = QueryThreadCycleTime(thread, &mut c0);
            while began.elapsed() < Duration::from_millis(200) {}
            let _ = QueryThreadCycleTime(thread, &mut c1);
            (c1 - c0) as f64 / began.elapsed().as_secs_f64()
        }
    })
}

/// The child's private working set in bytes: what the task manager shows as its memory.
pub fn private_memory(child: &Child) -> Option<u64> {
    let mut memory = PROCESS_MEMORY_COUNTERS_EX2 {
        cb: std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX2>() as u32,
        ..Default::default()
    };
    let counters = (&mut memory as *mut PROCESS_MEMORY_COUNTERS_EX2).cast::<PROCESS_MEMORY_COUNTERS>();
    // SAFETY: the structure is the size the call is told, which tells it which one it is.
    unsafe { GetProcessMemoryInfo(handle(child), counters, memory.cb).ok()? };
    Some(memory.PrivateWorkingSetSize as u64)
}

/// Whether a window of the child is visible and not minimized.
pub fn window_shown(child: &Child) -> bool {
    struct Find {
        pid: u32,
        found: bool,
    }
    unsafe extern "system" fn each(window: HWND, find: LPARAM) -> BOOL {
        // SAFETY: `find` is the `Find` below, alive through `EnumWindows`.
        let find = unsafe { &mut *(find.0 as *mut Find) };
        let mut pid = 0;
        // SAFETY: the window is one the enumeration gives; the id goes to a local.
        let shown = unsafe {
            GetWindowThreadProcessId(window, Some(&mut pid));
            pid == find.pid && IsWindowVisible(window).as_bool() && !IsIconic(window).as_bool()
        };
        find.found = shown;
        BOOL::from(!shown) // false stops the enumeration
    }
    let mut find = Find { pid: child.id(), found: false };
    // SAFETY: the callback reads `find` only during the call. It fails when the callback stops
    // it, which is a find.
    let _ = unsafe { EnumWindows(Some(each), LPARAM(&mut find as *mut Find as isize)) };
    find.found
}

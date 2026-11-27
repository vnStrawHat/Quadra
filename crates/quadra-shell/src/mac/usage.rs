//! What a process has used (specs/16, "Process usage"): the app's own, and another's read
//! from outside for the comparison bench's runner. From the system's `libproc`, where
//! Activity Monitor reads the same.

use std::ffi::{c_int, c_void};
use std::process::Child;
use std::time::Duration;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSString, ns_string};

use crate::Usage;

/// `rusage_info_v0`, what `proc_pid_rusage` writes for the flavor 0. Its times are in ticks
/// of the system's clock (`mach_absolute_time`), its sizes in bytes.
#[repr(C)]
#[derive(Default)]
pub(super) struct Used {
    uuid: [u8; 16],
    pub user_time: u64,
    pub system_time: u64,
    pkg_idle_wkups: u64,
    interrupt_wkups: u64,
    pageins: u64,
    wired_size: u64,
    pub resident_size: u64,
    /// What Activity Monitor shows as the process's memory.
    pub phys_footprint: u64,
    pub start: u64,
    exit: u64,
}

/// `proc_taskinfo`, what `proc_pidinfo` writes for `PROC_PIDTASKINFO`.
#[repr(C)]
#[derive(Default)]
struct Task {
    sizes_and_times: [u64; 6],
    /// Counts; the tenth is the task's threads.
    counts: [i32; 12],
}

const PROC_PIDTASKINFO: c_int = 4;

unsafe extern "C" {
    fn proc_pid_rusage(pid: c_int, flavor: c_int, buffer: *mut c_void) -> c_int;
    fn proc_pidinfo(pid: c_int, flavor: c_int, arg: u64, buffer: *mut c_void, size: c_int) -> c_int;
    /// `struct rusage` as eighteen words: two times of two words, then counts, the first of
    /// which is the most memory resident at once, in bytes.
    fn getrusage(who: c_int, usage: *mut [i64; 18]) -> c_int;
    fn mach_absolute_time() -> u64;
    fn mach_timebase_info(info: *mut [u32; 2]) -> c_int;
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    /// Gives an array it leaves to the caller to release.
    fn CGWindowListCopyWindowInfo(
        option: u32,
        relative_to: u32,
    ) -> *mut NSArray<NSDictionary<NSString, AnyObject>>;
}

/// What a process has used; none when it is gone, or another user's.
pub(super) fn used(pid: u32) -> Option<Used> {
    let mut used = Used::default();
    // SAFETY: the structure is the one the flavor 0 writes, in full or not at all.
    let status = unsafe { proc_pid_rusage(pid as c_int, 0, (&raw mut used).cast()) };
    (status == 0).then_some(used)
}

/// How many ticks the system's clock makes in a second.
fn ticks_per_second() -> f64 {
    let mut base = [1u32, 1];
    // SAFETY: two numbers, a tick being the first over the second in nanoseconds.
    unsafe { mach_timebase_info(&mut base) };
    1e9 * f64::from(base[1].max(1)) / f64::from(base[0].max(1))
}

/// The processor time of the process's threads, how long it has lived, its memory and threads.
/// ponytail: the time it has lived leaves out the time the Mac slept (the clock its start
/// is told in stands still then).
pub fn process_usage() -> Option<Usage> {
    let me = std::process::id();
    let (used, rate) = (used(me)?, ticks_per_second());
    let (mut task, mut most) = (Task::default(), [0i64; 18]);
    let size = size_of::<Task>() as c_int;
    // SAFETY: each structure is the one its call writes, told with its size where it asks.
    let (now, read) = unsafe {
        getrusage(0, &mut most);
        (mach_absolute_time(), proc_pidinfo(me as c_int, PROC_PIDTASKINFO, 0, (&raw mut task).cast(), size))
    };
    let seconds = |ticks: u64| Duration::from_secs_f64(ticks as f64 / rate);
    Some(Usage {
        cpu: seconds(used.user_time + used.system_time),
        uptime: seconds(now.saturating_sub(used.start)),
        memory: used.phys_footprint,
        working_set: used.resident_size,
        peak_working_set: most[4].max(0) as u64,
        // What is the process's alone, in RAM or compressed: the footprint again.
        private_bytes: used.phys_footprint,
        threads: if read == size { task.counts[9].max(0) as u32 } else { 0 },
    })
}

/// The processor time of every thread of the child since it started, in nanoseconds: what
/// stands for the count of cycles Windows reads.
pub fn process_cycles(child: &Child) -> Option<u64> {
    let used = used(child.id())?;
    Some(((used.user_time + used.system_time) as f64 * 1e9 / ticks_per_second()) as u64)
}

/// How many of them a second of processor time is.
pub fn cycles_per_second() -> f64 {
    1e9
}

/// The child's memory in bytes, as Activity Monitor shows it.
pub fn private_memory(child: &Child) -> Option<u64> {
    used(child.id()).map(|it| it.phys_footprint)
}

/// Whether a window of the child is on the screen: an app's own window (the layer 0), not
/// minimized and not hidden.
pub fn window_shown(child: &Child) -> bool {
    /// `kCGWindowListOptionOnScreenOnly` and `kCGWindowListExcludeDesktopElements`.
    const ON_SCREEN: u32 = 1 | 16;
    // SAFETY: a list the caller owns, or null: taken as it is given.
    let Some(windows) = (unsafe { Retained::from_raw(CGWindowListCopyWindowInfo(ON_SCREEN, 0)) }) else {
        return false;
    };
    let number = |of: &NSDictionary<NSString, AnyObject>, key: &NSString| {
        of.objectForKey(key)?.downcast::<NSNumber>().ok().map(|it| it.as_i64())
    };
    windows.iter().any(|window| {
        number(&window, ns_string!("kCGWindowOwnerPID")) == Some(i64::from(child.id()))
            && number(&window, ns_string!("kCGWindowLayer")) == Some(0)
    })
}

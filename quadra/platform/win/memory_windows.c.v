module win

// PROCESS_MEMORY_COUNTERS, x64 layout: cb + page faults (u32 each), then 8 SIZE_T fields.
struct ProcessMemoryCounters {
mut:
	cb                    u32
	page_fault_count      u32
	peak_working_set_size usize
	working_set_size      usize
	quota_peak_paged      usize
	quota_paged           usize
	quota_peak_nonpaged   usize
	quota_nonpaged        usize
	pagefile_usage        usize
	peak_pagefile_usage   usize
}

type FnGetProcessMemoryInfo = fn (process voidptr, counters &ProcessMemoryCounters, cb u32) int

// working_set_bytes returns the process's current working set (diagnostics, benchmarks).
pub fn working_set_bytes() u64 {
	lib := load_system_dll('kernel32.dll') or { return 0 }
	f := FnGetProcessMemoryInfo(dll_proc(lib, 'K32GetProcessMemoryInfo') or { return 0 })
	mut c := ProcessMemoryCounters{
		cb: u32(sizeof(ProcessMemoryCounters))
	}
	if f(C.GetCurrentProcess(), &c, c.cb) == 0 {
		return 0
	}
	return u64(c.working_set_size)
}

// PROCESS_MEMORY_COUNTERS_EX2 (Windows 10 1809+): the base fields, then private usage
// (commit), the private working set (Task Manager's "Memory" column) and shared commit.
struct ProcessMemoryCountersEx2 {
	ProcessMemoryCounters
mut:
	private_usage       usize
	private_working_set usize
	shared_commit       u64
}

fn C.GetProcessTimes(process voidptr, creation &u64, exit &u64, kernel &u64, user &u64) int

// process_stats returns the CPU time used so far (kernel + user, in 100 ns units), the
// working set and the private working set in bytes (0 when Windows is too old to report it).
pub fn process_stats() (u64, u64, u64) {
	mut created, mut exited, mut kernel, mut user := u64(0), u64(0), u64(0), u64(0)
	C.GetProcessTimes(C.GetCurrentProcess(), &created, &exited, &kernel, &user)
	lib := load_system_dll('kernel32.dll') or { return kernel + user, 0, 0 }
	f := FnGetProcessMemoryInfo(dll_proc(lib, 'K32GetProcessMemoryInfo') or {
		return kernel + user, 0, 0
	})
	mut c := ProcessMemoryCountersEx2{}
	c.cb = u32(sizeof(ProcessMemoryCountersEx2))
	if f(C.GetCurrentProcess(), &c.ProcessMemoryCounters, c.cb) != 0 {
		return kernel + user, u64(c.working_set_size), u64(c.private_working_set)
	}
	c.cb = u32(sizeof(ProcessMemoryCounters)) // before 1809: no private working set
	if f(C.GetCurrentProcess(), &c.ProcessMemoryCounters, c.cb) == 0 {
		return kernel + user, 0, 0
	}
	return kernel + user, u64(c.working_set_size), 0
}

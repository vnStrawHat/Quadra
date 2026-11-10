// The OS services on Linux and macOS: what plain POSIX gives now, stubs for the rest
// (see specs/13-apps-roadmap.md).
module sys

import os

fn C.kill(pid int, sig int) int

// replace_file renames over an existing destination.
pub fn replace_file(from string, to string) ! {
	os.rename(from, to)!
}

pub fn open_exclusive(path string) !voidptr {
	return error('open_exclusive is not implemented on this OS')
}

pub fn close_handle(handle voidptr) {}

// process_is_live probes with signal 0.
pub fn process_is_live(pid u32) bool {
	return C.kill(int(pid), 0) == 0 || C.errno == C.EPERM
}

pub fn process_is_elevated() ?bool {
	return none
}

pub fn shell_execute_runas(file string, params string, dir string) (bool, u32) {
	return false, 0
}

// GpuUsage: no per-process GPU counter is read on this OS yet.
@[heap]
pub struct GpuUsage {}

pub fn open_gpu_usage() ?&GpuUsage {
	return none
}

pub fn (mut g GpuUsage) sample() ?f32 {
	return none
}

pub fn (mut g GpuUsage) close() {}

pub fn memory_of(pid u32) ?ProcessCounters {
	return none
}

pub fn descendant_processes() []ChildProcess {
	return []
}

pub fn native_heaps() (u64, u64) {
	return 0, 0
}

pub fn loaded_modules() []LoadedModule {
	return []
}

pub fn process_counters() ProcessCounters {
	return ProcessCounters{
		threads: -1
	}
}

// Native capture is Windows-only for now; V panics still go to stderr.
pub fn open_crash_staging(path string) !voidptr {
	return unsafe { nil }
}

pub fn install_crash_filter(handle voidptr, header string) ! {}

pub fn redirect_stderr(path string, header string) ! {}

pub fn open_pipe_rw(path string) !voidptr {
	return error('named pipes are not implemented on this OS')
}

pub fn pipe_read_full(handle voidptr, mut buf []u8) ! {
	return error('named pipes are not implemented on this OS')
}

pub fn pipe_write_all(handle voidptr, data []u8) ! {
	return error('named pipes are not implemented on this OS')
}

pub struct Pageant {}

pub fn open_pageant() !&Pageant {
	return error('Pageant is Windows-only')
}

pub fn (mut a Pageant) query(framed []u8) ![]u8 {
	return error('Pageant is Windows-only')
}

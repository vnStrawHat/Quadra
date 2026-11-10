// Facts about processes on Windows: liveness, elevation and resource use.
module sys

import quadra.platform.win

const process_query_limited = u32(0x1000)
const error_invalid_parameter = u32(87)
const still_active = u32(259)
const token_query = u32(8)
const token_elevation_class = 20

@[callconv: stdcall]
type FnOpenProcess = fn (access u32, inherit int, pid u32) voidptr

@[callconv: stdcall]
type FnGetExitCode = fn (h voidptr, code &u32) int

type FnOpenProcessToken = fn (process voidptr, access u32, token &voidptr) int

type FnGetTokenInformation = fn (token voidptr, class int, out voidptr, size u32, got &u32) int

type FnShellExecuteEx = fn (info voidptr) int

type FnMemInfo = fn (process voidptr, c &MemCounters, cb u32) int

type FnSnapshot = fn (flags u32, pid u32) voidptr

type FnProcess32 = fn (snap voidptr, e &ProcessEntry) int

fn C.GetCurrentProcess() voidptr
fn C.GetCurrentProcessId() u32
fn C.GetProcessTimes(p voidptr, c &u64, e &u64, k &u64, u &u64) int

// process_is_live: only ERROR_INVALID_PARAMETER means gone; access denied is a live process of
// another user, and an answer we cannot get counts as live.
pub fn process_is_live(pid u32) bool {
	lib := win.load_system_dll('kernel32.dll') or { return true }
	open := FnOpenProcess(win.dll_proc(lib, 'OpenProcess') or { return true })
	get_code := FnGetExitCode(win.dll_proc(lib, 'GetExitCodeProcess') or { return true })
	close := FnCloseHandle(win.dll_proc(lib, 'CloseHandle') or { return true })
	handle := open(process_query_limited, 0, pid)
	if handle == unsafe { nil } {
		return C.GetLastError() != error_invalid_parameter
	}
	code := u32(0)
	ok := get_code(handle, &code) != 0
	close(handle)
	return !ok || code == still_active
}

// process_is_elevated reads the process token; none when the query fails.
pub fn process_is_elevated() ?bool {
	lib := win.load_system_dll('advapi32.dll') or { return none }
	open_tok := FnOpenProcessToken(win.dll_proc(lib, 'OpenProcessToken') or { return none })
	get_info := FnGetTokenInformation(win.dll_proc(lib, 'GetTokenInformation') or { return none })
	mut token := unsafe { nil }
	if open_tok(C.GetCurrentProcess(), token_query, &token) == 0 {
		return none
	}
	defer {
		C.CloseHandle(token)
	}
	mut elevated := u32(0)
	mut got := u32(0)
	if get_info(token, token_elevation_class, voidptr(&elevated), 4, &got) == 0 {
		return none
	}
	return elevated != 0
}

// ShellExecuteInfo is SHELLEXECUTEINFOW (x64).
struct ShellExecuteInfo {
mut:
	cb_size      u32
	mask         u32
	hwnd         voidptr
	verb         &u16 = unsafe { nil }
	file         &u16 = unsafe { nil }
	parameters   &u16 = unsafe { nil }
	directory    &u16 = unsafe { nil }
	show         int
	inst_app     voidptr
	id_list      voidptr
	class_name   voidptr
	key_class    voidptr
	hot_key      u32
	icon_monitor voidptr
	process      voidptr
}

// shell_execute_runas starts `file` elevated and returns whether it started and the Windows
// error when it did not. Blocks until the consent prompt answers: call it off the UI thread.
pub fn shell_execute_runas(file string, params string, dir string) (bool, u32) {
	lib := win.load_system_dll('shell32.dll') or { return false, 0 }
	execute := FnShellExecuteEx(win.dll_proc(lib, 'ShellExecuteExW') or { return false, 0 })
	file_w := file.to_wide()
	dir_w := dir.to_wide()
	verb := 'runas'.to_wide()
	params_w := params.to_wide()
	info := ShellExecuteInfo{
		cb_size:    u32(sizeof(ShellExecuteInfo))
		verb:       verb
		file:       file_w
		parameters: params_w
		directory:  dir_w
		show:       1
	}
	ok := execute(voidptr(&info))
	return ok != 0, if ok != 0 {
		u32(0)
	} else {
		C.GetLastError()
	}
}

// MemCounters is PROCESS_MEMORY_COUNTERS_EX2 (x64).
struct MemCounters {
mut:
	cb                  u32
	page_faults         u32
	peak_working_set    usize
	working_set         usize
	peak_paged          usize
	paged               usize
	peak_nonpaged       usize
	nonpaged            usize
	pagefile            usize
	peak_pagefile       usize
	private_usage       usize
	private_working_set usize
	shared_commit       u64
}

// ProcessEntry is PROCESSENTRY32W.
struct ProcessEntry {
mut:
	size      u32
	usage     u32
	pid       u32
	heap_id   usize
	module_id u32
	threads   u32
	parent    u32
	base      int
	flags     u32
	exe       [260]u16
}

fn thread_count() ?int {
	lib := win.load_system_dll('kernel32.dll') or { return none }
	snap := FnSnapshot(win.dll_proc(lib, 'CreateToolhelp32Snapshot') or { return none })
	first := FnProcess32(win.dll_proc(lib, 'Process32FirstW') or { return none })
	next := FnProcess32(win.dll_proc(lib, 'Process32NextW') or { return none })
	s := snap(2, 0) // TH32CS_SNAPPROCESS
	if s == voidptr(-1) {
		return none
	}
	defer {
		C.CloseHandle(s)
	}
	mut e := ProcessEntry{
		size: u32(sizeof(ProcessEntry))
	}
	own := C.GetCurrentProcessId()
	mut more := first(s, &e) != 0
	for more {
		if e.pid == own {
			return int(e.threads)
		}
		more = next(s, &e) != 0
	}
	return none
}

// process_counters reads this process's memory, CPU time and threads. Run it off the UI thread.
pub fn process_counters() ProcessCounters {
	mut created, mut exited, mut kernel, mut user := u64(0), u64(0), u64(0), u64(0)
	C.GetProcessTimes(C.GetCurrentProcess(), &created, &exited, &kernel, &user)
	mut out := ProcessCounters{
		cpu_time_100ns: kernel + user
		threads:        thread_count() or { -1 }
	}
	lib := win.load_system_dll('kernel32.dll') or { return out }
	f := FnMemInfo(win.dll_proc(lib, 'K32GetProcessMemoryInfo') or { return out })
	mut c := MemCounters{}
	c.cb = u32(sizeof(MemCounters))
	if f(C.GetCurrentProcess(), &c, c.cb) != 0 {
		out.private_ws = u64(c.private_working_set)
		out.working_set = u64(c.working_set)
		out.commit = u64(c.private_usage)
		out.peak = u64(c.peak_working_set)
	}
	return out
}

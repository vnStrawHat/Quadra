// Native crashes: an unhandled-exception filter writes a minidump plus caller-supplied
// text (the last log lines) to a directory, then hands over to the previous filter.
module win

import os

@[callconv: stdcall]
type FnMiniDumpWriteDump = fn (process voidptr, pid u32, file voidptr, kind u32, exc voidptr, user voidptr, callback voidptr) int

@[callconv: stdcall]
type FnExceptionFilter = fn (info voidptr) int

fn C.SetUnhandledExceptionFilter(f voidptr) voidptr
fn C.CreateFileW(name &u16, access u32, share u32, sec voidptr, disp u32, flags u32, tmpl voidptr) voidptr
fn C.CloseHandle(h voidptr) int
fn C.GetCurrentProcess() voidptr
fn C.GetCurrentProcessId() u32
fn C.GetCurrentThreadId() u32
fn C.GetTickCount() u32

struct CrashState {
mut:
	dir   string
	extra fn () string        = unsafe { nil }
	dump  FnMiniDumpWriteDump = unsafe { nil }
	prev  voidptr
}

// One per process: the filter has no other way to find its state.
const crash_state = &CrashState{}

// install_crash_handler makes unhandled exceptions write <dir>/crash-<tick>.dmp and .txt.
pub fn install_crash_handler(dir string, extra fn () string) {
	mut s := unsafe { crash_state }
	s.dir = dir
	s.extra = extra
	lib := load_system_dll('dbghelp.dll') or { return }
	s.dump = FnMiniDumpWriteDump(dll_proc(lib, 'MiniDumpWriteDump') or { return })
	s.prev = C.SetUnhandledExceptionFilter(voidptr(crash_filter))
}

@[callconv: stdcall]
fn crash_filter(info voidptr) int {
	s := crash_state
	base := os.join_path(s.dir, 'crash-${C.GetCurrentProcessId()}-${C.GetTickCount()}')
	h := C.CreateFileW('${base}.dmp'.to_wide(), 0x40000000, 0, unsafe { nil }, 2, 0x80,
		unsafe { nil }) // GENERIC_WRITE, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL
	if h != voidptr(-1) {
		// MINIDUMP_EXCEPTION_INFORMATION is packed to 4 bytes: thread id, pointers, bool.
		mut mei := [16]u8{}
		unsafe {
			*(&u32(&mei[0])) = C.GetCurrentThreadId()
			*(&voidptr(&mei[4])) = info
			*(&int(&mei[12])) = 0
		}
		s.dump(C.GetCurrentProcess(), C.GetCurrentProcessId(), h, 0, &mei[0], unsafe { nil },
			unsafe { nil })
		C.CloseHandle(h)
	}
	if s.extra != unsafe { nil } {
		os.write_file('${base}.txt', s.extra()) or {}
	}
	if s.prev != unsafe { nil } {
		prev := FnExceptionFilter(s.prev)
		return prev(info)
	}
	return 0 // EXCEPTION_CONTINUE_SEARCH
}

pub fn (mut p WinPlatform) install_crash_handler(dir string, extra fn () string) {
	install_crash_handler(dir, extra)
}

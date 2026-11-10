// Crash capture on Windows: an unhandled-exception filter writes a bounded report through a
// direct WriteFile on a pre-opened handle, and V panics land in a staging file via stderr.
// The filter runs in a compromised context: it does not allocate, lock or log.
module sys

import os
import quadra.platform.win

#include <wchar.h>

const share_all = u32(7) // FILE_SHARE_READ | WRITE | DELETE
const create_always = u32(2)
const std_error_handle = u32(0xFFFFFFF4) // (DWORD)-12
const sigsegv = 11

@[callconv: stdcall]
type FnSetFilter = fn (filter voidptr) voidptr

@[callconv: stdcall]
type FnExceptionFilter = fn (info voidptr) int

@[callconv: stdcall]
type FnWriteFile = fn (h voidptr, buf voidptr, n u32, written &u32, ov voidptr) int

@[callconv: stdcall]
type FnSetStdHandle = fn (which u32, h voidptr) int

fn C._get_osfhandle(fd int) isize
fn C.signal(sig int, handler voidptr) voidptr

struct NativeState {
mut:
	handle voidptr
	write  FnWriteFile = unsafe { nil }
	prev   voidptr
	header string
	pid    u32
}

// One per process: the filter has no other way to find its state.
const native_state = &NativeState{}

struct NativeExceptionRecord {
	code    u32
	flags   u32
	record  voidptr
	address voidptr
}

struct NativeExceptionPointers {
	record  &NativeExceptionRecord = unsafe { nil }
	context voidptr
}

// FixedBuffer formats without allocating.
struct FixedBuffer {
mut:
	bytes [512]u8
	len   int
}

fn (mut b FixedBuffer) push_bytes(data &u8, n int) {
	count := if n > 512 - b.len { 512 - b.len } else { n }
	if count <= 0 {
		return
	}
	unsafe { vmemcpy(&b.bytes[b.len], data, count) }
	b.len += count
}

fn (mut b FixedBuffer) push_str(value string) {
	b.push_bytes(value.str, value.len)
}

fn (mut b FixedBuffer) push_u64(value u64) {
	mut digits := [20]u8{}
	mut start := 20
	mut rest := value
	for {
		start--
		digits[start] = u8(`0`) + u8(rest % 10)
		rest /= 10
		if rest == 0 {
			break
		}
	}
	b.push_bytes(&digits[start], 20 - start)
}

fn (mut b FixedBuffer) push_i64(value i64) {
	if value < 0 {
		b.push_str('-')
		b.push_u64(u64(-value))
	} else {
		b.push_u64(u64(value))
	}
}

fn (mut b FixedBuffer) push_hex(value u64) {
	mut digits := [16]u8{}
	mut start := 16
	mut rest := value
	for {
		start--
		digits[start] = '0123456789ABCDEF'[int(rest & 0xF)]
		rest >>= 4
		if rest == 0 {
			break
		}
	}
	b.push_str('0x')
	b.push_bytes(&digits[start], 16 - start)
}

fn write_direct(data &u8, n int) bool {
	s := native_state
	mut p := unsafe { data }
	mut remaining := n
	for remaining > 0 {
		written := u32(0)
		if s.write(s.handle, p, u32(remaining), &written, unsafe { nil }) == 0 || written == 0 {
			return false
		}
		unsafe {
			p += int(written)
		}
		remaining -= int(written)
	}
	return true
}

@[callconv: stdcall]
fn native_filter(info voidptr) int {
	s := native_state
	if s.handle != unsafe { nil } && s.handle != invalid_handle {
		format_and_write(unsafe { &NativeExceptionPointers(info) })
	}
	if s.prev != unsafe { nil } {
		prev := FnExceptionFilter(s.prev)
		return prev(info)
	}
	return 0 // EXCEPTION_CONTINUE_SEARCH: normal termination proceeds
}

fn format_and_write(p &NativeExceptionPointers) {
	s := native_state
	mut details := FixedBuffer{}
	details.push_str('Exception code: ')
	record := p.record
	if record != unsafe { nil } {
		details.push_hex(u64(record.code))
		details.push_str('\nException address: ')
		details.push_hex(u64(record.address))
	}
	details.push_str('\nProcess ID: ')
	details.push_u64(u64(s.pid))
	details.push_str('\n')
	if write_direct(s.header.str, s.header.len) {
		write_direct(&details.bytes[0], details.len)
	}
}

// open_crash_staging creates the file the exception filter writes to.
pub fn open_crash_staging(path string) !voidptr {
	lib := win.load_system_dll('kernel32.dll')!
	create := FnCreateFileW(win.dll_proc(lib, 'CreateFileW')!)
	handle := create(path.to_wide(), generic_write, share_all, unsafe { nil }, create_always,
		file_attribute_normal, unsafe { nil })
	if handle == invalid_handle {
		return error('failed to open native crash staging file: ${C.GetLastError()}')
	}
	return handle
}

fn prepare_native_state(handle voidptr, header string) ! {
	lib := win.load_system_dll('kernel32.dll')!
	mut s := unsafe { native_state }
	s.write = FnWriteFile(win.dll_proc(lib, 'WriteFile')!)
	s.handle = handle
	s.header = header
	s.pid = u32(os.getpid())
}

// redirect_stderr points stderr at `path`, header first: V's panic() prints there and exits.
pub fn redirect_stderr(path string, header string) ! {
	os.write_file(path, header)!
	if C._wfreopen(path.to_wide(), 'ab'.to_wide(), C.stderr) == unsafe { nil } {
		return error('cannot redirect stderr to ${path}')
	}
	lib := win.load_system_dll('kernel32.dll')!
	set_std := FnSetStdHandle(win.dll_proc(lib, 'SetStdHandle')!)
	set_std(std_error_handle, voidptr(C._get_osfhandle(C.fileno(C.stderr))))
	C.setvbuf(C.stderr, unsafe { nil }, 4, 0) // _IONBF: nothing to lose at exit
}

// install_crash_filter attaches the exception filter, which writes `header` and the exception
// details to the staging `handle`. Call it before the UI starts.
pub fn install_crash_filter(handle voidptr, header string) ! {
	prepare_native_state(handle, header)!
	lib := win.load_system_dll('kernel32.dll')!
	set_filter := FnSetFilter(win.dll_proc(lib, 'SetUnhandledExceptionFilter')!)
	mut s := unsafe { native_state }
	// The mingw CRT turns an access violation into SIGSEGV, and V's handler for it exits before
	// any filter runs; default handling lets the exception reach the filter.
	C.signal(sigsegv, unsafe { nil })
	s.prev = set_filter(voidptr(native_filter))
}

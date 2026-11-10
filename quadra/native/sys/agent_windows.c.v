// Transports to a local SSH agent on Windows: a named pipe, and Pageant's shared-memory message.
module sys

import quadra.platform.win

const pageant_copydata_id = usize(0x804e50ba)
const pageant_max = 8192
const page_readwrite = u32(4)
const file_map_write = u32(2)

fn C.CreateFileMappingA(h voidptr, sec voidptr, protect u32, high u32, low u32, name &char) voidptr
fn C.MapViewOfFile(h voidptr, access u32, high u32, low u32, n usize) voidptr
fn C.UnmapViewOfFile(p voidptr) int
fn C.GetCurrentThreadId() u32

// open_pipe_rw opens an existing named pipe; the error's code is the Windows error.
pub fn open_pipe_rw(path string) !voidptr {
	h := C.CreateFileW(path.to_wide(), generic_read | generic_write, 0, unsafe { nil },
		open_existing, 0, unsafe { nil })
	if h == invalid_handle {
		return error_with_code('cannot open ${path}', int(C.GetLastError()))
	}
	return h
}

// pipe_read_full fills `buf` or fails.
pub fn pipe_read_full(handle voidptr, mut buf []u8) ! {
	mut got := 0
	for got < buf.len {
		mut n := u32(0)
		if !C.ReadFile(handle, unsafe { &u8(buf.data) + got }, u32(buf.len - got), &n, unsafe { nil })
			|| n == 0 {
			return error('agent pipe read failed')
		}
		got += int(n)
	}
}

// pipe_write_all writes all of `data` or fails.
pub fn pipe_write_all(handle voidptr, data []u8) ! {
	mut pos := 0
	for pos < data.len {
		mut n := u32(0)
		if !C.WriteFile(handle, unsafe { &u8(data.data) + pos }, u32(data.len - pos), &n,
			unsafe { nil }) {
			return error('agent pipe write failed')
		}
		pos += int(n)
	}
}

type FnFindWindow = fn (class &char, title &char) voidptr

type FnSendMessage = fn (hwnd voidptr, msg u32, wparam usize, lparam voidptr) isize

// CopyData mirrors COPYDATASTRUCT.
struct CopyData {
	data_id usize
	len     u32
	ptr     voidptr
}

// Pageant is a running Pageant window.
@[heap]
pub struct Pageant {
mut:
	hwnd voidptr
	send FnSendMessage = unsafe { nil }
}

pub fn open_pageant() !&Pageant {
	user32 := win.load_system_dll('user32.dll')!
	find := unsafe { FnFindWindow(win.dll_proc(user32, 'FindWindowA')!) }
	send := unsafe { FnSendMessage(win.dll_proc(user32, 'SendMessageA')!) }
	class := c'Pageant'
	hwnd := find(class, class)
	if hwnd == unsafe { nil } {
		return error('not running')
	}
	return &Pageant{
		hwnd: hwnd
		send: send
	}
}

// query sends one length-prefixed request and returns the reply without its length.
pub fn (mut a Pageant) query(framed []u8) ![]u8 {
	if framed.len > pageant_max {
		return error('agent request too large')
	}
	name := 'PageantRequest${C.GetCurrentThreadId()}'
	mapping := C.CreateFileMappingA(invalid_handle, unsafe { nil }, page_readwrite, 0, pageant_max,
		&char(name.str))
	if mapping == unsafe { nil } {
		return error('CreateFileMapping failed')
	}
	defer {
		C.CloseHandle(mapping)
	}
	view := C.MapViewOfFile(mapping, file_map_write, 0, 0, pageant_max)
	if view == unsafe { nil } {
		return error('MapViewOfFile failed')
	}
	defer {
		C.UnmapViewOfFile(view)
	}
	unsafe { vmemcpy(view, framed.data, framed.len) }
	cds := CopyData{
		data_id: pageant_copydata_id
		len:     u32(name.len + 1)
		ptr:     voidptr(name.str)
	}
	if a.send(a.hwnd, 0x004a, 0, voidptr(&cds)) == 0 {
		return error('Pageant refused the request')
	}
	mem := unsafe { &u8(view) }
	n := unsafe {
		int(u32(mem[0]) << 24 | u32(mem[1]) << 16 | u32(mem[2]) << 8 | u32(mem[3]))
	}
	if n < 1 || n + 4 > pageant_max {
		return error('bad Pageant reply')
	}
	mut body := []u8{len: n}
	unsafe { vmemcpy(body.data, mem + 4, n) }
	return body
}

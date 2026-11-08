// Unicode clipboard and cursors.
module win

import quadra.platform

fn C.OpenClipboard(hwnd voidptr) int
fn C.CloseClipboard() int
fn C.EmptyClipboard() int
fn C.GetClipboardData(format u32) voidptr
fn C.SetClipboardData(format u32, mem voidptr) voidptr
fn C.GlobalAlloc(flags u32, bytes usize) voidptr
fn C.GlobalLock(mem voidptr) voidptr
fn C.GlobalUnlock(mem voidptr) int

const cf_unicodetext = u32(13)
const gmem_moveable = u32(0x0002)

pub fn (p &WinPlatform) clipboard_read() ?string {
	if C.OpenClipboard(p.msg_hwnd) == 0 {
		return none
	}
	defer {
		C.CloseClipboard()
	}
	h := C.GetClipboardData(cf_unicodetext)
	if h == unsafe { nil } {
		return none
	}
	ptr := C.GlobalLock(h)
	if ptr == unsafe { nil } {
		return none
	}
	s := unsafe { string_from_wide(&u16(ptr)) }
	C.GlobalUnlock(h)
	return s.replace('\r\n', '\n')
}

pub fn (p &WinPlatform) clipboard_write(s string) {
	wide := s.replace('\n', '\r\n').to_wide()
	mut n := 0
	for unsafe { wide[n] } != 0 {
		n++
	}
	bytes := usize((n + 1) * 2)
	if C.OpenClipboard(p.msg_hwnd) == 0 {
		return
	}
	defer {
		C.CloseClipboard()
	}
	C.EmptyClipboard()
	h := C.GlobalAlloc(gmem_moveable, bytes)
	if h == unsafe { nil } {
		return
	}
	dst := C.GlobalLock(h)
	unsafe { vmemcpy(dst, wide, int(bytes)) }
	C.GlobalUnlock(h)
	C.SetClipboardData(cf_unicodetext, h) // the system owns h from here on
}

fn apply_cursor(c platform.Cursor) {
	id := match c {
		.arrow { 32512 }
		.ibeam { 32513 }
		.wait { 32514 }
		.crosshair { 32515 }
		.pointer { 32649 }
		.resize_nwse { 32642 }
		.resize_nesw { 32643 }
		.resize_ew { 32644 }
		.resize_ns { 32645 }
		.move { 32646 }
		.not_allowed { 32648 }
		.hidden { 0 }
	}

	mut cursor := unsafe { nil }
	if id != 0 {
		cursor = C.LoadCursorW(unsafe { nil }, voidptr(usize(id)))
	}
	C.SetCursor(cursor)
}

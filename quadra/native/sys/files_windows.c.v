// File and handle operations on Windows.
module sys

import quadra.platform.win

const generic_read = u32(0x80000000)
const generic_write = u32(0x40000000)
const open_existing = u32(3)
const file_attribute_normal = u32(0x80)
const invalid_handle = voidptr(-1)
const movefile_replace_existing = u32(1)
const movefile_write_through = u32(8)

@[callconv: stdcall]
type FnCreateFileW = fn (name &u16, access u32, share u32, sec voidptr, disp u32, flags u32, tmpl voidptr) voidptr

@[callconv: stdcall]
type FnCloseHandle = fn (h voidptr) int

type FnMoveFileExW = fn (from &u16, to &u16, flags u32) int

// replace_file renames over an existing destination; os.rename on Windows refuses one.
pub fn replace_file(from string, to string) ! {
	lib := win.load_system_dll('kernel32.dll')!
	move := FnMoveFileExW(win.dll_proc(lib, 'MoveFileExW')!)
	if move(from.to_wide(), to.to_wide(), movefile_replace_existing | movefile_write_through) == 0 {
		return error('cannot replace ${to}: error ${C.GetLastError()}')
	}
}

// open_exclusive opens `path` with no sharing, so any other open of it fails until the handle
// is given to close_handle.
pub fn open_exclusive(path string) !voidptr {
	lib := win.load_system_dll('kernel32.dll')!
	create := FnCreateFileW(win.dll_proc(lib, 'CreateFileW')!)
	handle := create(path.to_wide(), generic_read, 0, unsafe { nil }, open_existing,
		file_attribute_normal, unsafe { nil })
	if handle == invalid_handle {
		return error('cannot open ${path}: error ${C.GetLastError()}')
	}
	return handle
}

pub fn close_handle(handle voidptr) {
	C.CloseHandle(handle)
}

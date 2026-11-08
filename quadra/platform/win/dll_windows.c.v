// Dynamic loading of system DLLs. Only System32 is searched, which rules out DLL planting.
module win

const load_library_search_system32 = u32(0x00000800)

fn C.LoadLibraryExW(name &u16, file voidptr, flags u32) voidptr
fn C.GetProcAddress(module_ voidptr, name &char) voidptr

pub fn load_system_dll(name string) !voidptr {
	h := C.LoadLibraryExW(name.to_wide(), unsafe { nil }, load_library_search_system32)
	if h == unsafe { nil } {
		return error('LoadLibraryExW(${name}) failed: ${C.GetLastError()}')
	}
	return h
}

pub fn dll_proc(lib voidptr, name string) !voidptr {
	p := C.GetProcAddress(lib, &char(name.str))
	if p == unsafe { nil } {
		return error('GetProcAddress(${name}) failed')
	}
	return p
}

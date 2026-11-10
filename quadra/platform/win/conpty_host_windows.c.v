// Which host serves a pseudo console (specs/17, ConPTY host): the Windows Terminal pair beside
// the executable (`conpty.dll`, which starts `x64\OpenConsole.exe`), else the system one.
module win

import os

// Only the DLL's own directory and System32 are searched for its dependencies.
const load_library_search_dll_load_dir = u32(0x00000100)

// The width rule of the parser (PSEUDOCONSOLE_GLYPH_WIDTH_WCSWIDTH), passed to the bundled host.
const pseudoconsole_glyph_width_wcswidth = u32(0x10)

pub enum ConptyHost {
	system
	bundled
}

pub fn (h ConptyHost) name() string {
	return if h == .bundled { 'conpty: bundled' } else { 'conpty: system' }
}

// conpty_host names the host the next spawn_conpty gets.
pub fn conpty_host() ConptyHost {
	return pc_api() or { return .system }.host
}

// resolve_conpty is the host choice against an explicit directory.
pub fn resolve_conpty(dir string) ConptyHost {
	return resolve_pc_api(dir) or { return .system }.host
}

fn pc_api() !PcApi {
	return resolve_pc_api(os.dir(os.executable()))
}

fn resolve_pc_api(dir string) !PcApi {
	k := load_system_dll('kernel32.dll')!
	init := unsafe { FnInitAttrs(dll_proc(k, 'InitializeProcThreadAttributeList')!) }
	update := unsafe { FnUpdateAttr(dll_proc(k, 'UpdateProcThreadAttribute')!) }
	path := os.join_path(dir, 'conpty.dll')
	if os.is_file(path) {
		if api := pc_procs(load_dll_at(path) or { unsafe { nil } }) {
			return PcApi{
				...api
				init:   init
				update: update
				host:   .bundled
			}
		}
	}
	api := pc_procs(k)!
	return PcApi{
		...api
		init:   init
		update: update
	}
}

// pc_procs binds the three pseudo console functions of one module, all or none.
fn pc_procs(lib voidptr) !PcApi {
	if lib == unsafe { nil } {
		return error('no module')
	}
	return PcApi{
		create: unsafe { FnCreatePC(dll_proc(lib, 'CreatePseudoConsole')!) }
		resize: unsafe { FnResizePC(dll_proc(lib, 'ResizePseudoConsole')!) }
		close:  unsafe { FnClosePC(dll_proc(lib, 'ClosePseudoConsole')!) }
	}
}

// load_dll_at loads a DLL by absolute path; it is never freed, so its functions stay valid.
fn load_dll_at(path string) !voidptr {
	full := os.real_path(path).replace('/', '\\')
	h := C.LoadLibraryExW(full.to_wide(), unsafe { nil },
		load_library_search_dll_load_dir | load_library_search_system32)
	if h == unsafe { nil } {
		return error('LoadLibraryExW(${full}) failed: ${C.GetLastError()}')
	}
	return h
}

// Ending the console hosts an app started from its own directory (specs/25, update): a bundled
// OpenConsole.exe that outlived the app holds the files an update overwrites.
module sys

import os
import quadra.platform.win

const process_terminate = u32(0x0001)

@[callconv: stdcall]
type FnQueryImageName = fn (h voidptr, flags u32, name &u16, size &u32) int

@[callconv: stdcall]
type FnTerminate = fn (h voidptr, code u32) int

// terminate_console_hosts_in ends every OpenConsole.exe whose image path lies inside dir and
// returns their image paths. Other apps' hosts of the same name are untouched.
pub fn terminate_console_hosts_in(dir string) []string {
	return terminate_in_dir('OpenConsole.exe', dir)
}

fn terminate_in_dir(image string, dir string) []string {
	mut ended := []string{}
	lib := win.load_system_dll('kernel32.dll') or { return ended }
	snap := FnSnapshot(win.dll_proc(lib, 'CreateToolhelp32Snapshot') or { return ended })
	first := FnProcess32(win.dll_proc(lib, 'Process32FirstW') or { return ended })
	next := FnProcess32(win.dll_proc(lib, 'Process32NextW') or { return ended })
	open := FnOpenProcess(win.dll_proc(lib, 'OpenProcess') or { return ended })
	query := FnQueryImageName(win.dll_proc(lib, 'QueryFullProcessImageNameW') or { return ended })
	kill := FnTerminate(win.dll_proc(lib, 'TerminateProcess') or { return ended })
	root := os.real_path(dir)
	s := snap(2, 0) // TH32CS_SNAPPROCESS
	if s == voidptr(-1) {
		return ended
	}
	defer {
		C.CloseHandle(s)
	}
	mut e := ProcessEntry{
		size: u32(sizeof(ProcessEntry))
	}
	mut more := first(s, &e) != 0
	for more {
		if unsafe { string_from_wide(&e.exe[0]) }.to_lower() == image.to_lower() {
			h := open(process_query_limited | process_terminate, 0, e.pid)
			if h != unsafe { nil } {
				mut buf := [1024]u16{}
				mut n := u32(buf.len)
				if query(h, 0, &buf[0], &n) != 0 {
					path := unsafe { string_from_wide2(&buf[0], int(n)) }
					if under(os.real_path(path), root) && kill(h, 1) != 0 {
						ended << path
					}
				}
				C.CloseHandle(h)
			}
		}
		more = next(s, &e) != 0
	}
	return ended
}

// under: path lies inside dir (any case, either slash); a sibling with a longer name does not.
fn under(path string, dir string) bool {
	return path_key(path).starts_with(path_key(dir) + '\\')
}

fn path_key(p string) string {
	return p.replace('/', '\\').trim_right('\\').to_lower()
}

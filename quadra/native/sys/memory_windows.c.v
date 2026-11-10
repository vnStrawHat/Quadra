// Memory facts for a memory report (OneTerm --mem-report): other processes' counters, this
// process's children and its loaded modules.
module sys

import quadra.platform.win

type FnEnumModules = fn (process voidptr, mods &voidptr, cb u32, needed &u32) int

type FnModuleName = fn (process voidptr, m voidptr, name &u16, size u32) u32

type FnModuleInfo = fn (process voidptr, m voidptr, info &ModuleInfo, cb u32) int

type FnGetProcessHeaps = fn (n u32, heaps &voidptr) u32

type FnHeapSummary = fn (heap voidptr, flags u32, s &HeapSummary) int

struct HeapSummary {
mut:
	cb          u32
	allocated   usize
	committed   usize
	reserved    usize
	max_reserve usize
}

// native_heaps sums the process heaps (the C runtime's malloc, DirectWrite, D3D and other
// system DLLs): bytes allocated and committed.
pub fn native_heaps() (u64, u64) {
	lib := win.load_system_dll('kernel32.dll') or { return 0, 0 }
	heaps_of := FnGetProcessHeaps(win.dll_proc(lib, 'GetProcessHeaps') or { return 0, 0 })
	summary := FnHeapSummary(win.dll_proc(lib, 'HeapSummary') or { return 0, 0 })
	heaps := unsafe { []voidptr{len: 64} }
	mut n := int(heaps_of(u32(heaps.len), unsafe { &heaps[0] }))
	if n > heaps.len {
		n = heaps.len
	}
	mut alloc, mut commit := u64(0), u64(0)
	for i in 0 .. n {
		mut s := HeapSummary{
			cb: u32(sizeof(HeapSummary))
		}
		if summary(heaps[i], 0, &s) != 0 {
			alloc += u64(s.allocated)
			commit += u64(s.committed)
		}
	}
	return alloc, commit
}

struct ModuleInfo {
	base  voidptr
	size  u32
	entry voidptr
}

// memory_of reads the memory counters of another process (none when it cannot be opened).
pub fn memory_of(pid u32) ?ProcessCounters {
	lib := win.load_system_dll('kernel32.dll') or { return none }
	open := FnOpenProcess(win.dll_proc(lib, 'OpenProcess') or { return none })
	f := FnMemInfo(win.dll_proc(lib, 'K32GetProcessMemoryInfo') or { return none })
	h := open(process_query_limited, 0, pid)
	if h == unsafe { nil } {
		return none
	}
	defer {
		C.CloseHandle(h)
	}
	mut c := MemCounters{}
	c.cb = u32(sizeof(MemCounters))
	if f(h, &c, c.cb) == 0 {
		return none
	}
	return ProcessCounters{
		threads:     -1
		private_ws:  u64(c.private_working_set)
		working_set: u64(c.working_set)
		commit:      u64(c.private_usage)
		peak:        u64(c.peak_working_set)
	}
}

// descendant_processes lists every live process below this one (console hosts, shells).
pub fn descendant_processes() []ChildProcess {
	lib := win.load_system_dll('kernel32.dll') or { return [] }
	snap := FnSnapshot(win.dll_proc(lib, 'CreateToolhelp32Snapshot') or { return [] })
	first := FnProcess32(win.dll_proc(lib, 'Process32FirstW') or { return [] })
	next := FnProcess32(win.dll_proc(lib, 'Process32NextW') or { return [] })
	s := snap(2, 0) // TH32CS_SNAPPROCESS
	if s == voidptr(-1) {
		return []
	}
	defer {
		C.CloseHandle(s)
	}
	mut all := []ChildProcess{}
	mut parents := []u32{}
	mut e := ProcessEntry{
		size: u32(sizeof(ProcessEntry))
	}
	mut more := first(s, &e) != 0
	for more {
		all << ChildProcess{e.pid, unsafe { string_from_wide(&e.exe[0]) }}
		parents << e.parent
		more = next(s, &e) != 0
	}
	mut out := []ChildProcess{}
	mut owners := [C.GetCurrentProcessId()]
	for i := 0; i < owners.len; i++ { // breadth first; owners grows as children are found
		for j, p in all {
			if parents[j] == owners[i] && p.pid != 0 && p.pid !in owners {
				owners << p.pid
				out << p
			}
		}
	}
	return out
}

// loaded_modules lists the modules mapped in this process.
pub fn loaded_modules() []LoadedModule {
	lib := win.load_system_dll('kernel32.dll') or { return [] }
	enum_mods := FnEnumModules(win.dll_proc(lib, 'K32EnumProcessModules') or { return [] })
	base_name := FnModuleName(win.dll_proc(lib, 'K32GetModuleBaseNameW') or { return [] })
	info_of := FnModuleInfo(win.dll_proc(lib, 'K32GetModuleInformation') or { return [] })
	me := C.GetCurrentProcess()
	mods := unsafe { []voidptr{len: 512} }
	mut needed := u32(0)
	if enum_mods(me, unsafe { &mods[0] }, u32(mods.len * int(sizeof(voidptr))), &needed) == 0 {
		return []
	}
	n := int(needed / u32(sizeof(voidptr)))
	mut out := []LoadedModule{cap: n}
	mut buf := []u16{len: 260}
	mut count := n
	if count > mods.len {
		count = mods.len
	}
	for i in 0 .. count {
		len := base_name(me, mods[i], unsafe { &buf[0] }, u32(buf.len))
		mut info := ModuleInfo{}
		info_of(me, mods[i], &info, u32(sizeof(ModuleInfo)))
		out << LoadedModule{unsafe { string_from_wide2(&buf[0], int(len)) }, info.size}
	}
	return out
}

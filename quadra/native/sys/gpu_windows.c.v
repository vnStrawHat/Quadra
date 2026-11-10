// This process's GPU utilisation from PDH's `GPU Engine` counters, the source of Task
// Manager's GPU column (specs/22). pdh.dll is loaded at run time.
module sys

import quadra.platform.win

const pdh_more_data = u32(0x800007D2)
const pdh_fmt_double = u32(0x200)

type FnPdhOpenQuery = fn (source voidptr, user usize, query &voidptr) u32

type FnPdhAddCounter = fn (query voidptr, path &u16, user usize, counter &voidptr) u32

type FnPdhCollect = fn (query voidptr) u32

type FnPdhGetArray = fn (counter voidptr, format u32, bytes &u32, count &u32, items voidptr) u32

// PdhItem is PDH_FMT_COUNTERVALUE_ITEM_W with a double value (x64: 24 bytes).
struct PdhItem {
	name   &u16 = unsafe { nil }
	status u32
	value  f64
}

// GpuUsage is an open query on `\GPU Engine(*)\Utilization Percentage`. Use it from one
// thread at a time, off the UI thread: a sample walks every engine of every process.
@[heap]
pub struct GpuUsage {
mut:
	query   voidptr
	counter voidptr
	buf     []u64 // reused; PDH writes the names after the items, so it is sized in bytes
	collect FnPdhCollect  = unsafe { nil }
	get     FnPdhGetArray = unsafe { nil }
	close_q FnPdhCollect  = unsafe { nil }
}

// open_gpu_usage opens the query and takes the first collection (utilisation is a rate
// between two). The first open in a process takes 100-165 ms: call it off the UI thread.
pub fn open_gpu_usage() ?&GpuUsage {
	if sizeof(PdhItem) != 24 {
		return none
	}
	lib := win.load_system_dll('pdh.dll') or { return none }
	open := FnPdhOpenQuery(win.dll_proc(lib, 'PdhOpenQueryW') or { return none })
	add := FnPdhAddCounter(win.dll_proc(lib, 'PdhAddEnglishCounterW') or { return none })
	mut g := &GpuUsage{
		collect: FnPdhCollect(win.dll_proc(lib, 'PdhCollectQueryData') or { return none })
		get:     FnPdhGetArray(win.dll_proc(lib, 'PdhGetFormattedCounterArrayW') or { return none })
		close_q: FnPdhCollect(win.dll_proc(lib, 'PdhCloseQuery') or { return none })
	}
	if open(unsafe { nil }, 0, &g.query) != 0 {
		return none
	}
	// The English name resolves on a localized Windows too.
	path := r'\GPU Engine(*)\Utilization Percentage'.to_wide()
	if add(g.query, path, 0, &g.counter) != 0 {
		g.close()
		return none
	}
	g.collect(g.query)
	return g
}

// sample collects again and returns this process's busiest engine, in percent; none when
// PDH fails or the process has no engine.
pub fn (mut g GpuUsage) sample() ?f32 {
	if g.query == unsafe { nil } || g.collect(g.query) != 0 {
		return none
	}
	mut bytes, mut count := u32(0), u32(0)
	if g.get(g.counter, pdh_fmt_double, &bytes, &count, unsafe { nil }) != pdh_more_data
		|| bytes == 0 {
		return none
	}
	words := int((bytes + 7) / 8)
	if g.buf.len < words {
		g.buf = []u64{len: words}
	}
	if g.get(g.counter, pdh_fmt_double, &bytes, &count, g.buf.data) != 0 {
		return none
	}
	items := unsafe { &PdhItem(g.buf.data) }
	mut names := []string{cap: int(count)}
	mut values := []f64{cap: int(count)}
	for i in 0 .. int(count) {
		item := unsafe { items[i] }
		names << unsafe { string_from_wide(item.name) }
		values << item.value
	}
	return busiest_engine(C.GetCurrentProcessId(), names, values)
}

// close releases the query and its counter; sample answers none afterwards.
pub fn (mut g GpuUsage) close() {
	if g.query != unsafe { nil } {
		g.close_q(g.query)
		g.query = unsafe { nil }
	}
}

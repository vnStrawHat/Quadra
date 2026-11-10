// Giving memory back after a burst (specs/12, Memory). The collector keeps the heap a burst
// grew it to, and it collects only when allocating, so an app that goes quiet keeps that heap
// committed and resident. A check armed by drawn frames looks again every trim_check_ms;
// once a whole period allocated little and the heap grew by trim_min_growth since the last
// trim, it collects and returns the free blocks to the OS (decommits them). Nothing is armed
// while the heap stays below that, or while nothing is drawn, so idle stays free.
module core

import quadra.platform

fn C.GC_gcollect_and_unmap()

const trim_check_ms = 2000
// A period that allocated less than this is quiet (a blinking caret and a clock tick a
// second allocate about 0.5 MiB).
const trim_quiet_bytes = usize(4 << 20)
// Worth two collections: the heap grew this much since the last trim. Idle garbage between
// two ordinary collections stays below it, so a quiet app is not trimmed over and over.
const trim_min_growth = usize(8 << 20)

struct Trim {
mut:
	timer platform.TimerId
	total usize // the collector's allocation count when the period began
	heap  usize // the mapped heap right after the last trim
}

// arm_trim starts a check period after a frame, once the heap has grown enough to be worth
// it and none is running: an app whose heap stays put runs no timer for it.
fn (mut app App) arm_trim() {
	if app.trim.timer != 0 {
		return
	}
	u := gc_heap_usage()
	if u.heap_size < app.trim.heap + trim_min_growth {
		return
	}
	app.trim.total = u.total_bytes
	id := app.plat.set_timer(trim_check_ms, false)
	app.trim.timer = id
	app.timers[id] = TimerEntry{
		cb: check_trim
	}
}

fn check_trim(mut app App) {
	app.trim.timer = 0
	u := gc_heap_usage()
	if u.total_bytes - app.trim.total >= trim_quiet_bytes {
		app.arm_trim() // still busy
		return
	}
	if u.heap_size >= app.trim.heap + trim_min_growth {
		app.trim_now()
	}
}

// trim_now collects and returns the free heap blocks to the OS. Apps may call it after
// dropping large data (closing a tab with a long history). It needs the collector built with
// USE_MUNMAP (tools/v.sh); without it only the collection happens.
pub fn (mut app App) trim_now() {
	// The collector unmaps blocks that were already free when a collection began, so the
	// blocks the first collection frees go back to the OS in the second.
	C.GC_gcollect_and_unmap()
	C.GC_gcollect_and_unmap()
	app.trim.heap = gc_heap_usage().heap_size
}

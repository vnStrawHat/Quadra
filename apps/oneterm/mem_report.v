// --mem (docs/benchmarks.md, OneTerm memory): once a second, a `mem: {json}` line on stderr
// with the process's memory, the collector's heap, the GPU resources, the terminals' storage
// and each child process (console hosts, shells); the loaded modules once, as `mem-module:`
// lines. bench/mem/run.sh reads them. --tabs <n> opens n tabs at start, for the same runs.
module main

import json
import time
import quadra.core
import quadra.core.life
import quadra.native.sys

pub struct MemProbe {
	on   bool
	tabs int
mut:
	since   i64
	modules bool
}

// open_probe_tabs opens the extra tabs of --tabs, after the first.
fn (mut o OneTerm) open_probe_tabs(mut cx core.ViewCx[OneTerm]) {
	for _ in 1 .. o.mem.tabs {
		o.new_tab(o.first_shell, '', mut cx)
	}
	o.mem.since = time.now().unix_milli()
}

fn (mut o OneTerm) tick_mem(mut cx core.ViewCx[OneTerm]) {
	if o.mem.on && o.mem.since > 0 {
		o.write_mem_report(mut cx)
	}
}

struct MemReport {
mut:
	t           f64
	private_ws  u64
	working_set u64
	commit      u64
	peak_ws     u64
	gc_heap     u64 // the collector's heap: live + free
	gc_free     u64
	gc_unmapped u64 // returned to the OS
	gc_total    u64 // allocated since start
	closures    int // closures alive (V frees none on its own)
	gpu_bytes   i64 // atlas pages and instance buffers the app created
	heap_alloc  u64 // the process heaps (C runtime, DirectWrite, D3D, system DLLs)
	heap_commit u64
	terminals   int
	grid_bytes  i64 // visible grids, main and alternate
	history     int // scrollback lines, all terminals
	hist_bytes  i64
	children    []ChildMem
}

struct ChildMem {
	pid         u32
	name        string
	private_ws  u64
	working_set u64
	commit      u64
}

fn (mut o OneTerm) write_mem_report(mut cx core.ViewCx[OneTerm]) {
	p := sys.process_counters()
	gc := gc_heap_usage()
	mut r := MemReport{
		t:           f64(time.now().unix_milli() - o.mem.since) / 1000
		private_ws:  p.private_ws
		working_set: p.working_set
		commit:      p.commit
		peak_ws:     p.peak
		gc_heap:     u64(gc.heap_size)
		gc_free:     u64(gc.free_bytes)
		gc_unmapped: u64(gc.unmapped_bytes)
		gc_total:    u64(gc.total_bytes)
		closures:    life.live_closures()
		gpu_bytes:   cx.app.device.stats().resident_bytes
	}
	r.heap_alloc, r.heap_commit = sys.native_heaps()
	for tab in o.tabs {
		for s in tab.spaces {
			if s.view.id == 0 {
				continue
			}
			t := cx.peek(s.view.model())
			if t.screen == unsafe { nil } {
				continue
			}
			r.terminals++
			r.grid_bytes += t.screen.grid_bytes()
			r.history += t.screen.scrollback.len()
			r.hist_bytes += t.screen.scrollback.bytes()
		}
	}
	for c in sys.descendant_processes() {
		m := sys.memory_of(c.pid) or { continue }
		r.children << ChildMem{c.pid, c.name, m.private_ws, m.working_set, m.commit}
	}
	line := json.encode(r)
	eprintln('mem: ${line}')
	if !o.mem.modules && r.t >= 5 { // the DLL list once, after start-up settled
		o.mem.modules = true
		for m in sys.loaded_modules() {
			eprintln('mem-module: ${m.size} ${m.name}')
		}
	}
}

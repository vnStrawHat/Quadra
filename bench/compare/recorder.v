module main

import os
import time
import quadra.core
import quadra.ui

// Recorder times frames and says on standard output how the run goes: "start" after the
// warm-up frames, then "done {...}" with the frame intervals, as the runner expects.
struct Recorder {
mut:
	warmup    int
	frames    int
	seen      int
	last      u64
	started   u64
	intervals []u64
	phases    bool // BENCH_PHASES=1: also print the average time of each frame phase
	sum       [6]i64
	bsum      [4]u64
	bytes0    u64
	gc0       u64
}

fn env_int(name string, fallback int) int {
	v := os.getenv(name)
	return if v == '' { fallback } else { v.int() }
}

fn new_recorder() Recorder {
	C.GC_start_performance_measurement()
	now := time.sys_mono_now()
	return Recorder{
		warmup:  env_int('BENCH_WARMUP', 120)
		frames:  env_int('BENCH_FRAMES', 600)
		last:    now
		started: now
		phases:  os.getenv('BENCH_PHASES') != ''
	}
}

fn say(s string) {
	println(s)
	flush_stdout()
}

// tick counts a frame, and returns whether the run wants another.
fn (mut r Recorder) tick(w &core.Window) bool {
	now := time.sys_mono_now()
	if r.phases && r.seen > r.warmup {
		for k in 0 .. 4 {
			r.sum[k] += w.stats.phase_us[k]
		}
		r.sum[4] += w.stats.last_frame_us
		for k in 0 .. 4 {
			r.bsum[k] += w.stats.phase_bytes[k]
		}
	}
	r.seen++
	if r.seen == r.warmup {
		say('start')
		r.started = now
		r.bytes0 = u64(C.GC_get_total_bytes())
		r.gc0 = u64(C.GC_get_full_gc_total_time())
	} else if r.seen > r.warmup && r.seen <= r.warmup + r.frames {
		r.intervals << (now - r.last) / 1000
		if r.intervals.len == r.frames {
			r.intervals.sort()
			n := r.frames
			if r.phases {
				avg := r.sum[..5].map(f64(it) / n)
				gc := gc_heap_usage()
				kb := r.bsum[..].map(it / u64(n) / 1024)
				say('phase KB: render ${kb[0]} layout ${kb[1]} prepaint ${kb[2]} paint ${kb[3]}')
				say('phases us: render ${avg[0]:.0f} layout ${avg[1]:.0f} prepaint ${avg[2]:.0f} paint ${avg[3]:.0f} frame ${avg[4]:.0f}; gc heap ${gc.heap_size / 1048576} MB, ${C.GC_get_gc_no()} collections (${(u64(C.GC_get_full_gc_total_time()) - r.gc0) / u64(n)} ms a frame on average... total ${u64(C.GC_get_full_gc_total_time()) - r.gc0} ms), ${(u64(C.GC_get_total_bytes()) - r.bytes0) / u64(n) / 1024} KB allocated a frame, Div ${sizeof(ui.Div)} B, Text ${sizeof(ui.TextEl)} B')
			}
			say('done {"frames":${n},"wallUs":${(now - r.started) / 1000},"p50Us":${r.intervals[n / 2]},"p99Us":${r.intervals[n * 99 / 100]},"maxUs":${r.intervals[n - 1]}}')
		}
	}
	r.last = now
	return r.seen <= r.warmup + r.frames
}

fn C.GC_get_gc_no() usize
fn C.GC_get_total_bytes() usize
fn C.GC_get_full_gc_total_time() u32
fn C.GC_start_performance_measurement()

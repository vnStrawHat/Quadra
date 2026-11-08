module main

import os
import time

#flag -lpsapi
#include <windows.h>
#include <psapi.h>
#flag -luser32
#flag -ladvapi32

fn C.OpenProcess(access u32, inherit int, pid u32) voidptr
fn C.CloseHandle(h voidptr) int
fn C.GetProcessTimes(h voidptr, created &u64, exited &u64, kernel &u64, user &u64) int
fn C.QueryProcessCycleTime(h voidptr, cycles &u64) int
fn C.QueryThreadCycleTime(h voidptr, cycles &u64) int
fn C.GetCurrentThread() voidptr
fn C.GetProcessMemoryInfo(h voidptr, c voidptr, cb u32) int
fn C.EnumWindows(f voidptr, lparam voidptr) int
fn C.GetWindowThreadProcessId(hwnd voidptr, pid &u32) u32
fn C.IsWindowVisible(hwnd voidptr) int
fn C.IsIconic(hwnd voidptr) int

// PROCESS_MEMORY_COUNTERS_EX2, x64 layout.
struct MemoryCounters {
mut:
	cb                    u32
	page_fault_count      u32
	peak_working_set_size usize
	working_set_size      usize
	quota_peak_paged      usize
	quota_paged           usize
	quota_peak_nonpaged   usize
	quota_nonpaged        usize
	pagefile_usage        usize
	peak_pagefile_usage   usize
	private_usage         usize
	private_working_set   usize
	shared_commit         u64
}

struct Usage {
	cpu_100ns  u64 // CPU time of every thread of the process, from its cycle count
	private_ws u64 // private working set: what Task Manager shows as the process's memory
	peak_ws    u64
}

@[heap]
struct Proc {
mut:
	p       &os.Process = unsafe { nil }
	pid     u32
	handle  voidptr
	buf     string
	stopped bool
}

fn start(path string, scene string) !&Proc {
	mut p := os.new_process(path)
	p.set_args([scene])
	p.set_redirect_stdio()
	p.run()
	if p.pid <= 0 {
		return error('cannot start ${path}')
	}
	h := C.OpenProcess(0x1000 | 0x0010, 0, u32(p.pid)) // QUERY_LIMITED_INFORMATION | VM_READ
	return &Proc{
		p:      p
		pid:    u32(p.pid)
		handle: h
	}
}

// wait_line reads standard output until a line starting with prefix (3 minutes at most).
fn (mut p Proc) wait_line(prefix string) !string {
	deadline := time.now().add(3 * time.minute)
	for time.now() < deadline {
		for p.buf.contains('\n') {
			line := p.buf.all_before('\n').trim_space()
			p.buf = p.buf.all_after('\n')
			if line.starts_with(prefix) {
				return line
			}
		}
		if !p.p.is_alive() {
			return error('exited before saying "${prefix}"')
		}
		p.buf += p.p.stdout_read()
	}
	return error('did not say "${prefix}"')
}

fn (mut p Proc) stop() {
	p.stopped = true
	p.p.signal_kill()
	p.p.wait()
	p.p.close()
	C.CloseHandle(p.handle)
}

fn (p &Proc) usage() Usage {
	return usage_of(p.handle)
}

fn usage_of(h voidptr) Usage {
	// GetProcessTimes counts whole clock ticks (15.6 ms) per thread, too coarse for a frame:
	// the cycle count is exact, and converted with the rate measured by cycles_per_100ns.
	mut cycles := u64(0)
	C.QueryProcessCycleTime(h, &cycles)
	mut m := MemoryCounters{
		cb: u32(sizeof(MemoryCounters))
	}
	C.GetProcessMemoryInfo(h, &m, m.cb)
	return Usage{u64(f64(cycles) / cycle_rate()), u64(m.private_working_set), u64(m.peak_working_set_size)}
}

// sample_peak samples the private working set every 10 ms until the run stops.
fn sample_peak(h voidptr, stopped &bool) u64 {
	mut peak := u64(0)
	for !*stopped {
		ws := usage_of(h).private_ws
		if ws > peak {
			peak = ws
		}
		time.sleep(10 * time.millisecond)
	}
	return peak
}

struct Find {
mut:
	pid   u32
	found bool
}

fn enum_cb(hwnd voidptr, lparam voidptr) int {
	mut f := unsafe { &Find(lparam) }
	mut pid := u32(0)
	C.GetWindowThreadProcessId(hwnd, &pid)
	if pid == f.pid && C.IsWindowVisible(hwnd) != 0 && C.IsIconic(hwnd) == 0 {
		f.found = true
		return 0
	}
	return 1
}

// wait_window returns the time from began until a visible window of pid exists (0: none
// within a minute).
fn wait_window(pid u32, began u64) u64 {
	mut f := &Find{
		pid: pid
	}
	for time.sys_mono_now() - began < 60 * u64(time.second) {
		C.EnumWindows(voidptr(enum_cb), voidptr(f))
		if f.found {
			return time.sys_mono_now() - began
		}
		time.sleep(time.millisecond)
	}
	return 0
}

fn machine() string {
	cpu := os.execute('wmic cpu get name').output.split_into_lines()
	return if cpu.len > 1 { cpu[1].trim_space() } else { 'unknown' }
}

// cycle_rate is how many cycles the CPU's cycle counter advances in 100 ns, measured once at
// start by spinning this thread for 200 ms (the counter runs at a constant rate).
const cycle_rate_v = measure_cycle_rate()

fn cycle_rate() f64 {
	return cycle_rate_v
}

fn measure_cycle_rate() f64 {
	mut c0, mut c1 := u64(0), u64(0)
	th := C.GetCurrentThread()
	sw := time.new_stopwatch()
	C.QueryThreadCycleTime(th, &c0)
	for sw.elapsed() < 200 * time.millisecond {}
	C.QueryThreadCycleTime(th, &c1)
	return f64(c1 - c0) / (f64(sw.elapsed().nanoseconds()) / 100)
}

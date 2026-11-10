// Memory gates (specs/14, docs/benchmarks.md "OneTerm idle allocation"): idle ticks and the
// typical flows, repeated, leave the live closure count where it was and the dispatch detector
// quiet; idle allocation and the heap after a vault unlock are printed for the benchmarks.
// Run with `tools/v.sh -cc msvc -stats test apps/oneterm/leak_gate_test.v` for the numbers.
module main

import os
import update
import vault
import quadra.core
import quadra.core.life
import quadra.kit.term
import quadra.platform
import quadra.testing

struct GateReleases {}

fn (f &GateReleases) fetch_releases(_ string, _ ?string) !update.ReleaseResponse {
	return update.ReleaseResponse{}
}

fn (f &GateReleases) download_to_file(_ string, _ string, _ u64) ! {
	return error('no download in this test')
}

const gate_password = 'correct-horse'

fn gate_rig(name string, with_vault bool) !(&testing.Harness, core.View[OneTerm]) {
	dir := '.tmp/oneterm-gate-${name}'
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	mut h := testing.new_harness()!
	_, mut one := prepare(mut h.app, dir)!
	mut fake := &term.FakePty{}
	one.spawn = fake.spawner()
	one.update_client = update.ReleaseClient(&GateReleases{})
	if with_vault {
		vault.init(dir, gate_password, false)!
		one.vault = vault.load_at_start(dir, false)
	}
	v := h.open(one, width: 1000, height: 700)!
	h.frame()
	return h, v
}

fn allocated() u64 {
	return u64(gc_heap_usage().total_bytes)
}

fn unlock(mut h testing.Harness) ! {
	h.click('lock-input') or {} // focused at start; after a lock from a key, click it
	h.type_text(gate_password)
	h.press('enter')!
	h.wait_tasks()!
	h.frame()
}

// round runs the flows once: idle ticks, tabs and spaces, menus, the settings window, an
// update check, About, and the lock screen.
fn round(mut h testing.Harness, v core.View[OneTerm]) ! {
	for _ in 0 .. 3 {
		h.advance(1000)
		h.wait_tasks()!
	}
	h.press('ctrl-t')!
	h.frame()
	h.press('ctrl-shift-right')!
	h.press('ctrl-shift-down')!
	h.frame()
	h.press('ctrl-shift-x')!
	h.press('ctrl-shift-x')!
	h.press('ctrl-w')!
	h.frame()
	for menu in ['titlebar-menu', 'new-tab'] {
		h.click(menu)!
		h.press('escape')!
	}
	h.press('ctrl-,')!
	h.frame()
	main_win := h.win
	h.win = h.app.windows[h.app.windows.len - 1].id
	h.inject(platform.CloseRequested{})
	h.win = main_win
	h.frame()
	mut cx := core.Cx{
		app: unsafe { h.app }
		win: h.window()
	}
	cx.update_view(v, fn (mut o OneTerm, mut cx core.ViewCx[OneTerm]) {
		o.check_for_updates(mut cx)
		open_about(cx.self_model(), mut cx.Cx)
	})
	h.wait_tasks()!
	h.frame()
	h.click('about-close')!
	h.press('ctrl-shift-l')! // the lock screen, then the password
	h.frame()
	unlock(mut h)!
	h.frame()
	h.frame()
}

fn test_flows_leak_no_closures() {
	mut h, v := gate_rig('flows', true)!
	unlock(mut h)!
	round(mut h, v)! // warm up: first-time caches, views and windows
	c0 := life.live_closures()
	for _ in 0 .. 3 {
		round(mut h, v)!
	}
	grown := life.live_closures() - c0
	eprintln('oneterm: closures +${grown} over 3 rounds of flows')
	assert grown <= 0
	assert h.app.closure_leaks() == []
}

fn test_idle_allocation_and_closures() {
	mut h, v := gate_rig('idle', false)!
	for _ in 0 .. 10 {
		h.advance(1000)
		h.wait_tasks()!
	}
	c0 := life.live_closures()
	mut a := allocated()
	for _ in 0 .. 20 {
		h.advance(1000)
		h.wait_tasks()!
	}
	idle := (allocated() - a) / 20
	closures := life.live_closures() - c0
	mut cx := core.Cx{
		app:    unsafe { h.app }
		entity: v.id
	}
	a = allocated()
	for _ in 0 .. 20 {
		cx.notify()
		h.frame()
	}
	root := (allocated() - a) / 20
	eprintln('oneterm idle: ${idle / 1024} KB/s allocated, a root render ${root / 1024} KB, closures +${closures} in 20 s')
	assert closures <= 0
	assert h.app.closure_leaks() == []
}

fn heap_mb() string {
	u := gc_heap_usage()
	return 'heap ${u.heap_size >> 20} MB, free ${u.free_bytes >> 20} MB, unmapped ${u.unmapped_bytes >> 20} MB'
}

fn test_unlock_memory_comes_back() {
	mut h, _ := gate_rig('unlock', true)!
	gc_collect()
	locked := heap_mb()
	unlock(mut h)!
	for _ in 0 .. 4 {
		h.advance(1000)
	}
	gc_collect()
	eprintln('oneterm vault: locked ${locked}; unlocked +4 s ${heap_mb()}')
	assert h.app.closure_leaks() == []
}

// The workspace shell in a headless window, on fake terminals (specs/22).
module main

import os
import time
import vault
import quadra.core
import quadra.kit.term
import quadra.platform
import quadra.prim
import quadra.testing

@[heap]
struct Fakes {
mut:
	list []&term.FakePty
}

fn rig(name string, width f32) !(&testing.Harness, core.View[OneTerm], &Fakes) {
	dir := '.tmp/oneterm-ws-${name}'
	os.rmdir_all(dir) or {}
	mut h := testing.new_harness()!
	_, mut one := prepare(mut h.app, dir)!
	mut fakes := &Fakes{}
	one.spawn = fn [mut fakes] (opts platform.PtyOptions) !platform.Pty {
		mut f := &term.FakePty{}
		fakes.list << f
		return f.spawner()(opts)!
	}
	v := h.open(one, width: width, height: 700)!
	h.frame()
	return h, v, fakes
}

fn st(h &testing.Harness, v core.View[OneTerm]) &OneTerm {
	cx := core.Cx{
		app: unsafe { h.app }
	}
	return cx.peek(v.model())
}

fn commit_now(mut h testing.Harness, mut o OneTerm) {
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	o.apply_and_persist(mut cx)
	h.frame()
}

fn test_the_app_menu_has_every_row_and_submenus_check() {
	mut h, _, _ := rig('menu', 1000)!
	h.click('titlebar-menu')!
	h.frame()
	for label in ['About', 'Appearance', 'Theme', 'Show FPS Monitor', 'Settings...', 'Lock OneTerm',
		'Quit'] {
		assert h.has_text(label), label
	}
	r := h.text_bounds('Appearance') or { panic('no Appearance row') }
	h.move_to(r.x + 4, r.y + 4)
	h.advance(400)
	h.frame()
	assert h.has_text('Light') && h.has_text('Dark')
}

fn test_the_right_dock_toggles_and_keeps_unknown_keys() {
	mut h, v, _ := rig('dock', 1600)!
	o := st(h, v)
	assert h.find('right-dock') == none
	h.click('tabs-dock')!
	h.frame()
	dock := h.find('right-dock') or { panic('no dock') }
	assert dock.width == 480
	assert h.has_text('Session') && h.has_text('SFTP Browser')
	path := '${o.dir}/docks.json'
	text := os.read_file(path)!
	assert text.contains('right_dock')
	os.write_file(path, text.replace_once('{', '{"future":7,'))!
	h.click('tabs-dock')!
	h.frame()
	assert h.find('right-dock') == none
	kept := os.read_file(path)!
	assert kept.contains('future') && kept.contains('"open":false')
}

fn test_the_status_bar_survives_a_narrow_window() {
	mut h, v, _ := rig('narrow', 1000)!
	o := st(h, v)
	assert h.find('statusbar') != none
	assert h.has_text(o.clock)
	for _ in 0 .. 3 { // sampled every second tick, off the UI thread
		h.advance(1100)
		h.frame()
		h.wait_tasks()!
		h.frame()
	}
	assert h.find('status-res') != none
	assert h.texts().any(it.starts_with('CPU '))
	h.resize(640, 480)
	h.frame()
	bar := h.find('statusbar') or { panic('no bar') }
	assert bar.width == 640
	path, branch := divide_centre(640, 2, 300, 200)
	assert path + branch <= 640 && branch >= 40
}

fn test_broadcast_reaches_members_only() {
	mut h, v, fakes := rig('channels', 1000)!
	h.press('ctrl-t')!
	h.frame()
	h.press('ctrl-t')!
	h.frame()
	mut o := st(h, v)
	assert fakes.list.len == 3
	o.tabs[0].spaces[0].channel = 'A'
	o.tabs[2].spaces[0].channel = 'A'
	h.type_text('x')
	h.frame()
	assert fakes.list[2].written.bytestr().contains('x')
	assert fakes.list[0].written.bytestr().contains('x')
	assert !fakes.list[1].written.bytestr().contains('x')
	// what the terminal answers by itself stays in its own PTY
	before := fakes.list[0].written.len
	o.tabs[2].spaces[0].pty.write('\x1b[<0;1;1M'.bytes())!
	assert fakes.list[0].written.len == before
}

fn test_a_tab_drags_into_an_empty_space() {
	mut h, v, _ := rig('drag', 1000)!
	h.press('ctrl-t')!
	h.frame()
	mut o := st(h, v)
	o.tabs[1].name = 'second'
	h.click('tabs-${o.tabs[0].id}')!
	h.frame()
	h.press('ctrl-shift-right')!
	h.frame()
	assert o.tabs[0].spaces.len == 2
	target := o.tabs[0].spaces.filter(it.view.id == 0)
	assert target.len == 1
	to := h.find('empty-${target[0].id}') or { panic('no empty space') }
	from := h.text_bounds('second') or { panic('no second tab') }
	h.drag(from.x + 4, from.y + 4, to.x + to.width / 2, to.y + to.height / 2, true)
	h.frame()
	assert o.tabs.len == 1
	assert o.tabs[0].spaces.all(it.view.id != 0)
}

fn test_the_lock_screen_checks_the_password() {
	dir := '.tmp/oneterm-ws-lock'
	os.rmdir_all(dir) or {}
	mut h := testing.new_harness()!
	_, mut one := prepare(mut h.app, dir)!
	mut fake := &term.FakePty{}
	one.spawn = fake.spawner()
	vault.init(dir, 'correct-horse', false)!
	one.vault = vault.load_at_start(dir, false)
	v := h.open(one, width: 1000, height: 700)!
	h.frame()
	h.frame()
	o := st(h, v)
	assert o.vault.is_screen_locked() && h.has_text('OneTerm is locked')
	h.type_text('bad-password')
	h.press('enter')!
	h.wait_tasks()!
	h.frame()
	assert o.vault.is_screen_locked() && h.has_text('Wrong master password.')
	h.type_text('correct-horse')
	h.press('enter')!
	h.wait_tasks()!
	h.frame()
	assert !o.vault.is_screen_locked() && !h.has_text('OneTerm is locked')
}

fn test_idle_time_locks() {
	dir := '.tmp/oneterm-ws-idle'
	os.rmdir_all(dir) or {}
	mut h := testing.new_harness()!
	_, mut one := prepare(mut h.app, dir)!
	mut fake := &term.FakePty{}
	one.spawn = fake.spawner()
	vault.init(dir, 'correct-horse', false)!
	one.vault = vault.load_at_start(dir, false)
	mut state := one.vault
	job := state.begin_unlock_screen('correct-horse')!
	state.apply(job.run())!
	v := h.open(one, width: 1000, height: 700)!
	h.frame()
	mut o := st(h, v)
	assert !o.vault.is_screen_locked() && o.vault.phase() == .unlocked
	o.last_input = time.now().add(-2 * time.hour)
	h.advance(1100)
	h.frame()
	assert o.vault.is_screen_locked()
}

fn test_fps_hud_and_rail() {
	mut h, v, _ := rig('rail', 1000)!
	mut o := st(h, v)
	o.ui.show_fps = true
	commit_now(mut h, mut o)
	assert h.find('fps-hud') != none
	assert h.has_text('Null device (software)') // the headless device, through core
	o.ui.show_fps = false
	o.ui.vertical_tabs.enabled = true
	commit_now(mut h, mut o)
	assert h.find('fps-hud') == none
	assert h.find('tabs') == none
	rail := h.find('rail') or { panic('no rail') }
	assert rail.width == 220
	handle := h.find('rail-handle') or { panic('no handle') }
	h.drag(handle.x + 1, handle.y + 40, handle.x + 61, handle.y + 40, true)
	h.frame()
	assert o.ui.vertical_tabs.width > 260
	assert load_ui_config(o.dir).vertical_tabs.width == o.ui.vertical_tabs.width
	o.ui.vertical_tabs.collapsed = true
	commit_now(mut h, mut o)
	collapsed := h.find('rail') or { panic('no rail') }
	assert collapsed.width == 44
}

fn test_the_window_opens_at_the_original_size_centred() {
	mut h := testing.new_harness()!
	opts, _ := prepare(mut h.app, '.tmp/oneterm-ws-winsize')!
	assert opts.width == 1600 && opts.height == 1000 && opts.centered && opts.fit == f32(0.85)
	assert opts.min_width == 640 && opts.min_height == 480 && !opts.placement.valid
	s := platform.fit_size(opts.width, opts.height, prim.size(1366, 768), opts.fit)
	assert s.width == f32(1366) * 0.85 && s.height == f32(768) * 0.85
	big := platform.fit_size(opts.width, opts.height, prim.size(2560, 1440), opts.fit)
	assert big.width == 1600 && big.height == 1000
}

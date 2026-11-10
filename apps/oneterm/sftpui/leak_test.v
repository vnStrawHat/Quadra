// Closure leak gate (specs/14): the browser's flows on the fake client, repeated, leave the
// live closure count where it was and the dispatch detector quiet.
module sftpui

import os
import quadra.core
import quadra.core.life
import quadra.kit
import quadra.testing
import quadra.ui

@[heap; markused]
pub struct LeakRoot {
	panel core.View[SftpPanel]
}

pub fn (mut r LeakRoot) render(mut cx core.ViewCx[LeakRoot]) core.Element {
	return ui.div().size_full().child(core.view_element(r.panel.any()))
}

fn leak_rig() !(&testing.Harness, core.View[SftpPanel]) {
	dir := os.real_path('.tmp') + '/sftpui_leaks'
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	mut h := testing.new_harness(width: 900, height: 700)!
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	kit.install(mut h.app)!
	v := new_panel(mut cx, edit_dir: '${dir}/edit', local_dir: dir)
	h.open(LeakRoot{ panel: v },
		width:  900
		height: 700
	)!
	mut f := &FakeClient{}
	f.add_dir('/home/user', [fake_entry('/home/user', 'docs', true, 0, 100),
		fake_entry('/home/user', 'notes.txt', false, 2048, 200)])
	f.add_dir('/home/user/docs', [
		fake_entry('/home/user/docs', 'inner.txt', false, 1, 1),
	])
	set_client(mut cx, v, f, 1)
	h.wait_tasks()!
	return h, v
}

fn row_center(h &testing.Harness, i int) (f32, f32) {
	r := h.find('sftp-table-row${i}') or { panic('no row ${i}') }
	return r.x + r.width / 2, r.y + r.height / 2
}

fn browse(mut h testing.Harness, v core.View[SftpPanel]) ! {
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	for _ in 0 .. 3 {
		h.click('sftp-table-h-name')! // descending, ascending, default
	}
	x, y := row_center(h, 0)
	h.click_at(x, y)
	h.click_at(x, y) // into docs
	h.wait_tasks()!
	h.click('sftp-back')!
	h.wait_tasks()!
	x1, y1 := row_center(h, 1)
	h.click_at(x1, y1)
	run_action(mut cx, v, .properties)
	h.wait_tasks()!
	h.press('escape')!
	run_action(mut cx, v, .delete) // asks first; no
	h.frame()
	h.press('escape')!
	h.frame()
	h.frame()
}

fn test_browsing_leaks_no_closures() {
	mut h, v := leak_rig()!
	browse(mut h, v)!
	c0 := life.live_closures()
	for _ in 0 .. 3 {
		browse(mut h, v)!
	}
	grown := life.live_closures() - c0
	eprintln('sftp: closures +${grown} over 3 rounds of browsing')
	assert grown <= 0
	assert h.app.closure_leaks() == []
}

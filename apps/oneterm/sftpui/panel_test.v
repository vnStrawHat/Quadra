// The panel in a headless window, on the fake client (specs/27).
module sftpui

import os
import quadra.core
import quadra.kit
import quadra.platform
import quadra.testing
import quadra.ui

@[heap; markused]
pub struct Root {
	panel core.View[SftpPanel]
}

pub fn (mut r Root) render(mut cx core.ViewCx[Root]) core.Element {
	return ui.div().size_full().child(core.view_element(r.panel.any()))
}

fn rig(name string, width f32) !(&testing.Harness, core.View[SftpPanel], &FakeClient, string) {
	dir := os.real_path('.tmp') + '/sftpui_${name}'
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir)!
	mut h := testing.new_harness(width: width, height: 700)!
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	kit.install(mut h.app)!
	v := new_panel(mut cx, edit_dir: '${dir}/edit', local_dir: dir)
	h.open(Root{ panel: v },
		width:  width
		height: 700
	)!
	mut f := &FakeClient{}
	f.add_dir('/home/user', [fake_entry('/home/user', 'docs', true, 0, 100),
		fake_entry('/home/user', 'notes.txt', false, 2048, 200),
		fake_entry('/home/user', 'zeta.log', false, 10, 300)])
	f.add_dir('/home/user/docs', [
		fake_entry('/home/user/docs', 'inner.txt', false, 1, 1),
	])
	f.add_dir('/home', [fake_entry('/home', 'user', true, 0, 1)])
	return h, v, f, dir
}

fn connect(mut h testing.Harness, v core.View[SftpPanel], f &FakeClient) ! {
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	set_client(mut cx, v, f, 1)
	h.wait_tasks()!
}

fn panel_of(h &testing.Harness, v core.View[SftpPanel]) &SftpPanel {
	cx := core.Cx{
		app: unsafe { h.app }
	}
	return cx.peek(v.model())
}

fn table_row_center(h &testing.Harness, id string, i int) (f32, f32) {
	r := h.find('${id}-row${i}') or { panic('no row ${i}') }
	return r.x + r.width / 2, r.y + r.height / 2
}

fn right_click(mut h testing.Harness, x f32, y f32) {
	h.move_to(x, y)
	h.inject(platform.MouseEvent{
		action: .down
		button: .right
		x:      x
		y:      y
	})
	h.inject(platform.MouseEvent{
		action: .up
		button: .right
		x:      x
		y:      y
	})
}

fn test_no_connection_shows_the_empty_state() {
	mut h, _, _, _ := rig('empty', 600)!
	assert h.has_text('No SFTP connection.')
}

fn test_listing_resolves_home_and_shows_columns() {
	mut h, v, f, _ := rig('list', 600)!
	connect(mut h, v, f)!
	p := panel_of(h, v)
	assert p.active.cwd == '/home/user'
	assert h.has_text('Name') && h.has_text('Date Modified') && h.has_text('Permissions')
	assert h.has_text('notes.txt') && h.has_text('docs')
	assert h.has_text('-rw-r--r-- (0644)')
	assert p.active.entries[0].name == 'docs' // folders first
}

fn test_a_header_click_cycles_the_sort() {
	mut h, v, f, _ := rig('sort', 600)!
	connect(mut h, v, f)!
	h.click('sftp-table-h-name')! // descending
	h.frame()
	mut p := panel_of(h, v)
	assert p.active.entries.map(it.name)[1..] == ['zeta.log', 'notes.txt']
	h.click('sftp-table-h-name')! // ascending
	h.frame()
	assert panel_of(h, v).active.entries.map(it.name)[1..] == ['notes.txt', 'zeta.log']
	h.click('sftp-table-h-name')! // back to the default
	assert panel_of(h, v).active.sort.on == false
}

fn test_navigation_into_a_folder_and_back() {
	mut h, v, f, _ := rig('nav', 600)!
	connect(mut h, v, f)!
	x, y := table_row_center(h, 'sftp-table', 0)
	h.click_at(x, y)
	h.click_at(x, y) // double click
	h.wait_tasks()!
	assert panel_of(h, v).active.cwd == '/home/user/docs'
	assert h.has_text('inner.txt')
	h.click('sftp-back')!
	h.wait_tasks()!
	assert panel_of(h, v).active.cwd == '/home/user'
}

fn test_a_bad_path_marks_the_box_and_a_failed_listing_keeps_rows() {
	mut h, v, f, _ := rig('badpath', 600)!
	connect(mut h, v, f)!
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	cx.update_view(v, fn (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		p.goto_path('/nope', mut cx)
	})
	h.wait_tasks()!
	assert panel_of(h, v).active.path_error
	unsafe {
		f.fail_read['/home/user'] = 'permission denied'
	}
	h.click('sftp-refresh')!
	h.wait_tasks()!
	assert h.has_text('Could not open "/home/user": permission denied')
	assert h.has_text('notes.txt') // previous rows stay
}

fn test_an_os_drop_with_no_connection_says_so() {
	mut h, _, _, dir := rig('dropnone', 600)!
	r := h.find('sftp-panel') or { panic('no panel') }
	h.inject(platform.FileDropEvent{['${dir}/x.txt'], r.x + 10, r.y + 10})
	h.frame()
	assert h.has_text('No active SFTP connection — dropped files were not uploaded.')
}

fn test_a_late_listing_is_dropped() {
	mut h, v, f, _ := rig('stale', 600)!
	connect(mut h, v, f)!
	unsafe {
		f.slow['/home'] = 200
	}
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	cx.update_view(v, fn (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		p.load_dir('/home', mut cx)
		p.load_dir('/home/user/docs', mut cx)
	})
	h.wait_tasks()!
	assert panel_of(h, v).active.cwd == '/home/user/docs'
	assert h.has_text('inner.txt')
}

fn test_the_context_menu_renames_with_a_validated_name() {
	mut h, v, f, _ := rig('rename', 600)!
	connect(mut h, v, f)!
	x, y := table_row_center(h, 'sftp-table', 1)
	right_click(mut h, x, y)
	assert h.has_text('Edit') && h.has_text('Rename') && h.has_text('Properties')
	h.click('menu-8')! // Rename: 3 + sep + 3 + sep + Rename (index 8)
	assert h.has_text('New name')
	h.type_text('/bad')
	h.press('enter')!
	h.frame()
	assert h.has_text("Name cannot contain '/' or '\\'.")
	assert panel_of(h, v).form_modal != unsafe { nil } // the form stays open
	h.press('ctrl-a')!
	h.type_text('renamed.txt')
	h.click('form-ok')!
	h.wait_tasks()!
	assert 'rename /home/user/notes.txt /home/user/renamed.txt' in f.log
	assert panel_of(h, v).form_modal == unsafe { nil }
	assert h.has_text('Renamed to "renamed.txt".')
}

fn test_delete_asks_first() {
	mut h, v, f, _ := rig('delete', 600)!
	connect(mut h, v, f)!
	x, y := table_row_center(h, 'sftp-table', 2)
	h.click_at(x, y)
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	run_action(mut cx, v, .delete)
	h.frame()
	assert h.has_text('Confirm Delete')
	h.click('ask-yes')!
	h.wait_tasks()!
	assert 'remove /home/user/zeta.log' in f.log
	assert h.has_text('Deleted successfully.')
}

fn test_properties_show_the_stat_result() {
	mut h, v, f, _ := rig('props', 600)!
	connect(mut h, v, f)!
	x, y := table_row_center(h, 'sftp-table', 1)
	h.click_at(x, y)
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	run_action(mut cx, v, .properties)
	h.wait_tasks()!
	assert h.has_text('Owner:') && h.has_text('notes.txt')
	assert h.has_text('2.0 KB (2048 bytes)')
}

fn test_follow_cwd_moves_the_browser() {
	mut h, v, f, _ := rig('follow', 600)!
	connect(mut h, v, f)!
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	follow_cwd(mut cx, v, '/home')
	h.frame()
	h.click('sftp-sync-cwd')!
	h.wait_tasks()!
	assert panel_of(h, v).active.cwd == '/home'
	mut p := panel_of(h, v)
	p.active.follow = true
	follow_cwd(mut cx, v, '/home/user/docs')
	h.wait_tasks()!
	assert panel_of(h, v).active.cwd == '/home/user/docs'
}

fn test_columns_toggle_and_persist() {
	mut h, v, f, dir := rig('persist', 600)!
	connect(mut h, v, f)!
	path := '${dir}/docks.json'
	mut p := panel_of(h, v)
	p.state_path = path
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	cx.update_view(v, fn (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		p.toggle_column(1, mut cx) // Size on
	})
	h.advance(1100)
	assert h.has_text('Size')
	text := os.read_file(path)!
	assert text.contains('"size":true')
	mut h2 := testing.new_harness()!
	mut cx2 := core.Cx{
		app: unsafe { h2.app }
	}
	v2 := new_panel(mut cx2, state_path: path)
	table := cx2.peek(v2.model()).table
	assert cx2.peek(table.model()).cols[1].visible
}

fn test_each_session_keeps_its_folder_and_queue() {
	mut h, v, f, _ := rig('backends', 600)!
	connect(mut h, v, f)!
	mut g := &FakeClient{}
	g.add_dir('/home/user', [fake_entry('/home/user', 'other.dat', false, 1, 1)])
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	set_client(mut cx, v, g, 2)
	h.wait_tasks()!
	assert h.has_text('other.dat') && !h.has_text('notes.txt')
	h.click('sftp-table-h-name')! // sorted descending, on this session only
	set_client(mut cx, v, f, 1)
	h.frame()
	assert h.has_text('notes.txt') && !h.has_text('other.dat')
	assert !panel_of(h, v).active.sort.on
	set_client(mut cx, v, g, 2)
	h.frame()
	assert panel_of(h, v).active.sort.on
	unsafe {
		g.dead = true
	}
	h.advance(600) // the tick purges a closed session
	assert panel_of(h, v).active == unsafe { nil }
	assert h.has_text('No SFTP connection.')
}

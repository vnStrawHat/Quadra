// Upload, download, the queue, the dual-pane layout and Edit, on the fake client.
module sftpui

import os
import time
import quadra.core
import quadra.kit
import quadra.platform
import quadra.testing
import quadra.ui

@[heap; markused]
pub struct FlowRoot {
	panel core.View[SftpPanel]
}

pub fn (mut r FlowRoot) render(mut cx core.ViewCx[FlowRoot]) core.Element {
	return ui.div().size_full().child(core.view_element(r.panel.any()))
}

fn flow_rig(name string) !(&testing.Harness, core.View[SftpPanel], &FakeClient, string) {
	dir := os.real_path('.tmp') + '/sftpui_${name}'
	os.rmdir_all(dir) or {}
	os.mkdir_all(dir + '/local')!
	mut h := testing.new_harness(width: 1000, height: 700)!
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	kit.install(mut h.app)!
	v := new_panel(mut cx, edit_dir: '${dir}/edit', local_dir: '${dir}/local')
	h.open(FlowRoot{ panel: v },
		width:  1000
		height: 700
	)!
	mut f := &FakeClient{}
	f.add_dir('/home/user', [fake_entry('/home/user', 'notes.txt', false, 2048, 200),
		fake_entry('/home/user', '../../evil.txt', false, 3, 5)])
	set_client(mut cx, v, f, 1)
	h.wait_tasks()!
	return h, v, f, dir
}

fn the_panel(h &testing.Harness, v core.View[SftpPanel]) &SftpPanel {
	cx := core.Cx{
		app: unsafe { h.app }
	}
	return cx.peek(v.model())
}

fn select_row(mut h testing.Harness, id string, i int) {
	r := h.find('${id}-row${i}') or { panic('no row') }
	h.click_at(r.x + r.width / 2, r.y + r.height / 2)
}

fn select_named(mut h testing.Harness, v core.View[SftpPanel], name string) {
	p := the_panel(h, v)
	mut idx := 0
	for i, e in p.active.entries {
		if e.name == name {
			idx = i
		}
	}
	select_row(mut h, 'sftp-table', idx)
}

fn act(mut h testing.Harness, v core.View[SftpPanel], a SftpAction) {
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	run_action(mut cx, v, a)
	h.frame()
}

fn write(dir string, name string, text string) string {
	path := '${dir}/${name}'
	os.write_file(path, text) or { panic(err) }
	return path
}

fn upload(mut h testing.Harness, v core.View[SftpPanel], paths []string) ! {
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	cx.update_view(v, fn [paths] (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		p.upload_paths(paths, mut cx)
	})
	h.wait_tasks()!
}

fn test_upload_without_collision_runs_through_the_queue() {
	mut h, v, f, dir := flow_rig('up')!
	path := write(dir, 'new.txt', 'hello')
	upload(mut h, v, [path])!
	assert f.uploaded['/home/user/new.txt'] == 'hello'
	assert h.has_text('Transfers') && h.has_text('0 active, 1 done') && h.has_text('Done')
	h.click('sftp-clear-transfers')!
	assert !h.has_text('Transfers')
}

fn test_a_collision_asks_and_skips() {
	mut h, v, f, dir := flow_rig('coll')!
	a := write(dir, 'notes.txt', 'mine')
	b := write(dir, 'other.txt', 'x')
	upload(mut h, v, [a, b])!
	assert h.has_text('Replace Remote File')
	assert h.has_text('Skip')
	h.click('ask-no')!
	h.wait_tasks()!
	assert f.uploaded['/home/user/other.txt'] == 'x'
	assert '/home/user/notes.txt' !in f.uploaded
}

fn test_replace_writes_the_colliding_file() {
	mut h, v, f, dir := flow_rig('repl')!
	upload(mut h, v, [write(dir, 'notes.txt', 'mine')])!
	h.click('ask-yes')!
	h.wait_tasks()!
	assert f.uploaded['/home/user/notes.txt'] == 'mine'
}

fn test_cancel_stops_a_running_transfer() {
	mut h, v, f, dir := flow_rig('cancel')!
	unsafe {
		f.hang_chunk = true
	}
	path := write(dir, 'big.bin', 'zzz')
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	cx.update_view(v, fn [path] (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		p.upload_paths([path], mut cx)
	})
	h.frame()
	for _ in 0 .. 500 {
		if _ := h.find('sftp-cancel-1') {
			break
		}
		time.sleep(2 * time.millisecond)
		h.frame()
	}
	assert h.has_text('50%')
	h.click('sftp-cancel-1')!
	h.wait_tasks()!
	assert h.has_text('Cancelled')
	assert h.has_text('0 active, 1 done')
}

fn test_download_goes_through_the_save_dialog() {
	mut h, v, f, dir := flow_rig('down')!
	out := '${dir}/saved.txt'
	h.plat.picks << [out]
	select_named(mut h, v, 'notes.txt')
	act(mut h, v, .download)
	h.wait_tasks()!
	assert os.read_file(out)! == 'data of /home/user/notes.txt'
}

fn test_expanded_panes_move_files_both_ways() {
	mut h, v, f, dir := flow_rig('dual')!
	local := '${dir}/local'
	write(local, 'from_local.txt', 'L')
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	set_expanded(mut cx, v, true)
	h.wait_tasks()!
	assert h.has_text('from_local.txt') && h.has_text('Local') && h.has_text('Remote')
	// Double click on a local file uploads it.
	r := h.find('sftp-local-table-row0') or { panic('no local row') }
	h.click_at(r.x + r.width / 2, r.y + r.height / 2)
	h.click_at(r.x + r.width / 2, r.y + r.height / 2)
	h.wait_tasks()!
	assert f.uploaded['/home/user/from_local.txt'] == 'L'
	// A remote row dragged onto the local list downloads into its folder.
	rr := h.find('sftp-table-row1') or { panic('no remote row') }
	target := h.find('sftp-local-table') or { panic('no local table') }
	h.drag(rr.x + 20, rr.y + rr.height / 2, target.x + target.width / 2, target.y +
		target.height / 2, true)
	h.wait_tasks()!
	assert os.read_file('${local}/notes.txt')! == 'data of /home/user/notes.txt'
	// A local row dragged onto the remote list uploads.
	h.wait_tasks()!
	lr := h.find('sftp-local-table-row0') or { panic('no local row') }
	remote := h.find('sftp-table') or { panic('no remote table') }
	f_before := f.log.len
	h.drag(lr.x + 20, lr.y + lr.height / 2, remote.x + remote.width / 2, remote.y +
		remote.height / 2, true)
	h.wait_tasks()!
	assert f.log.len > f_before
}

fn test_os_drop_on_the_remote_list_uploads() {
	mut h, v, f, dir := flow_rig('osdrop')!
	path := write(dir, 'dropped.txt', 'D')
	r := h.find('sftp-table') or { panic('no table') }
	h.inject(platform.FileDropEvent{[path], r.x + 10, r.y + 10})
	h.wait_tasks()!
	assert f.uploaded['/home/user/dropped.txt'] == 'D'
	h.inject(platform.FileDropEvent{[path], f32(2000), f32(2000)}) // outside: not ours
	h.wait_tasks()!
	assert f.log.filter(it.starts_with('upload ')).len == 1
}

fn test_an_unsafe_remote_name_never_escapes_the_local_folder() {
	mut h, v, f, dir := flow_rig('evil')!
	local := '${dir}/local'
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	set_expanded(mut cx, v, true)
	h.wait_tasks()!
	select_named(mut h, v, '../../evil.txt')
	act(mut h, v, .download)
	h.wait_tasks()!
	assert !os.exists('${dir}/evil.txt') && !os.exists('${local}/evil.txt')
	assert !f.log.any(it.starts_with('download '))
	// Edit keeps the working copy inside its own folder, sanitized.
	act(mut h, v, .edit)
	h.wait_tasks()!
	assert h.plat.opened.len == 1
	assert h.plat.opened[0].starts_with('${dir}/edit'.replace('/', os.path_separator))
	assert os.exists(h.plat.opened[0]) && os.file_name(h.plat.opened[0]) == '.._.._evil.txt'
}

fn test_edit_uploads_a_save_after_asking() {
	mut h, v, f, dir := flow_rig('edit')!
	select_named(mut h, v, 'notes.txt')
	act(mut h, v, .edit)
	h.wait_tasks()!
	temp := h.plat.opened[0]
	assert temp.starts_with(dir.replace('/', os.path_separator))
	mut p := the_panel(h, v)
	assert p.edits.len == 1
	// Opening the file is not a save: no prompt until the bytes change.
	h.advance(600)
	h.advance(600)
	assert !h.has_text('Upload change')
	os.write_file(temp, 'edited')!
	h.advance(600)
	h.advance(600)
	assert h.has_text('Upload change')
	h.click('upload-ok')!
	h.wait_tasks()!
	assert f.uploaded['/home/user/notes.txt'] == 'edited'
	// A remote change since the baseline warns first.
	unsafe {
		f.mtime['/home/user/notes.txt'] = 999
	}
	os.write_file(temp, 'edited twice')!
	h.advance(600)
	h.advance(600)
	h.click('upload-ok')!
	h.wait_tasks()!
	assert h.has_text('Remote file changed')
	h.click('ask-yes')!
	h.wait_tasks()!
	assert f.uploaded['/home/user/notes.txt'] == 'edited twice'
	// Ending the session removes the working copy.
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	dispose(mut cx, v)
	assert !os.exists(temp)
}

fn test_edit_respects_the_size_limit_and_custom_editor() {
	mut h, v, _, _ := flow_rig('limit')!
	mut launched := &[]string{}
	mut cx := core.Cx{
		app: unsafe { h.app }
	}
	cx.update_view(v, fn [mut launched] (mut p SftpPanel, mut cx core.ViewCx[SftpPanel]) {
		p.launch = fn [mut launched] (program string, args []string, path string) ! {
			launched << program + ' ' + args.join(' ') + ' ' + path
		}
	})
	set_edit_settings(mut cx, v, EditSettings{
		mode:    'custom'
		program: 'ed'
		args:    [
			'-n',
		]
		limit:   10
	})
	select_named(mut h, v, 'notes.txt')
	act(mut h, v, .edit)
	assert h.has_text('Large file')
	h.click('ask-yes')!
	h.wait_tasks()!
	assert launched.len == 1
	unsafe {
		assert (*launched)[0].starts_with('ed -n ')
	}
}

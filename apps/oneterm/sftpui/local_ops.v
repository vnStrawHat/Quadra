// The Local pane's mutations, menus and toolbar.
module sftpui

import os
import ssh
import quadra.core
import quadra.kit
import quadra.ui

struct LocalDone {
	ok  bool
	msg string
	err string
}

enum LocalAct {
	upload
	open_dir
	rename
	delete
	new_folder
	refresh
}

fn on_local_pick(mut p SftpPanel, i &int, mut cx core.ViewCx[SftpPanel]) {
	match unsafe { LocalAct(*i) } {
		.upload { p.local_upload_selected(mut cx) }
		.open_dir { p.local_open_selected(mut cx) }
		.rename { p.local_do_rename(mut cx) }
		.delete { p.local_do_delete(mut cx) }
		.new_folder { p.local_do_new_folder(mut cx) }
		.refresh { p.local_refresh(mut cx) }
	}
}

fn (mut p SftpPanel) local_open_selected(mut cx core.ViewCx[SftpPanel]) {
	if e := p.local_selected() {
		if e.is_dir {
			p.local_load(e.path, mut cx)
		}
	}
}

fn local_item(label string, glyph string, act LocalAct, l core.AnyListener) kit.MenuItem {
	return kit.MenuItem{
		label:  label
		glyph:  glyph
		action: kit.MenuAction(PanelPick{l, int(act)})
	}
}

fn (p &SftpPanel) local_row_menu(is_dir bool, mut cx core.ViewCx[SftpPanel]) []kit.MenuItem {
	l := cx.listener(on_local_pick).any
	mut out := []kit.MenuItem{}
	if is_dir {
		out << local_item('Open', '', .open_dir, l)
	}
	out << local_item('Upload', kit.icon_arrow_right, .upload, l)
	out << kit.separator()
	out << local_item('Rename', icon_replace, .rename, l)
	out << local_item('Delete', kit.icon_trash_2, .delete, l)
	out << kit.separator()
	out << local_item('New Folder', kit.icon_plus, .new_folder, l)
	out << local_item('Refresh', kit.icon_refresh_cw, .refresh, l)
	return out
}

fn (p &SftpPanel) local_empty_menu(mut cx core.ViewCx[SftpPanel]) []kit.MenuItem {
	l := cx.listener(on_local_pick).any
	return [local_item('New Folder', kit.icon_plus, .new_folder, l),
		local_item('Refresh', kit.icon_refresh_cw, .refresh, l)]
}

fn (p &SftpPanel) local_more_menu(mut cx core.ViewCx[SftpPanel]) []kit.MenuItem {
	l := cx.listener(on_local_pick).any
	return [local_item('New Folder', kit.icon_plus, .new_folder, l),
		local_item('Upload', kit.icon_arrow_right, .upload, l),
		local_item('Rename', icon_replace, .rename, l),
		local_item('Delete', kit.icon_trash_2,
			.delete, l),
		kit.separator(), local_item('Refresh', kit.icon_refresh_cw, .refresh, l)]
}

// run_local runs one blocking change on a worker; on success the form closes and the list
// refreshes, on failure the form stays and a notice says why.
fn (mut p SftpPanel) run_local(success string, prefix string, op LocalOp, mut cx core.ViewCx[SftpPanel]) {
	core.spawn_task_with[SftpPanel, LocalJob, LocalDone](mut cx, LocalJob{op, success, prefix}, fn (j LocalJob, _ &core.TaskCx) !LocalDone {
		j.op.run() or { return LocalDone{
			err: '${j.prefix}: ${err.msg()}'
		} }
		return LocalDone{
			ok:  true
			msg: j.success
		}
	}, fn (mut p SftpPanel, r &core.TaskResult[LocalDone], mut cx core.ViewCx[SftpPanel]) {
		d := r.value
		if !d.ok {
			p.notice(.error, d.err, mut cx)
			return
		}
		if p.local.form_modal != unsafe { nil } {
			kit.close_modal(mut cx.Cx, p.local.form_modal.id)
			p.local.form_modal = unsafe { nil }
		}
		p.notice(.success, d.msg, mut cx)
		p.local_refresh(mut cx)
	})
}

fn (mut p SftpPanel) local_do_new_folder(mut cx core.ViewCx[SftpPanel]) {
	if p.local.cwd == '' {
		return
	}
	p.local.form_modal = open_form(mut cx.Cx, 'New Local Folder', 'Folder name', '', 'Folder name',
		'Create', cx.listener(fn (mut p SftpPanel, v &string, mut cx core.ViewCx[SftpPanel]) {
		name := validate_entry_name(*v) or {
			p.notice(.warning, err.msg(), mut cx)
			return
		}
		p.run_local('Folder "${name}" created.', 'Create folder failed', LocalOp{
			kind: .mkdir
			path: os.join_path(p.local.cwd, name)
		}, mut cx)
	}))
}

fn (mut p SftpPanel) local_do_rename(mut cx core.ViewCx[SftpPanel]) {
	e := p.local_selected() or {
		p.notice(.warning, 'Select a local file or folder to rename.', mut cx)
		return
	}
	p.local.form_modal = open_form(mut cx.Cx, 'Rename', 'New name', e.name, 'New name', 'Rename', core.listener_with[SftpPanel, ssh.FileEntry, string](cx.entity,
		e, fn (mut p SftpPanel, e ssh.FileEntry, v &string, mut cx core.ViewCx[SftpPanel]) {
		name := validate_entry_name(*v) or {
			p.notice(.warning, err.msg(), mut cx)
			return
		}
		p.run_local('Renamed to "${name}".', 'Rename failed', LocalOp{
			kind: .rename
			path: e.path
			to:   os.join_path(p.local.cwd, name)
		}, mut cx)
	}))
}

fn (mut p SftpPanel) local_do_delete(mut cx core.ViewCx[SftpPanel]) {
	e := p.local_selected() or {
		p.notice(.warning, 'Select a local file or folder to delete.', mut cx)
		return
	}
	p.ask(mut cx, 'Confirm Delete', delete_confirmation(e.name, e.is_dir), 'Delete', 'Cancel',
		true, core.listener_with[SftpPanel, ssh.FileEntry, bool](cx.entity, e, fn (mut p SftpPanel, e ssh.FileEntry, yes &bool, mut cx core.ViewCx[SftpPanel]) {
		if !*yes {
			return
		}
		p.run_local('Deleted successfully.', 'Delete failed', LocalOp{
			kind:   .delete
			path:   e.path
			is_dir: e.is_dir
		}, mut cx)
	}))
}

fn (mut p SftpPanel) local_pane(mut cx core.ViewCx[SftpPanel], th &kit.Theme) core.Element {
	mut row :=
		ui.div().flex_row().items_center().gap(4).w_full().h(40).shrink_0().px(8).border_b(1).border_color(th.p.line)
	row.child(ui.text('Local').size(th.font_size(.xs)).color(th.p.text_2))
	row.child(path_box(core.view_element(p.local.path.any()), p.local.path_error, th))
	row.child(tool_button('local-back', kit.icon_arrow_left, cx.listener(fn (mut p SftpPanel, _ &core.ClickEvent, mut cx core.ViewCx[SftpPanel]) {
		p.local_parent(mut cx)
	})))
	row.child(tool_button('local-refresh', kit.icon_refresh_cw, cx.listener(fn (mut p SftpPanel, _ &core.ClickEvent, mut cx core.ViewCx[SftpPanel]) {
		p.local_refresh(mut cx)
	})))
	row.child(kit.icon_button('local-more', icon_ellipsis_vertical).ghost().size(.sm).on_click(core.handler_with(p.local_more_menu(mut cx),
		open_more_menu)))
	row.child(kit.button('local-upload').label('Upload').icon(kit.icon_arrow_up).ghost().size(.sm).disabled(p.local.selected < 0).tooltip('Upload the selected entry to the remote directory').on_click(cx.listener(fn (mut p SftpPanel, _ &core.ClickEvent, mut cx core.ViewCx[SftpPanel]) {
		p.local_upload_selected(mut cx)
	})))
	mut col := ui.div().flex_col().size_full().min_h(0).child(row)
	if p.local.error != '' {
		col.child(ui.div().test_id('local-listing-error').w_full().shrink_0().px(8).py(4).bg(th.p.error_tint).text_size(th.font_size(.xs)).text_color(th.p.error).child(ui.text('Could not open "${p.local.cwd}": ${p.local.error}')))
	}
	return col.child(ui.div().grow().min_h(0).child(core.view_element(p.local.table.any())))
}

enum LocalOpKind {
	mkdir
	rename
	delete
}

// LocalOp is one blocking change on the local disk, run on a worker (no closure: specs/14).
struct LocalOp {
	kind   LocalOpKind
	path   string
	to     string // rename target
	is_dir bool   // delete a folder tree
}

fn (o LocalOp) run() ! {
	match o.kind {
		.mkdir {
			os.mkdir(o.path)!
		}
		.rename {
			if os.exists(o.to) {
				return error('an entry with that name already exists')
			}
			os.mv(o.path, o.to)!
		}
		.delete {
			if o.is_dir {
				os.rmdir_all(o.path)!
			} else {
				os.rm(o.path)!
			}
		}
	}
}

struct LocalJob {
	op      LocalOp
	success string
	prefix  string
}

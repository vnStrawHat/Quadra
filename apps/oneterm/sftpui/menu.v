// The browser's one action list, and the menus built from it.
module sftpui

import quadra.core
import quadra.kit

pub enum SftpAction {
	open
	edit
	download
	upload_files
	upload_folder
	new_folder
	rename
	delete
	properties
	refresh
}

enum MenuTarget {
	empty_area
	file
	directory
}

// key is the app's bindable action id (Bindable in apps/oneterm/keys.v).
pub fn (a SftpAction) key() string {
	return 'sftp_${a.str()}'
}

pub fn (a SftpAction) label() string {
	return match a {
		.open { 'Open' }
		.edit { 'Edit' }
		.download { 'Download' }
		.upload_files { 'Upload Files' }
		.upload_folder { 'Upload Folder' }
		.new_folder { 'New Folder' }
		.rename { 'Rename' }
		.delete { 'Delete' }
		.properties { 'Properties' }
		.refresh { 'Refresh' }
	}
}

fn (a SftpAction) glyph() string {
	return match a {
		.open { icon_folder_open }
		.edit { kit.icon_file_text }
		.download { kit.icon_arrow_down }
		.upload_files, .upload_folder { kit.icon_arrow_up }
		.new_folder { kit.icon_plus }
		.rename { icon_replace }
		.delete { kit.icon_trash_2 }
		.properties { kit.icon_info }
		.refresh { kit.icon_refresh_cw }
	}
}

fn (a SftpAction) needs_selection() bool {
	return a !in [.upload_files, .upload_folder, .new_folder, .refresh]
}

// available_for is part of the list: both row menus drop the same rows for the same entry.
fn (a SftpAction) available_for(target MenuTarget) bool {
	return match target {
		.empty_area { !a.needs_selection() }
		.directory { a != .edit }
		.file { true }
	}
}

// menu_rows is the canonical order, grouped by the separators between the groups.
fn menu_rows(target MenuTarget) [][]SftpAction {
	all := [
		[SftpAction.open, .edit, .download],
		[SftpAction.upload_files, .upload_folder, .new_folder],
		[SftpAction.rename, .delete],
		[SftpAction.properties, .refresh],
	]
	mut out := [][]SftpAction{}
	for group in all {
		kept := group.filter(it.available_for(target))
		if kept.len > 0 {
			out << kept
		}
	}
	return out
}

// PanelPick is a menu row's action: the index goes to the panel's listener (no closure per
// row, specs/14).
struct PanelPick {
	l core.AnyListener
	i int
}

fn (a PanelPick) run(mut cx core.Cx) {
	i := a.i
	cx.app.invoke(a.l, &i, cx.win, true)
}

const pick_follow = 1000
const pick_column = 2000

fn on_menu_pick(mut p SftpPanel, i &int, mut cx core.ViewCx[SftpPanel]) {
	v := *i
	if v == pick_follow {
		p.toggle_follow(mut cx)
	} else if v >= pick_column {
		p.toggle_column(v - pick_column, mut cx)
	} else {
		p.run(unsafe { SftpAction(v) }, mut cx)
	}
}

fn (p &SftpPanel) menu_items(target MenuTarget, mut cx core.ViewCx[SftpPanel]) []kit.MenuItem {
	l := cx.listener(on_menu_pick).any
	mut out := []kit.MenuItem{}
	for g, group in menu_rows(target) {
		if g > 0 {
			out << kit.separator()
		}
		for a in group {
			out << kit.MenuItem{
				label:    a.label()
				glyph:    a.glyph()
				shortcut: p.shortcut[a.key()]
				action:   kit.MenuAction(PanelPick{l, int(a)})
			}
		}
	}
	return out
}

// toolbar_menu is the row list for the selection, plus what configures the browser.
fn (p &SftpPanel) toolbar_menu(mut cx core.ViewCx[SftpPanel]) []kit.MenuItem {
	l := cx.listener(on_menu_pick).any
	entry := p.selected_entry()
	target := if e := entry {
		if e.is_dir { MenuTarget.directory } else { MenuTarget.file }
	} else {
		MenuTarget.file
	}
	mut out := p.menu_items(target, mut cx)
	out << kit.separator()
	out << kit.MenuItem{
		label:   'Follow Terminal Cwd'
		checked: p.active != unsafe { nil } && p.active.follow
		action:  kit.MenuAction(PanelPick{l, pick_follow})
	}
	out << kit.separator()
	out << kit.menu_heading('Columns')
	cols := cx.peek(p.table.model()).cols
	for i, c in cols {
		out << kit.MenuItem{
			label:    c.label
			checked:  c.visible
			disabled: c.sort == .name
			action:   kit.MenuAction(PanelPick{l, pick_column + i})
		}
	}
	return out
}

fn (mut p SftpPanel) toggle_column(i int, mut cx core.ViewCx[SftpPanel]) {
	mut t := cx.peek(p.table.model())
	if i < 0 || i >= t.cols.len || t.cols[i].sort == .name {
		return
	}
	t.cols[i].visible = !t.cols[i].visible
	cx.touch(p.table.model())
	p.schedule_save(mut cx)
	cx.notify()
}

// run performs an action (menus and key bindings).
fn (mut p SftpPanel) run(a SftpAction, mut cx core.ViewCx[SftpPanel]) {
	p.bind(mut cx)
	match a {
		.open { p.do_open(mut cx) }
		.edit { p.do_edit(mut cx) }
		.download { p.do_download(mut cx) }
		.upload_files { p.pick_and_upload(false, mut cx) }
		.upload_folder { p.pick_and_upload(true, mut cx) }
		.new_folder { p.do_new_folder(mut cx) }
		.rename { p.do_rename(mut cx) }
		.delete { p.do_delete(mut cx) }
		.properties { p.do_properties(mut cx) }
		.refresh { p.refresh(mut cx) }
	}
}

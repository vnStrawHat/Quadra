// The right dock (specs/22): frame, width rule, docks.json, the Session and SFTP panels and what
// makes them follow the active terminal.
module main

import os
import x.json2
import sessionui
import sftpui
import ssh
import quadra.core
import quadra.kit
import quadra.prim
import quadra.ui

// clamp_right_dock_width caps a width at 35 % of the window, with a 240 px floor.
pub fn clamp_right_dock_width(pref f32, window f32) f32 {
	return prim.clamp(pref, 240, prim.max(240, window * 0.35))
}

// session_panel is the dock's upper section: its focus is the Session key bindings' context.
pub fn (mut o OneTerm) session_panel() core.Element {
	return ui.div().size_full().key_context('SessionPanel').child(core.view_element(o.sess_view.any()))
}

// sftp_panel is the dock's lower section; its focus is the SFTP key bindings' context.
pub fn (mut o OneTerm) sftp_panel() core.Element {
	return ui.div().size_full().key_context('SftpPanel').test_id('sftp-panel').child(core.view_element(o.sftp_view.any()))
}

// mount_panels creates the two panels; an elevated window has neither.
fn (mut o OneTerm) mount_panels(mut cx core.ViewCx[OneTerm]) {
	if is_restricted() {
		return
	}
	o.sess_view = sessionui.new_panel(mut cx.Cx, sessionui.PanelOptions{
		store:      o.sessions
		vault:      o.vault
		on_connect: cx.listener(fn (mut o OneTerm, r &sessionui.ConnectRequest, mut cx core.ViewCx[OneTerm]) {
			o.on_connect(r, mut cx)
		})
		on_cancel:  cx.listener(fn (mut o OneTerm, id &u64, mut _ core.ViewCx[OneTerm]) {
			o.on_cancel(*id)
		})
	})
	o.sftp_view = sftpui.new_panel(mut cx.Cx, sftpui.PanelOptions{
		edit_dir:    os.join_path(o.dir, 'sftp-edit')
		state_path:  o.docks_path()
		local_dir:   o.local_home
		editor:      o.edit_settings()
		on_expanded: cx.listener(fn (mut o OneTerm, on &bool, mut cx core.ViewCx[OneTerm]) {
			o.sftp_expanded = *on
			cx.notify()
		})
	})
	o.sftp_expanded = cx.peek(o.sftp_view.model()).expanded
	o.push_panel_settings(mut cx.Cx)
	cx.on_close_requested(cx.listener(fn (mut o OneTerm, _ &core.CloseRequest, mut cx core.ViewCx[OneTerm]) {
		o.before_exit(mut cx)
	}))
}

fn (o &OneTerm) edit_settings() sftpui.EditSettings {
	e := o.cfg.sftp.editor
	return sftpui.EditSettings{
		mode:    e.mode
		program: e.program
		args:    e.args
		limit:   o.cfg.sftp.edit_max_file_size
	}
}

// push_panel_settings runs at start and after every change in the settings window.
fn (o &OneTerm) push_panel_settings(mut cx core.Cx) {
	if o.sftp_view.id == 0 {
		return
	}
	sftpui.set_edit_settings(mut cx, o.sftp_view, o.edit_settings())
	mut keys := map[string]string{}
	for b in bindables {
		if b.group == sftp_menu {
			keys[b.id] = effective_key(o.ui.key_bindings, b.id)
		}
	}
	sftpui.set_shortcuts(mut cx, o.sftp_view, keys)
}

fn (mut o OneTerm) quit_app(mut cx core.ViewCx[OneTerm]) {
	o.before_exit(mut cx)
	cx.app.quit()
}

// before_exit releases what must not outlive the window: edit copies, SSH sessions.
fn (mut o OneTerm) before_exit(mut cx core.ViewCx[OneTerm]) {
	if o.sftp_view.id != 0 {
		sftpui.dispose(mut cx.Cx, o.sftp_view)
	}
	for tab in o.tabs {
		for s in tab.spaces {
			if s.remote {
				stop(s, mut cx)
			}
		}
	}
}

// session_cmd runs a Session panel action (the + menu, key bindings).
fn (mut o OneTerm) session_cmd(d Do, mut cx core.ViewCx[OneTerm]) {
	if is_restricted() || o.sess_view.id == 0 {
		return
	}
	mut p := cx.peek(o.sess_view.model())
	match d.cmd {
		.quick_connect { p.open_quick_connect(mut cx.Cx) }
		.new_saved_session { p.open_new_session(mut cx.Cx) }
		.open_saved { p.open_connect(mut cx.Cx, d.arg.u64()) }
		.open_selected { p.open_selected(mut cx.Cx) }
		.delete_selected { p.delete_selected(mut cx.Cx) }
		.properties_selected { p.properties_selected(mut cx.Cx) }
		else {}
	}
}

fn (mut o OneTerm) sftp_cmd(name string, mut cx core.ViewCx[OneTerm]) {
	if is_restricted() || o.sftp_view.id == 0 {
		return
	}
	action := match name {
		'open' { sftpui.SftpAction.open }
		'download' { .download }
		'rename' { .rename }
		'delete' { .delete }
		'properties' { .properties }
		'upload_files' { .upload_files }
		'upload_folder' { .upload_folder }
		'new_folder' { .new_folder }
		else { .refresh }
	}

	sftpui.run_action(mut cx.Cx, o.sftp_view, action)
}

// sync_active follows the active terminal: the dock (on a kind change only), SFTP, the cwd.
fn (mut o OneTerm) sync_active(mut cx core.ViewCx[OneTerm]) {
	now_local := o.active_is_local()
	if now_local != o.last_local {
		o.last_local = now_local
		if !is_restricted() {
			o.dock_open = o.ui.right_dock.open_for(now_local)
		}
	}
	if o.sftp_view.id == 0 {
		return
	}
	s := o.current().active_space() or { return }
	mut sftp := ?&ssh.Sftp(none)
	mut key := u64(0)
	if s.session != unsafe { nil } && !s.ssh_closed {
		sftp = s.session.sftp()
		if live := sftp {
			key = u64(voidptr(live))
		}
	}
	if key != o.bound_sftp {
		o.bound_sftp = key
		sftpui.set_session(mut cx.Cx, o.sftp_view, sftp)
	}
	o.follow_cwd_of(s, mut cx)
}

// follow_cwd_of tells the browser the active SSH terminal's OSC 7 path ('' for a local one).
fn (mut o OneTerm) follow_cwd_of(s &Space, mut cx core.ViewCx[OneTerm]) {
	if o.sftp_view.id == 0 {
		return
	}
	active := o.current().active_space() or { return }
	if active.id != s.id {
		return
	}
	cwd := if s.remote && s.view.id != 0 { cx.peek(s.view.model()).screen.cwd } else { '' }
	sftpui.follow_cwd(mut cx.Cx, o.sftp_view, cwd)
}

// section is a dock section: a header (the title in the text colour, the actions in a
// full-height group behind a left border, as the tab strip's) above its content.
fn section(title string, actions []core.Element, content core.Element, t &kit.Theme) core.Element {
	actions_row :=
		ui.div().flex_row().items_center().h_full().shrink_0().border_l(1).border_color(t.p.line).px(t.sp(8)).gap(t.sp(4)).children(actions)
	header :=
		ui.div().flex_row().items_center().w_full().h(t.panel_header_h).shrink_0().bg(t.p.tab_bg).border_b(1).border_color(t.p.line).child(ui.div().grow().min_w(0).px(t.sp(8)).child(ui.text(title).size(t.font_size(.sm)).color(t.p.text))).child(actions_row)
	return ui.div().flex_col().size_full().child(header).child(ui.div().grow().min_h(0).child(content))
}

// right_dock is the dock; full: the expanded browser over the whole workspace, no handle.
fn (mut o OneTerm) right_dock(t &kit.Theme, full bool, mut cx core.ViewCx[OneTerm]) core.Element {
	w := clamp_right_dock_width(o.dock_pref, cx.win.size().width)
	bottom := section('SFTP Browser', [sftpui.expand_button(mut cx.Cx, o.sftp_view)],
		o.sftp_panel(), t)
	mut content := core.Element(bottom)
	if !o.sftp_expanded { // an expanded browser takes the whole dock
		add := kit.icon_button('session-add', kit.icon_plus).ghost().size(.xs).tooltip('New Session').on_click(cx.listener(fn (mut o OneTerm, _ &core.ClickEvent, mut cx core.ViewCx[OneTerm]) {
			mut p := cx.peek(o.sess_view.model())
			p.open_new_session(mut cx.Cx)
		}))
		top := section('Session', [core.Element(add)], o.session_panel(), t)
		content = core.Element(kit.split('dock-split', top, bottom, o.dock_split).vertical().on_resize(cx.listener(fn (mut o OneTerm, r &f32, mut cx core.ViewCx[OneTerm]) {
			o.dock_split = *r
			o.save_docks()
			cx.notify()
		})))
	}
	if full {
		return ui.div().test_id('right-dock').flex_col().grow().min_w(0).h_full().bg(t.p.canvas).child(content)
	}
	// A 5 px band centred on the dock's left border; its transparent line carries the pill.
	handle := ui.div().id('dock-handle').test_id('dock-handle').group().absolute().top(0).bottom(0).left(-3).w(5).flex_row().justify_center().cursor(.resize_ew).child(kit.resize_line(false,
		prim.transparent, t)).on_mouse_down(core.handler(fn (_ &core.MouseDownEvent, mut cx core.Cx) {
		cx.capture_mouse()
	})).on_mouse_move(cx.listener(fn (mut o OneTerm, e &core.MouseMoveEvent, mut cx core.ViewCx[OneTerm]) {
		if !e.pressed {
			return
		}
		win := cx.win.size().width
		o.dock_pref = clamp_right_dock_width(win - e.x, win)
		cx.notify()
	})).on_mouse_up(cx.listener(fn (mut o OneTerm, _ &core.MouseUpEvent, mut _ core.ViewCx[OneTerm]) {
		o.save_docks()
	}))
	return ui.div().test_id('right-dock').flex_col().w(w).h_full().shrink_0().bg(t.p.canvas).border_l(1).border_color(t.p.line).child(content).child(handle)
}

// toggle_dock opens or closes the dock and remembers the choice for this tab's kind.
fn (mut o OneTerm) toggle_dock(mut _ core.ViewCx[OneTerm]) {
	if is_restricted() {
		return
	}
	o.dock_open = !o.dock_open
	if o.active_is_local() {
		o.ui.right_dock.local = o.dock_open
	} else {
		o.ui.right_dock.ssh = o.dock_open
	}
	o.persist_ui() or {}
	o.save_docks()
}

// active_is_local: the kind the dock follows is the active Space's; an empty Space counts as local.
fn (o &OneTerm) active_is_local() bool {
	s := o.current().active_space() or { return true }
	return !s.remote
}

struct DockFile {
	schema_version int = 1
	version        int = 3
}

fn (o &OneTerm) docks_path() string {
	return os.join_path(o.dir, 'docks.json')
}

// load_docks restores the width and the split; the open state comes from the tab's kind.
fn (mut o OneTerm) load_docks() {
	o.dock_open = o.ui.right_dock.open_for(true)
	text := os.read_file(o.docks_path()) or { return }
	raw := json2.decode[json2.Any](text) or { return }
	right := raw.as_map()['right_dock'] or { return }
	m := right.as_map()
	if size := m['size'] {
		o.dock_pref = size.f32()
	}
	if split := m['split'] {
		o.dock_split = split.f32()
	}
}

// save_docks writes docks.json, keeping keys it does not own (SFTP table state, center).
fn (o &OneTerm) save_docks() {
	if is_restricted() {
		return
	}
	mut root := map[string]json2.Any{}
	if text := os.read_file(o.docks_path()) {
		if raw := json2.decode[json2.Any](text) {
			root = raw.as_map().clone()
		} else {
			quarantine(o.docks_path()) // an invalid file is moved aside, not overwritten
		}
	}
	base := DockFile{}
	root['schema_version'] = json2.Any(base.schema_version)
	root['version'] = json2.Any(base.version)
	mut right := map[string]json2.Any{}
	right['open'] = json2.Any(o.dock_open)
	right['size'] = json2.Any(o.dock_pref)
	right['split'] = json2.Any(o.dock_split)
	root['right_dock'] = json2.Any(right)
	write_config(o.docks_path(), json2.encode(root)) or {}
}

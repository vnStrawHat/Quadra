// The panel's layout: toolbar, file list, Local + Remote split, transfer queue.
module sftpui

import quadra.core
import quadra.kit
import quadra.platform
import quadra.prim
import quadra.ui

// expand_button is the toggle for the dock header (title_suffix in the original).
pub fn expand_button(mut cx core.Cx, v core.View[SftpPanel]) core.Element {
	on := cx.read_view(v).expanded
	tip := if on {
		'Collapse: back to the docked remote browser'
	} else {
		'Expand: Local + Remote files side by side'
	}
	return kit.icon_button('sftp-expand', if on { icon_minimize } else { icon_maximize }).ghost().size(.xs).tooltip(tip).on_click(core.listener_of[SftpPanel, core.ClickEvent](v.id, fn (mut p SftpPanel, _ &core.ClickEvent, mut cx core.ViewCx[SftpPanel]) {
		p.set_expanded(!p.expanded, mut cx)
	}))
}

fn path_box(field core.Element, invalid bool, th &kit.Theme) core.Element {
	return ui.div().grow().min_w(0).h(th.height(.sm)).flex_row().items_center().border_b(1).border_color(if invalid {
		th.p.error
	} else {
		th.p.line
	}).child(ui.div().grow().min_w(0).child(field))
}

fn tool_button(id string, glyph string, l core.Listener[core.ClickEvent]) &kit.Button {
	return kit.icon_button(id, glyph).ghost().size(.sm).on_click(l)
}

fn open_more_menu(items []kit.MenuItem, _ &core.ClickEvent, mut cx core.Cx) {
	kit.open_popup_with(mut cx, cx.target_bounds(), .bottom_end, items, fn (items []kit.MenuItem, _ &kit.Theme) core.Element {
		return kit.menu(items)
	})
}

fn (mut p SftpPanel) toolbar(mut cx core.ViewCx[SftpPanel], th &kit.Theme) core.Element {
	st := p.active
	mut row :=
		ui.div().flex_row().items_center().gap(4).w_full().h(40).shrink_0().px(8).border_b(1).border_color(th.p.line)
	if p.expanded {
		mut dl :=
			kit.button('sftp-download-local').label('Download').icon(kit.icon_arrow_down).ghost().size(.sm).disabled(st.selected < 0).tooltip("Download the selected entry into the Local pane's directory")
		dl.on_click(cx.listener(fn (mut p SftpPanel, _ &core.ClickEvent, mut cx core.ViewCx[SftpPanel]) {
			p.do_download(mut cx)
		}))
		row.child(dl).child(ui.text('Remote').size(th.font_size(.xs)).color(th.p.text_2))
	}
	row.child(path_box(core.view_element(p.path.any()), st.path_error, th))
	row.child(tool_button('sftp-back', kit.icon_arrow_left, cx.listener(fn (mut p SftpPanel, _ &core.ClickEvent, mut cx core.ViewCx[SftpPanel]) {
		p.navigate_parent(mut cx)
	})))
	tip := if p.term_cwd != '' {
		"Go to terminal's current directory: ${p.term_cwd}"
	} else {
		'Terminal has not reported a directory (needs shell integration / OSC 7)'
	}
	row.child(kit.icon_button('sftp-sync-cwd', icon_folder_sync).ghost().size(.sm).disabled(p.term_cwd == '').tooltip(tip).on_click(cx.listener(fn (mut p SftpPanel, _ &core.ClickEvent, mut cx core.ViewCx[SftpPanel]) {
		p.sync_to_terminal(mut cx)
	})))
	row.child(tool_button('sftp-refresh', kit.icon_refresh_cw, cx.listener(fn (mut p SftpPanel, _ &core.ClickEvent, mut cx core.ViewCx[SftpPanel]) {
		p.refresh(mut cx)
	})))
	row.child(kit.icon_button('sftp-more', icon_ellipsis_vertical).ghost().size(.sm).on_click(core.handler_with(p.toolbar_menu(mut cx),
		open_more_menu)))
	return row
}

fn (mut p SftpPanel) remote_pane(mut cx core.ViewCx[SftpPanel], th &kit.Theme) core.Element {
	mut col := ui.div().flex_col().size_full().min_h(0).child(p.toolbar(mut cx, th))
	if banner := p.remote_error_banner() {
		col.child(ui.div().test_id('sftp-listing-error').w_full().shrink_0().px(8).py(4).bg(th.p.error_tint).text_size(th.font_size(.xs)).text_color(th.p.error).child(ui.text(banner)))
	}
	return col.child(ui.div().grow().min_h(0).child(core.view_element(p.table.any())))
}

fn split_with_local(local core.Element, remote core.Element, ratio f32, mut cx core.ViewCx[SftpPanel]) core.Element {
	return kit.split('sftp-panes', local, remote, ratio).on_resize(cx.listener(fn (mut p SftpPanel, r &f32, mut cx core.ViewCx[SftpPanel]) {
		p.split = *r
		cx.notify()
	}))
}

@[markused]
pub fn (mut p SftpPanel) render(mut cx core.ViewCx[SftpPanel]) core.Element {
	if !p.drop_bound && cx.win != unsafe { nil } {
		p.drop_bound = true
		p.win = cx.win
		cx.on_files_dropped(cx.listener(on_files_dropped))
	}
	th := kit.theme(cx)
	mut root :=
		ui.div().test_id('sftp-panel').flex_col().size_full().min_h(0).bg(th.p.canvas).text_size(th.font_size(.md)).text_color(th.p.text).label('SFTP browser')
	if p.active == unsafe { nil } {
		empty :=
			ui.div().size_full().center().text_color(th.p.text_2).child(ui.text('No SFTP connection.'))
		if p.expanded {
			return root.child(split_with_local(p.local_pane(mut cx, th), empty, p.split, mut cx))
		}
		return root.child(empty)
	}
	remote := p.remote_pane(mut cx, th)
	if p.expanded {
		root.child(ui.div().grow().min_h(0).child(split_with_local(p.local_pane(mut cx, th),
			remote, p.split, mut cx)))
	} else {
		root.child(ui.div().grow().min_h(0).child(remote))
	}
	return root.child(p.transfer_queue(mut cx, th))
}

// on_files_dropped is the OS drop: inside the remote list it uploads; elsewhere it is not
// ours (the window also holds terminals).
fn on_files_dropped(mut p SftpPanel, e &platform.FileDropEvent, mut cx core.ViewCx[SftpPanel]) {
	at := prim.pt(e.x, e.y)
	if p.active == unsafe { nil } { // no list to drop on: the panel says why nothing happened
		panel := cx.win.test_ids['sftp-panel'] or { return }
		if panel.contains(at) {
			p.notice(.warning, 'No active SFTP connection — dropped files were not uploaded.', mut
				cx)
		}
		return
	}
	r := cx.win.test_ids['sftp-table'] or { return }
	if r.contains(at) {
		p.upload_paths(e.paths, mut cx)
	}
}

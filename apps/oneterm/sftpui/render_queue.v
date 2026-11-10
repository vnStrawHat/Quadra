// The transfer queue under the file list: progress bars, cancel, and Clear.
module sftpui

import quadra.core
import quadra.kit
import quadra.ui

fn (mut p SftpPanel) clear_finished(mut cx core.ViewCx[SftpPanel]) {
	if p.active != unsafe { nil } {
		p.active.transfers = p.active.transfers.filter(it.status == .in_progress)
		cx.notify()
	}
}

// cancel_transfer signals the session and shows the item cancelled at once.
fn (mut p SftpPanel) cancel_transfer(id u64, mut cx core.ViewCx[SftpPanel]) {
	if p.active == unsafe { nil } {
		return
	}
	mut st := p.active
	st.client.cancel_transfer(id)
	i := st.find_transfer(id)
	if i >= 0 && st.transfers[i].status == .in_progress {
		st.transfers[i].status = .cancelled
	}
	cx.notify()
}

fn (t &TransferItem) label() string {
	if t.status == .in_progress && t.discovered >= 0 {
		noun := if t.discovered == 1 { 'file' } else { 'files' }
		return '${t.filename} (scanning, ${t.discovered} ${noun} found)'
	}
	return t.filename
}

fn (t &TransferItem) status_text() string {
	return match t.status {
		.in_progress { '${int(t.progress * 100 + 0.5)}%' }
		.completed { 'Done' }
		.cancelled { 'Cancelled' }
		.error { 'Error' }
	}
}

fn (mut p SftpPanel) transfer_row(t TransferItem, mut cx core.ViewCx[SftpPanel], th &kit.Theme) core.Element {
	pal := th.p
	glyph, glyph_color := if t.direction == .upload {
		kit.icon_arrow_up, pal.ok
	} else {
		kit.icon_arrow_down, pal.cyan
	}
	bar := match t.status {
		.in_progress { pal.text }
		.completed { pal.ok }
		.cancelled { pal.text_2 }
		.error { pal.error }
	}

	frac := if t.progress < 0 {
		f32(0)
	} else if t.progress > 1 {
		f32(1)
	} else {
		f32(t.progress)
	}
	mut row :=
		ui.div().test_id('sftp-transfer-${t.id}').flex_row().items_center().gap(8).w_full().h(24).shrink_0().px(8).text_size(th.font_size(.xs)).text_color(pal.text)
	row.child(ui.div().w(16).shrink_0().child(kit.icon(glyph).size(th.sp(14)).color(glyph_color)))
	row.child(ui.div().grow().min_w(0).overflow_hidden().child(ui.text(t.label()).truncate()))
	row.child(ui.div().w(100).h(4).shrink_0().rounded(2).bg(bar.alpha(0.2)).child(ui.div().w(100 * frac).h_full().rounded(2).bg(bar)))
	row.child(ui.div().w(60).shrink_0().child(ui.text(t.status_text())))
	if t.error != '' {
		row.child(ui.div().shrink_0().max_w(160).overflow_hidden().text_color(pal.error).child(ui.text(t.error).truncate()))
	}
	if t.status == .in_progress {
		row.child(ui.div().shrink_0().child(kit.icon_button('sftp-cancel-${t.id}', kit.icon_x).ghost().size(.sm).tooltip('Cancel transfer').on_click(core.listener_with[SftpPanel, u64, core.ClickEvent](cx.entity,
			t.id, fn (mut p SftpPanel, id u64, _ &core.ClickEvent, mut cx core.ViewCx[SftpPanel]) {
			p.cancel_transfer(id, mut cx)
		}))))
	}
	return row
}

fn (mut p SftpPanel) transfer_queue(mut cx core.ViewCx[SftpPanel], th &kit.Theme) core.Element {
	if p.active == unsafe { nil } || p.active.transfers.len == 0 {
		return ui.div()
	}
	st := p.active
	active := st.active_count()
	done := st.transfers.len - active
	mut q :=
		ui.div().test_id('sftp-transfers').flex_col().w_full().shrink_0().max_h(200).border_t(1).border_color(th.p.line)
	q.child(ui.div().flex_row().items_center().gap(8).w_full().h(28).shrink_0().px(8).text_size(th.font_size(.xs)).child(ui.text('Transfers').color(th.p.text)).child(ui.text('${active} active, ${done} done').color(th.p.text_2)).child(ui.div().grow()).child(kit.button('sftp-clear-transfers').label('Clear').ghost().size(.xs).disabled(done == 0).on_click(cx.listener(fn (mut p SftpPanel, _ &core.ClickEvent, mut cx core.ViewCx[SftpPanel]) {
		p.clear_finished(mut cx)
	}))))
	mut list :=
		ui.div().id('sftp-transfer-list').flex_col().w_full().overflow_y_scroll().grow().min_h(0)
	for t in st.transfers {
		list.child(p.transfer_row(t, mut cx, th))
	}
	return q.child(list)
}

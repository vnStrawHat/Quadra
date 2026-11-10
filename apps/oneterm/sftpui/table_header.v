// The header of a file table: sort titles and column resize handles.
module sftpui

import quadra.core
import quadra.kit
import quadra.prim
import quadra.ui

fn header_click(mut t FileTable, i int, _ &core.ClickEvent, mut cx core.ViewCx[FileTable]) {
	t.cycle_sort(i, mut cx)
}

fn resize_down(mut t FileTable, i int, e &core.MouseDownEvent, mut cx core.ViewCx[FileTable]) {
	t.drag_col, t.drag_x, t.drag_w = i, e.x, t.cols[i].width
	cx.capture_mouse()
}

fn resize_move(mut t FileTable, e &core.MouseMoveEvent, mut cx core.ViewCx[FileTable]) {
	if e.pressed && t.drag_col >= 0 {
		t.cols[t.drag_col].width = prim.clamp(t.drag_w + e.x - t.drag_x, column_min_width,
			column_max_width)
		cx.notify()
	} else if t.drag_col >= 0 {
		t.drag_col = -1
		t.emit(TableEvent{ kind: .resized }, mut cx.Cx)
	}
}

fn resize_up(mut t FileTable, _ &core.MouseUpEvent, mut cx core.ViewCx[FileTable]) {
	if t.drag_col >= 0 {
		t.drag_col = -1
		t.emit(TableEvent{ kind: .resized }, mut cx.Cx)
	}
}

fn (t &FileTable) header_cell(i int, c Column, mut cx core.ViewCx[FileTable], th &kit.Theme) core.Element {
	p := th.p
	on := t.sort.on && t.sort.col == c.sort
	glyph := if on {
		if t.sort.desc { kit.icon_arrow_down } else { kit.icon_arrow_up }
	} else {
		kit.icon_chevrons_up_down
	}
	// The sort icon in the secondary foreground (the text), at half opacity while unsorted.
	icon_color := if on { p.text } else { p.text.alpha(0.5) }
	mut title :=
		ui.div().test_id('${t.id}-h-${c.key}').flex_row().items_center().gap(4).grow().min_w(0).h_full().px(cell_px).text_color(p.table_head_fg).child(ui.div().grow().min_w(0).child(ui.text(c.label).truncate())).child(kit.icon(glyph).size(th.sp(12)).color(icon_color))

	title.cursor(.pointer).hover(ui.Patch{
		background: p.panel_hover
	}).on_click(core.listener_with[FileTable, int, core.ClickEvent](cx.entity, i, header_click))
	mut cell := ui.div().flex_row().h_full().child(title)
	if c.sort == .name {
		return cell.grow().min_w(name_min_width).overflow_hidden()
	}
	// Only a resizable column has a divider: the handle's right edge in the row border colour,
	// the border colour while hovered.
	cell.w(c.width).shrink_0().child(ui.div().test_id('${t.id}-r-${c.key}').w(resize_w).h_full().border_r(1).border_color(p.table_row_border).cursor(.resize_ew).hover(ui.Patch{
		border_color: p.line
	}).on_mouse_down(core.listener_with[FileTable, int, core.MouseDownEvent](cx.entity, i,
		resize_down)).on_mouse_move(cx.listener(fn (mut t FileTable, e &core.MouseMoveEvent, mut cx core.ViewCx[FileTable]) {
		resize_move(mut t, e, mut cx)
	})).on_mouse_up(cx.listener(resize_up)))
	return cell
}

fn (t &FileTable) header(mut cx core.ViewCx[FileTable], th &kit.Theme) core.Element {
	p := th.p
	mut row :=
		ui.div().flex_row().w_full().h(header_h).shrink_0().bg(p.table_head_bg).border_b(1).border_color(p.line).text_size(th.font_size(.md))
	rest := t.rest_width()
	mut inner := ui.div().flex_row().absolute().top(0).left(-t.scroll_x).w(rest).h_full()
	for i, c in t.cols {
		if !c.visible {
			continue
		}
		if c.sort == .name {
			row.child(t.header_cell(i, c, mut cx, th))
		} else {
			inner.child(t.header_cell(i, c, mut cx, th))
		}
	}
	return row.child(ui.div().w(rest).min_w(0).h_full().overflow_hidden().child(inner))
}

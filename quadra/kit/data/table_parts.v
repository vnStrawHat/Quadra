// Table header (sorting, column resizing) and virtualized body.
module data

import quadra.core
import quadra.prim
import quadra.kit
import quadra.ui

const handle_w = f32(5)

fn (t &Table) total_width() f32 {
	mut w := f32(0)
	for c in t.columns {
		if !c.hidden {
			w += c.width
		}
	}
	return w
}

fn (t &Table) header(mut cx core.ViewCx[Table], th &kit.Theme) core.Element {
	p := th.p
	pinned := t.pinned_mode()
	_, rest_w := t.split_widths()
	mut cells := []core.Element{}
	mut pins := []bool{}
	// The header (specs/15): muted 14 px titles over a hairline; the sort arrow in accent.
	mut row :=
		ui.div().flex_row().h(th.panel_header_h).w(if pinned { 0 } else { t.total_width() }).shrink_0().bg(p.table_head_bg).border_b(1).border_color(p.line).text_size(th.ui_text(.md)).text_color(p.table_head_fg)
	for i, col in t.columns {
		if col.hidden {
			continue
		}
		mut cell :=
			ui.div().test_id('${t.id}-h${i}').flex_row().items_center().gap(4).w(col.width - handle_w).shrink_0().h_full().pl(8).role(.cell).label(col.title)
		if col.numeric || col.kind == .age {
			cell.justify_end().pr(16 - handle_w) // clear of the 14 px scroll track
		}
		cell.child(ui.text(col.title))
		if i == t.sort_col {
			cell.child(kit.icon(if t.sort_desc { kit.icon_arrow_down } else { kit.icon_arrow_up }).size(12).color(p.accent_text))
		}
		if col.sortable {
			cell.cursor(.pointer).hover(ui.Patch{
				background: p.panel_hover
				text_color: p.text
			}).on_click(core.listener_with[Table, int, core.ClickEvent](cx.entity, i, fn (mut t Table, i int, _ &core.ClickEvent, mut _ core.ViewCx[Table]) {
				t.sort_by(i)
			}))
		}
		// Resize handle on the right edge, inside the column width: drag with the mouse captured.
		mut wrap := ui.div().flex_row().shrink_0().h_full().child(cell)
		wrap.child(ui.div().test_id('${t.id}-r${i}').w(handle_w).h_full().ml_auto().cursor(.resize_ew).hover(ui.Patch{
			background: p.border_focused
		}).on_mouse_down(core.listener_with[Table, int, core.MouseDownEvent](cx.entity, i, fn (mut t Table, i int, e &core.MouseDownEvent, mut cx core.ViewCx[Table]) {
			t.drag_col, t.drag_x, t.drag_w = i, e.x, t.columns[i].width
			cx.capture_mouse()
		})).on_mouse_move(cx.listener(fn (mut t Table, e &core.MouseMoveEvent, mut cx core.ViewCx[Table]) {
			if e.pressed && t.drag_col >= 0 {
				col := t.drag_col
				t.columns[col].width = prim.max(t.columns[col].min_width, t.drag_w + e.x - t.drag_x)
				cx.notify()
			} else {
				t.drag_col = -1
			}
		})))
		cells << wrap
		pins << col.pinned
	}
	if pinned {
		row.w_full()
	}
	assemble(mut row, cells, pins, t.scroll_x, rest_w, p.line)
	return row
}

fn (t &Table) body(mut cx core.ViewCx[Table], th &kit.Theme) core.Element {
	cols := t.columns.filter(!it.hidden)
	mut shown := []int{}
	for i, col in t.columns {
		if !col.hidden {
			shown << i
		}
	}
	order := t.order
	rows := t.rows
	cell := t.cell
	keys := t.keys
	flash := t.flash.clone()
	app := cx.app
	selected := t.selected.clone()
	cursor := t.cursor
	focused := t.focus.is_focused(cx.Cx)
	id := t.id
	total := t.total_width()
	ent := cx.entity
	pinned := t.pinned_mode()
	pins := cols.map(it.pinned)
	scroll_x := t.scroll_x
	_, rest_w := t.split_widths()
	cell_mode := t.cell_mode
	cell_col := if cell_mode { t.cell_col } else { -1 }
	list := ui.uniform_list_with('${t.id}-rows', t.rows, t.row_h, TableRows{ent, cols, shown, order, rows, cell, keys, flash, app, selected, cursor, focused, id, unsafe { th }, total, pinned, pins, scroll_x, rest_w, cell_mode, cell_col},
		table_row).reveal(t.cursor)
	mut tick := &Ticker{
		t: unsafe { t }
	}
	mut col := ui.div().flex_col().grow().child(list).child(tick)
	if pinned {
		col.w_full()
	} else {
		col.w(t.total_width())
	}
	return col
}

// age_text formats seconds the way cluster tools do: 45s, 12m, 3h, 6d.
pub fn age_text(s int) string {
	return if s < 60 {
		'${s}s'
	} else if s < 3600 {
		'${s / 60}m'
	} else if s < 86400 {
		'${s / 3600}h'
	} else {
		'${s / 86400}d'
	}
}

// TableRows is what the visible rows are built from (a copy taken at render).
struct TableRows {
	ent       u32
	cols      []Column
	shown     []int
	order     []int
	rows      int
	cell      CellFn = unsafe { nil }
	keys      []string
	flash     map[string]f64
	app       &core.App = unsafe { nil }
	selected  map[int]bool
	cursor    int
	focused   bool
	id        string
	th        &kit.Theme = unsafe { nil }
	total     f32
	pinned    bool
	pins      []bool
	scroll_x  f32
	rest_w    f32
	cell_mode bool
	cell_col  int
}

fn table_row(d TableRows, pos int) core.Element {
	ent, cols, shown, order, rows, cell, keys, app, cursor, focused, id, th, total, pinned, pins, scroll_x, rest_w, cell_mode, cell_col := d.ent, d.cols, d.shown, d.order, d.rows, d.cell, d.keys, d.app, d.cursor, d.focused, d.id, d.th, d.total, d.pinned, d.pins, d.scroll_x, d.rest_w, d.cell_mode, d.cell_col
	p := th.p
	row := if order.len == rows { order[pos] } else { pos }
	on := d.selected[row]
	key := if keys.len == rows { keys[row] } else { '' }
	now := if d.flash.len > 0 { app.plat.now_ms() } else { 0 }
	mut r := ui.div().test_id('${id}-row${pos}').flex_row().items_center().w(if pinned {
		0
	} else {
		total
	}).h_full().border_b(1).border_color(p.table_row_border).text_size(th.ui_text(.md)).role(.row).selected(on)
	// Rows (specs/15): a seam below each, hover a step, selected two; the cursor row of a
	// focused table is outlined in the focus color (an overlay, so nothing shifts).
	if on {
		r.bg(if focused { p.press } else { p.raised_hover })
	} else {
		r.hover(ui.Patch{
			background: p.raised_hover.alpha(0.6)
		})
	}
	if focused && pos == cursor {
		r.child(ui.div().absolute().top(0).left(0).size_full().border(1).border_color(p.border_focused))
	}
	r.on_click(core.listener_with[Table, int, core.ClickEvent](ent, pos, fn (mut t Table, pos int, e &core.ClickEvent, mut cx core.ViewCx[Table]) {
		t.click_row(pos, *e, mut cx)
	}))
	if pinned {
		r.w_full()
	}
	mut cells := []core.Element{cap: cols.len}
	for k, col in cols {
		ci := shown[k]
		v := cell(row, ci)
		mut cl :=
			ui.div().w(col.width).h_full().flex_row().items_center().shrink_0().px(8).overflow_hidden().role(.cell)
		if col.numeric || col.kind == .age {
			cl.justify_end().pr(16)
		}
		if key != '' {
			level := flash_level(d.flash, key, ci, now)
			if level > 0 {
				cl.bg(th.p.accent_tint.alpha(th.p.accent_tint.a * level))
			}
		}
		// Numbers in the mono face with tabular figures, so columns line up (specs/15).
		family := if col.numeric { th.mono_family } else { th.font_family }
		txt := match col.kind {
			.status { core.Element(kit.status_label(kit.status_of(v), v)) }
			.age { core.Element(ui.text(age_text(v.int())).family(th.mono_family)) }
			.text { core.Element(ui.text(v).family(family)) }
		}

		if ci == cell_col && pos == cursor { // the cursor cell (cell selection)
			cl.border(1).border_color(p.border_focused).selected(true)
		}
		if cell_mode {
			cl.on_mouse_down(core.listener_with[Table, int, core.MouseDownEvent](ent, ci, fn (mut t Table, ci int, _ &core.MouseDownEvent, mut _ core.ViewCx[Table]) {
				t.cell_col = ci
			}))
		}
		cells << cl.child(txt)
	}
	assemble(mut r, cells, pins, scroll_x, rest_w, p.line)
	return r
}

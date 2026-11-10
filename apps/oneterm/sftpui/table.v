// FileTable: the file list of both panes: icon cell, sortable resizable columns, a pinned
// Name, single selection, double click, a context menu per row, and rows as drag sources.
// kit.data.Table has string cells only (specs/27).
module sftpui

import quadra.core
import quadra.kit
import quadra.prim
import quadra.ui

pub enum TableEventKind {
	select
	clear
	open
	context_row
	context_empty
	sorted
	resized
	dropped
}

// TableEvent is the one event a table sends its owner.
pub struct TableEvent {
pub:
	kind TableEventKind
	row  int
	x    f32
	y    f32
	path string // what was dropped (the drag value)
}

const row_h = f32(30)
const header_h = f32(30)
const resize_w = f32(5)
const cell_px = f32(6) // the original's small table cells

@[heap]
pub struct FileTable {
pub mut:
	id         string
	cols       []Column
	rows       []Row
	paths      []string // the drag value of each row
	sort       SortState
	selected   int = -1
	loading    bool
	drag_kind  string // rows are drag sources of this kind; '' = not draggable
	drop_kind  string // accepts drags of this kind
	empty_text string = 'Empty directory.'
mut:
	on_event core.AnyListener
	focus    core.FocusHandle
	scroll_x f32
	drag_col int = -1
	drag_x   f32
	drag_w   f32
}

@[params]
pub struct TableOptions {
pub:
	id        string
	cols      []Column
	drag_kind string
	drop_kind string
	on_event  core.Listener[TableEvent]
}

pub fn new_table(mut cx core.Cx, opts TableOptions) core.View[FileTable] {
	return cx.new_view(FileTable{
		id:        opts.id
		cols:      opts.cols
		drag_kind: opts.drag_kind
		drop_kind: opts.drop_kind
		on_event:  opts.on_event.any
		focus:     cx.focus_handle()
	})
}

fn (t &FileTable) emit(ev TableEvent, mut cx core.Cx) {
	cx.app.invoke(t.on_event, &ev, cx.win, true)
}

fn (t &FileTable) rest_width() f32 {
	mut w := f32(0)
	for c in t.cols {
		if c.visible && c.sort != .name {
			w += c.width
		}
	}
	return w
}

// select_row selects and tells the owner.
fn (mut t FileTable) select_row(i int, mut cx core.ViewCx[FileTable]) {
	if i < 0 || i >= t.rows.len {
		return
	}
	t.selected = i
	t.emit(TableEvent{ kind: .select, row: i }, mut cx.Cx)
	cx.notify()
}

// cycle_sort walks a column through none, descending, ascending, as the header click does.
fn (mut t FileTable) cycle_sort(i int, mut cx core.ViewCx[FileTable]) {
	col := t.cols[i].sort
	if t.sort.on && t.sort.col == col {
		if t.sort.desc {
			t.sort.desc = false
		} else {
			t.sort.on = false
		}
	} else {
		t.sort = SortState{true, col, true}
	}
	t.emit(TableEvent{ kind: .sorted, row: i }, mut cx.Cx)
	cx.notify()
}

fn row_click(mut t FileTable, i int, e &core.ClickEvent, mut cx core.ViewCx[FileTable]) {
	t.focus.focus(mut cx.Cx)
	t.select_row(i, mut cx)
	if e.clicks == 2 {
		t.emit(TableEvent{ kind: .open, row: i }, mut cx.Cx)
	}
}

fn row_press(mut t FileTable, i int, e &core.MouseDownEvent, mut cx core.ViewCx[FileTable]) {
	if e.button == .right {
		t.select_row(i, mut cx)
		t.emit(TableEvent{ kind: .context_row, row: i, x: e.x, y: e.y }, mut cx.Cx)
		cx.stop_propagation()
	}
}

struct RowsData {
	ent      u32
	rows     []Row
	paths    []string
	cols     []Column
	selected int
	focused  bool
	id       string
	th       &kit.Theme = unsafe { nil }
	scroll_x f32
	rest_w   f32
	drag     string
}

// cell_el is one cell: the Name cell carries the file icon, the rest are muted small text.
fn cell_el(row &Row, c Column, k int, th &kit.Theme) core.Element {
	if c.sort == .name {
		src := if row.is_dir { file_icon_folder } else { file_icon_file }
		// The 19 px icon sits in a 16 px box, so the name starts 20 px in, as in the original.
		return ui.div().flex_row().items_center().gap(4).w_full().min_w(0).child(ui.div().w(16).shrink_0().child(ui.svg(src).size(19))).child(ui.text(row.cells[k]).color(th.p.text).truncate())
	}
	mut d :=
		ui.div().w(c.width).shrink_0().h_full().flex_row().items_center().px(cell_px).overflow_hidden().text_size(th.font_size(.sm)).text_color(th.p.text_2)
	if c.right {
		d.justify_end()
	}
	return d.child(ui.text(row.cells[k]).truncate())
}

fn file_row(d RowsData, i int) core.Element {
	p := d.th.p
	row := d.rows[i]
	mut r :=
		ui.div().test_id('${d.id}-row${i}').flex_row().items_center().w_full().h_full().border_b(1).border_color(p.table_row_border).role(.row).selected(i == d.selected)
	if i == d.selected {
		r.bg(p.panel_hover)
	}
	r.hover(ui.Patch{
		background: p.panel_hover
	})
	r.on_click(core.listener_with[FileTable, int, core.ClickEvent](d.ent, i, row_click))
	r.on_mouse_down(core.listener_with[FileTable, int, core.MouseDownEvent](d.ent, i, row_press))
	if d.drag != '' && i < d.paths.len {
		r.drag_source(core.DragSource{
			payload:    core.DragPayload{d.drag, d.paths[i]}
			preview_el: core.Element(ui.div().px(8).py(4).rounded(4).border(1).border_color(p.line).bg(p.panel).text_color(p.text).child(ui.text(row.name)))
		})
	}
	mut left :=
		ui.div().flex_row().items_center().grow().min_w(name_min_width).h_full().px(cell_px).overflow_hidden()
	mut inner :=
		ui.div().flex_row().items_center().absolute().top(0).left(-d.scroll_x).w(d.rest_w).h_full()
	for k, c in d.cols {
		if !c.visible {
			continue
		}
		if c.sort == .name {
			left.child(cell_el(row, c, k, d.th))
		} else {
			inner.child(cell_el(row, c, k, d.th))
		}
	}
	return r.child(left).child(ui.div().w(d.rest_w).min_w(0).h_full().overflow_hidden().child(inner))
}

fn table_scroll(mut t FileTable, e &core.ScrollEvent, mut cx core.ViewCx[FileTable]) {
	dx := if e.dx != 0 {
		e.dx
	} else if e.mods.has(.shift) {
		e.dy
	} else {
		f32(0)
	}
	if dx == 0 {
		return
	}
	view := cx.target_bounds().width
	max_x := prim.max(0, t.rest_width() - prim.max(0, view - name_min_width))
	nx := prim.clamp(t.scroll_x - dx, 0, max_x)
	if nx != t.scroll_x {
		t.scroll_x = nx
		cx.notify()
	}
	cx.stop_propagation()
}

fn table_key(mut t FileTable, e &core.KeyDownEvent, mut cx core.ViewCx[FileTable]) {
	match e.key {
		.up {
			t.select_row(if t.selected > 0 { t.selected - 1 } else { 0 }, mut cx)
		}
		.down {
			t.select_row(if t.selected < 0 { 0 } else { t.selected + 1 }, mut cx)
		}
		.escape {
			if t.selected >= 0 {
				t.selected = -1
				t.emit(TableEvent{ kind: .clear }, mut cx.Cx)
				cx.notify()
			}
		}
		else {
			return
		}
	}

	cx.stop_propagation()
}

fn table_press(mut t FileTable, e &core.MouseDownEvent, mut cx core.ViewCx[FileTable]) {
	if e.button == .right && e.y >= cx.target_bounds().y + header_h {
		t.emit(TableEvent{ kind: .context_empty, x: e.x, y: e.y }, mut cx.Cx)
	}
}

fn table_drop(mut t FileTable, e &core.DropEvent, mut cx core.ViewCx[FileTable]) {
	t.emit(TableEvent{ kind: .dropped, path: e.payload.value }, mut cx.Cx)
}

@[markused]
pub fn (mut t FileTable) render(mut cx core.ViewCx[FileTable]) core.Element {
	th := kit.theme(cx)
	mut root :=
		ui.div().id(t.id).test_id(t.id).flex_col().size_full().min_h(0).overflow_hidden().role(.table).track_focus(t.focus).on_key_down(cx.listener(table_key)).on_mouse_down(cx.listener(table_press))
	root.handlers << ui.Handler{.scroll, true, cx.listener(table_scroll).any}
	if t.drop_kind != '' {
		root.on_drop(t.drop_kind, cx.listener(table_drop)).drag_over(ui.Patch{
			background: th.p.accent_tint
		})
	}
	root.child(t.header(mut cx, th))
	if t.loading {
		mut col := ui.div().flex_col().gap(10).p(10).w_full()
		for _ in 0 .. 8 {
			col.child(kit.skeleton(200, 14))
		}
		return root.child(col)
	}
	if t.rows.len == 0 {
		return root.child(ui.div().grow().center().text_color(th.p.text_2).child(ui.text(t.empty_text)))
	}
	list := ui.uniform_list_with('${t.id}-rows', t.rows.len, row_h, RowsData{cx.entity, t.rows, t.paths, t.cols, t.selected, t.focus.is_focused(cx.Cx), t.id, th, t.scroll_x, t.rest_width(), t.drag_kind},
		file_row).reveal(t.selected)
	return root.child(ui.div().grow().min_h(0).w_full().text_size(th.font_size(.md)).child(list))
}

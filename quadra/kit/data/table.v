// Table: virtualized rows, sortable and resizable columns, sticky header, row selection.
module data

import quadra.core
import quadra.kit
import quadra.ui

// CellKind picks how a column draws its value.
pub enum CellKind {
	text
	status // a status word, drawn with kit.status_label
	age    // seconds, drawn relative (45s, 12m, 3h, 6d) and sorted as a number
}

pub struct Column {
pub mut:
	title     string
	width     f32  = 140
	min_width f32  = 48
	sortable  bool = true
	numeric   bool // right-aligned, sorted as numbers
	kind      CellKind
	hidden    bool
	pinned    bool // stays at the left while the other columns scroll sideways
}

pub type CellFn = fn (row int, col int) string

@[heap]
pub struct Table {
pub mut:
	id        string
	columns   []Column
	rows      int
	cell      CellFn = unsafe { nil }
	row_h     f32    = 26
	sort_col  int    = -1
	sort_desc bool
	order     []int // display position -> data row
	selected  map[int]bool
	anchor    int = -1
	cursor    int = -1 // display position of the keyboard cursor
	focus     core.FocusHandle
	on_sel    ?core.AnyListener // []int of data rows
	drag_col  int = -1
	drag_x    f32
	drag_w    f32
	// keyed updates (live.v)
	keys        []string
	prev        map[string][]string // last cell texts by key
	flash       map[string]f64      // when a cell last changed, by flash_key
	flash_until f64
	// table_pin.v
	scroll_x  f32  // sideways offset of the unpinned columns (with pinned ones only)
	view_w    f32  // the table's width in the last prepaint
	cell_mode bool // the cursor is a cell: left and right move it across columns
	cell_col  int = -1 // the cursor cell's column
}

@[params]
pub struct TableOptions {
pub:
	id      string
	columns []Column
	rows    int
	cell    CellFn @[required]
	row_h   f32 = 26
	// select_cells makes the cursor a cell (spreadsheet-like); rows are still selected.
	select_cells bool
}

pub fn new_table(mut cx core.Cx, opts TableOptions) core.View[Table] {
	return cx.new_view(Table{
		id:        opts.id
		columns:   opts.columns
		rows:      opts.rows
		cell:      opts.cell
		row_h:     opts.row_h
		focus:     cx.focus_handle()
		cell_mode: opts.select_cells
	})
}

pub fn on_select(mut cx core.Cx, t core.View[Table], l core.Listener[[]int]) {
	mut tb := cx.peek(t.model())
	tb.on_sel = l.any
}

// set_rows replaces the data (live data sources): the selection is cleared and the current
// sort is applied again.
pub fn set_rows(mut cx core.Cx, t core.View[Table], rows int, cell CellFn) {
	mut tb := cx.peek(t.model())
	tb.rows, tb.cell = rows, cell
	tb.keys.clear()
	tb.prev.clear()
	tb.flash.clear()
	tb.order = []int{}
	tb.selected.clear()
	tb.anchor, tb.cursor = -1, -1
	if tb.sort_col >= 0 {
		tb.sort_desc = !tb.sort_desc // sort_by toggles: keep the direction
		tb.sort_by(tb.sort_col)
	}
	cx.touch(t.model())
}

// data_row maps a display position to a data row (after sorting).
pub fn (t &Table) data_row(pos int) int {
	return if t.order.len == t.rows { t.order[pos] } else { pos }
}

pub fn (t &Table) selection() []int {
	mut out := t.selected.keys()
	out.sort()
	return out
}

fn (mut t Table) sort_by(col int) {
	if t.sort_col == col {
		t.sort_desc = !t.sort_desc
	} else {
		t.sort_col, t.sort_desc = col, false
	}
	numeric := t.columns[col].numeric || t.columns[col].kind == .age
	keys := []string{len: t.rows, init: t.cell(index, col)}
	nums := if numeric { keys.map(it.f64()) } else { []f64{} }
	mut order := []int{len: t.rows, init: index}
	desc := t.sort_desc
	order.sort_with_compare(fn [keys, nums, numeric, desc] (a &int, b &int) int {
		mut c := 0
		if numeric {
			c = if nums[*a] < nums[*b] {
				-1
			} else if nums[*a] > nums[*b] {
				1
			} else {
				0
			}
		} else {
			c = compare_strings(keys[*a], keys[*b])
		}
		if c == 0 {
			c = *a - *b // stable
		}
		return if desc { -c } else { c }
	})
	t.order = order
}

fn (mut t Table) click_row(pos int, mods core.ClickEvent, mut cx core.ViewCx[Table]) {
	row := t.data_row(pos)
	if mods.mods.has(.shift) && t.anchor >= 0 {
		t.selected.clear()
		lo, hi := if t.anchor < pos { t.anchor, pos } else { pos, t.anchor }
		for p in lo .. hi + 1 {
			t.selected[t.data_row(p)] = true
		}
	} else if mods.mods.has(.ctrl) {
		if row in t.selected {
			t.selected.delete(row)
		} else {
			t.selected[row] = true
		}
		t.anchor = pos
	} else {
		t.selected.clear()
		t.selected[row] = true
		t.anchor = pos
	}
	t.cursor = pos
	t.emit(mut cx)
}

fn (t &Table) emit(mut cx core.ViewCx[Table]) {
	if l := t.on_sel {
		sel := t.selection()
		cx.app.invoke(l, &sel, cx.win, true)
	}
}

fn (mut t Table) key(e &core.KeyDownEvent, mut cx core.ViewCx[Table]) {
	if e.key == .c && e.mods.has(.ctrl) {
		s := t.copy_text()
		if s != '' {
			cx.clipboard_write(s)
		}
		cx.stop_propagation()
		return
	}
	if t.cell_mode && e.key in [.left, .right] {
		t.move_cell(if e.key == .left { -1 } else { 1 })
		cx.notify()
		cx.stop_propagation()
		return
	}
	step := match e.key {
		.down { 1 }
		.up { -1 }
		.page_down { 20 }
		.page_up { -20 }
		.home { -t.rows }
		.end { t.rows }
		else { 0 }
	}

	if step == 0 || t.rows == 0 {
		return
	}
	pos := if t.cursor < 0 { 0 } else { t.cursor + step }
	t.cursor = if pos < 0 {
		0
	} else if pos >= t.rows {
		t.rows - 1
	} else {
		pos
	}
	ev := core.ClickEvent{
		mods: e.mods
	}
	t.click_row(t.cursor, ev, mut cx)
	cx.stop_propagation()
}

@[markused]
pub fn (mut t Table) render(mut cx core.ViewCx[Table]) core.Element {
	th := kit.theme(cx)
	mut root := ui.div().id(t.id).test_id(t.id).flex_col().size_full().role(.table).track_focus(t.focus).on_key_down(cx.listener(fn (mut t Table, e &core.KeyDownEvent, mut cx core.ViewCx[Table]) {
		t.key(e, mut cx)
	}))
	if t.pinned_mode() {
		root.overflow_hidden() // the table scrolls its unpinned columns itself (table_pin.v)
		root.handlers << ui.Handler{.scroll, true, cx.listener(table_scroll).any}
	} else {
		root.overflow_x_scroll()
	}
	return root.child(t.header(mut cx, th)).child(t.body(mut cx, th))
}

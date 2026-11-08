// Live tables (specs/19): keyed updates keep the selection, cursor and sort, and changed
// cells flash once.
module data

import quadra.core
import quadra.prim
import quadra.layout

pub const flash_ms = 800.0

fn flash_key(key string, col int) string {
	return '${key}\t${col}'
}

// set_keyed_rows replaces the rows with data identified by keys (one per row). The cell
// function is normally the same one every time (made once, reading the owner's data), so
// live updates make no closures.
pub fn set_keyed_rows(mut cx core.Cx, t core.View[Table], keys []string, cell CellFn) {
	mut tb := cx.peek(t.model())
	tb.update_keyed(keys, cell, cx.app.plat.now_ms())
	cx.touch(t.model())
}

// update_keyed is set_keyed_rows on the model (tests and owners holding the table).
pub fn (mut t Table) update_keyed(keys []string, cell CellFn, now f64) {
	// Remember what was selected and where the cursor was, by key.
	same_shape := t.keys.len == t.rows
	mut sel_keys := map[string]bool{}
	mut cursor_key := ''
	if same_shape {
		for row, _ in t.selected {
			if row < t.keys.len {
				sel_keys[t.keys[row]] = true
			}
		}
		if t.cursor >= 0 && t.cursor < t.rows {
			cursor_key = t.keys[t.data_row(t.cursor)]
		}
	}
	t.flash_cells(keys, cell, now)
	t.keys = keys
	t.rows, t.cell = keys.len, cell
	t.order = []int{}
	if t.sort_col >= 0 {
		t.sort_desc = !t.sort_desc // sort_by toggles: keep the direction
		t.sort_by(t.sort_col)
	}
	t.selected.clear()
	t.cursor, t.anchor = -1, -1
	for row, k in keys {
		if k in sel_keys {
			t.selected[row] = true
		}
	}
	if cursor_key != '' {
		for pos in 0 .. t.rows {
			if keys[t.data_row(pos)] == cursor_key {
				t.cursor, t.anchor = pos, pos
				break
			}
		}
	}
}

// flash_cells records the cells whose text changed under the same key.
fn (mut t Table) flash_cells(keys []string, cell CellFn, now f64) {
	mut next := map[string][]string{}
	for row, k in keys {
		cells := []string{len: t.columns.len, init: t.shown_text(cell(row, index), index)}
		if old := t.prev[k] {
			for c in 0 .. cells.len {
				if c < old.len && old[c] != cells[c] {
					t.flash[flash_key(k, c)] = now
					t.flash_until = now + flash_ms
				}
			}
		}
		next[k] = cells
	}
	t.prev = next.move()
	// Drop finished flashes.
	for fk, at in t.flash {
		if now - at > flash_ms {
			t.flash.delete(fk)
		}
	}
}

// flash_level is how strongly a cell is tinted now (1 just changed, 0 none).
fn flash_level(flash map[string]f64, key string, col int, now f64) f32 {
	at := flash[flash_key(key, col)] or { return 0 }
	k := 1 - (now - at) / flash_ms
	return if k > 0 { f32(k) } else { 0 }
}

// Ticker keeps prepaint running while a flash fades (rows rebuild in prepaint).
struct Ticker {
mut:
	t    &Table = unsafe { nil }
	node layout.NodeId
}

fn (mut k Ticker) request_layout(mut cx core.LayoutCx) layout.NodeId {
	k.node = cx.tree.node(layout.Style{
		width:  layout.pct(100) // measures the table width (Table.view_w)
		height: layout.px(0)
	}, [])
	return k.node
}

fn (mut k Ticker) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	k.t.view_w = bounds.width
	if cx.app.plat.now_ms() < k.t.flash_until {
		cx.request_prepaint_after(33)
	}
}

fn (mut k Ticker) paint(_ prim.Rect, mut _ core.PaintCx) {}

// shown_text is a cell as drawn (ages are relative), which is what a flash compares.
fn (t &Table) shown_text(v string, col int) string {
	return if t.columns[col].kind == .age { age_text(v.int()) } else { v }
}

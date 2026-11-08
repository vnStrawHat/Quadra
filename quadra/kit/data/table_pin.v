// Pinned columns, cell selection and copying. With a pinned column the table scrolls
// sideways itself: pinned cells stay in a fixed group at the left, the others sit in a
// clip shifted by scroll_x (rows and header alike), so they never move vertically apart.
module data

import strings
import quadra.core
import quadra.prim
import quadra.ui

fn (t &Table) pinned_mode() bool {
	return t.columns.any(it.pinned && !it.hidden)
}

// widths of the pinned columns and of the others (shown ones only).
fn (t &Table) split_widths() (f32, f32) {
	mut pin, mut rest := f32(0), f32(0)
	for c in t.columns {
		if c.hidden {
			continue
		}
		if c.pinned {
			pin += c.width
		} else {
			rest += c.width
		}
	}
	if pin > 0 {
		pin += 1 // the pinned group's right border (assemble)
	}
	return pin, rest
}

// assemble adds a row's cells (one per shown column, in order) to row: directly, or split
// into the pinned group and the shifted clip.
fn assemble(mut row ui.Div, cells []core.Element, pinned []bool, scroll_x f32, rest_w f32, line prim.Color) {
	if !pinned.any(it) {
		row.children(cells)
		return
	}
	mut left := ui.div().flex_row().shrink_0().h_full().border_r(1).border_color(line)
	mut inner := ui.div().flex_row().absolute().top(0).left(-scroll_x).w(rest_w).h_full()
	for k, c in cells {
		if pinned[k] {
			left.child(c)
		} else {
			inner.child(c)
		}
	}
	row.child(left).child(ui.div().grow().h_full().overflow_hidden().child(inner))
}

// scroll_by moves the shifted columns, within what they need to show their last column.
fn (mut t Table) scroll_by(dx f32) bool {
	pin, rest := t.split_widths()
	max_x := prim.max(0, rest - (t.view_w - pin))
	nx := prim.clamp(t.scroll_x + dx, 0, max_x)
	if nx == t.scroll_x {
		return false
	}
	t.scroll_x = nx
	return true
}

// table_scroll runs in the capture phase: sideways wheels (or shift with the wheel) scroll
// the columns before the row list takes the event.
fn table_scroll(mut t Table, e &core.ScrollEvent, mut cx core.ViewCx[Table]) {
	if !t.pinned_mode() {
		return
	}
	dx := if e.dx != 0 {
		-e.dx
	} else if e.mods.has(.shift) {
		-e.dy
	} else {
		f32(0)
	}
	if dx != 0 && t.scroll_by(dx) {
		cx.notify()
		cx.stop_propagation()
	}
}

// move_cell moves the cell cursor by step shown columns and scrolls it into view.
fn (mut t Table) move_cell(step int) {
	mut shown := []int{}
	for i, c in t.columns {
		if !c.hidden {
			shown << i
		}
	}
	if shown.len == 0 {
		return
	}
	k := shown.index(t.cell_col)
	nk := if k < 0 { 0 } else { int(prim.clamp(f32(k + step), 0, f32(shown.len - 1))) }
	t.cell_col = shown[nk]
	if !t.pinned_mode() || t.columns[t.cell_col].pinned {
		return
	}
	pin, _ := t.split_widths()
	mut x := f32(0) // the column's left edge in the shifted group
	for i in shown {
		if i == t.cell_col {
			break
		}
		if !t.columns[i].pinned {
			x += t.columns[i].width
		}
	}
	w := t.columns[t.cell_col].width
	view := t.view_w - pin
	if x < t.scroll_x {
		t.scroll_by(x - t.scroll_x)
	} else if x + w > t.scroll_x + view {
		t.scroll_by(x + w - view - t.scroll_x)
	}
}

// copy_text is what ctrl-c copies: the cursor cell in cell mode, else the selected rows as
// tab-separated shown columns.
fn (t &Table) copy_text() string {
	if t.cell_mode {
		if t.cursor < 0 || t.cell_col < 0 {
			return ''
		}
		return t.cell(t.data_row(t.cursor), t.cell_col)
	}
	mut b := strings.new_builder(256)
	for row in t.selection() {
		mut first := true
		for i, c in t.columns {
			if c.hidden {
				continue
			}
			if !first {
				b.write_u8(`\t`)
			}
			b.write_string(t.cell(row, i))
			first = false
		}
		b.write_u8(`\n`)
	}
	return b.str()
}

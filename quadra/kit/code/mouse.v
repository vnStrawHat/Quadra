// Mouse: click places the cursor, drag selects (by character, word or line after a double
// or triple click), ctrl-click adds a cursor (on a URL it opens it, links.v), alt-drag
// selects a column, shift-click extends, and a click in the gutter selects lines.
module code

import math
import quadra.core
import quadra.text

// line_at is the line under window y.
fn (e &Editor) line_at(y f32) int {
	return e.row_line(e.row_at_y(y))
}

// offset_at maps a window point to a document offset.
pub fn (mut e Editor) offset_at(ts &text.TextSystem, x f32, y f32) int {
	e.sync_rows()
	d := e.doc
	r := e.row_at_y(y)
	n := e.row_line(r)
	dl := display_line(d.buf.line(n), d.tab_size)
	mut sys := unsafe { ts }
	l := sys.layout_mono(dl.shown, e.style, e.cell.width)
	ra, rb := e.row_span(n, r)
	mut k := math.max(ra, l.index_for_x(x - e.bounds.x + e.scroll.x + l.x_for_index(ra)))
	if rb < dl.shown.len && k >= rb { // past a wrapped row's end: stay on the row
		k = text.prev_grapheme(dl.shown, rb)
	}
	return d.buf.line_start(n) + dl.to_src(k)
}

// col_at is the display column under window x (past the line end too).
fn (e &Editor) col_at(x f32) int {
	if e.cell.width <= 0 {
		return 0
	}
	return math.max(0, int((x - e.bounds.x + e.scroll.x) / e.cell.width + 0.5))
}

fn (d &Doc) next_line_start(n int) int {
	return if n + 1 < d.buf.line_count() { d.buf.line_start(n + 1) } else { d.buf.len() }
}

fn (mut e Editor) mouse_down(ev &core.MouseDownEvent, mut cx core.ViewCx[Editor]) {
	if ev.button != .left {
		return
	}
	if ev.mods.has(.ctrl) && !ev.mods.has(.alt) && e.ctrl_click(ev, mut cx) {
		return
	}
	e.focus.focus(mut cx.Cx)
	mut d := e.doc
	off := e.offset_at(cx.app.text, ev.x, ev.y)
	add := ev.mods.has(.ctrl)
	if ev.mods.has(.alt) {
		e.drag, e.drag_a, e.drag_e = .column, e.line_at(ev.y), e.col_at(ev.x)
		d.set_cursor(off, off, false)
	} else if ev.clicks >= 3 || ev.x < e.bounds.x {
		n := d.buf.line_of(off)
		e.drag, e.drag_a, e.drag_e = .line, d.buf.line_start(n), d.next_line_start(n)
		d.set_cursor(e.drag_a, e.drag_e, add)
	} else if ev.clicks == 2 {
		a, b := d.word_at(off)
		e.drag, e.drag_a, e.drag_e = .word, a, b
		d.set_cursor(a, b, add)
	} else if ev.mods.has(.shift) {
		p := d.primary()
		e.drag, e.drag_a, e.drag_e = .char, p.anchor, p.anchor
		d.set_cursor(p.anchor, off, false)
	} else {
		e.drag, e.drag_a, e.drag_e = .char, off, off
		d.set_cursor(off, off, add)
	}
	cx.capture_mouse()
	e.last_edit = cx.now_ms()
	cx.notify()
}

fn (mut e Editor) mouse_drag(ev &core.MouseMoveEvent, mut cx core.ViewCx[Editor]) {
	if !ev.pressed || e.drag == .none {
		e.drag = .none
		if !ev.pressed {
			e.hover_link(ev.x, ev.y, mut cx)
		}
		return
	}
	mut d := e.doc
	off := e.offset_at(cx.app.text, ev.x, ev.y)
	match e.drag {
		.char {
			e.drag_to(e.drag_a, off)
		}
		.word {
			a, b := d.word_at(off)
			if off < e.drag_a {
				e.drag_to(e.drag_e, a)
			} else {
				e.drag_to(e.drag_a, b)
			}
		}
		.line {
			n := d.buf.line_of(off)
			if d.buf.line_start(n) < e.drag_a {
				e.drag_to(e.drag_e, d.buf.line_start(n))
			} else {
				e.drag_to(e.drag_a, d.next_line_start(n))
			}
		}
		.column {
			e.column_select(e.line_at(ev.y), e.col_at(ev.x))
		}
		.none {}
	}

	e.reveal = true
	e.last_edit = cx.now_ms()
	cx.notify()
}

// drag_to updates the cursor being dragged (the one holding the drag origin).
fn (mut e Editor) drag_to(anchor int, head int) {
	mut d := e.doc
	for i, c in d.cursors {
		if (c.start() <= e.drag_a && e.drag_a <= c.end())
			|| (c.start() <= e.drag_e && e.drag_e <= c.end()) {
			d.cursors[i] = Cursor{
				anchor: anchor
				head:   head
			}
			break
		}
	}
	d.normalize()
}

// column_select makes one cursor per line from the drag origin to line n, column col.
fn (mut e Editor) column_select(n int, col int) {
	mut d := e.doc
	a, b := math.min(e.drag_a, n), math.max(e.drag_a, n)
	mut cs := []Cursor{cap: b - a + 1}
	for k in a .. b + 1 {
		cs << Cursor{
			anchor: d.off_at_col(k, e.drag_e)
			head:   d.off_at_col(k, col)
		}
	}
	d.cursors = cs
	d.normalize()
}

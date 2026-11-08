// Selection by cell over the history and the screen, copy, and mouse reporting.
module term

import strings
import quadra.core

// Pos is a cell boundary: line counts from the oldest history line, col from 0 to cols.
pub struct Pos {
pub:
	line int
	col  int
}

fn (a Pos) before(b Pos) bool {
	return a.line < b.line || (a.line == b.line && a.col < b.col)
}

pub struct Selection {
pub mut:
	anchor   Pos
	head     Pos
	active   bool
	dragging bool
}

pub fn (mut s Selection) clear() {
	s.active, s.dragging = false, false
}

// range returns the selection with its start first.
pub fn (s &Selection) range() (Pos, Pos) {
	return if s.head.before(s.anchor) { s.head, s.anchor } else { s.anchor, s.head }
}

// top_line is the history line shown on the first row.
pub fn (t &Terminal) top_line() int {
	return t.screen.scrollback.len() - t.offset
}

// row_at returns the cells of a line (history or screen).
pub fn (t &Terminal) row_at(line int) Row {
	n := t.screen.scrollback.len()
	if line < n {
		return unpack(t.screen.scrollback.at(line), t.screen.cols)
	}
	return t.screen.lines[clamp(line - n, 0, t.screen.rows - 1)]
}

fn (t &Terminal) pos_at(x f32, y f32) Pos {
	if t.cell.width <= 0 {
		return Pos{}
	}
	row := clamp(int((y - t.bounds.y) / t.cell.height), 0, t.screen.rows - 1)
	col := clamp(int((x - t.bounds.x) / t.cell.width + 0.5), 0, t.screen.cols)
	return Pos{t.top_line() + row, col}
}

fn is_word(c Cell) bool {
	ch := c.ch
	if ch == 0 || ch == 32 {
		return false
	}
	return ch > 127 || (ch >= `0` && ch <= `9`) || (ch >= `a` && ch <= `z`)
		|| (ch >= `A` && ch <= `Z`) || rune(ch) in [`_`, `-`, `.`, `/`, `~`, `:`]
}

fn (mut t Terminal) select_word(p Pos) {
	cells := t.row_at(p.line).cells
	mut a := clamp(p.col, 0, cells.len - 1)
	mut b := a
	if is_word(cells[a]) {
		for a > 0 && is_word(cells[a - 1]) {
			a--
		}
		for b < cells.len && is_word(cells[b]) {
			b++
		}
	} else {
		b = a + 1
	}
	t.sel.anchor, t.sel.head = Pos{p.line, a}, Pos{p.line, b}
}

// selected_text joins the selected cells; rows that wrap join without a newline.
pub fn (t &Terminal) selected_text() string {
	if !t.sel.active {
		return ''
	}
	a, b := t.sel.range()
	mut sb := strings.new_builder(64)
	for line in a.line .. b.line + 1 {
		row := t.row_at(line)
		c0 := if line == a.line { a.col } else { 0 }
		c1 := if line == b.line { b.col } else { row.cells.len }
		mut part := strings.new_builder(c1 - c0)
		for x in clamp(c0, 0, row.cells.len) .. clamp(c1, 0, row.cells.len) {
			c := row.cells[x]
			if c.flags & f_spacer != 0 {
				continue
			}
			part.write_rune(if c.ch == 0 { ` ` } else { rune(c.ch) })
		}
		txt := part.str()
		sb.write_string(if row.wrapped && line != b.line { txt } else { txt.trim_right(' ') })
		if line != b.line && !row.wrapped {
			sb.write_u8(`\n`)
		}
	}
	return sb.str()
}

pub fn (mut t Terminal) copy(mut cx core.ViewCx[Terminal]) {
	s := t.selected_text()
	if s != '' {
		cx.clipboard_write(s)
	}
}

fn (mut t Terminal) mouse_down(e &core.MouseDownEvent, mut cx core.ViewCx[Terminal]) {
	t.focus.focus(mut cx.Cx)
	p := t.pos_at(e.x, e.y)
	if t.screen.mouse != .off && t.screen.mouse_sgr && !e.mods.has(.shift) {
		t.report(e.button == .right, p, `M`) // the program wants the mouse
		return
	}
	if e.button != .left {
		return
	}
	match e.clicks {
		2 {
			t.select_word(p)
		}
		3 {
			t.sel.anchor, t.sel.head = Pos{p.line, 0}, Pos{p.line, t.screen.cols}
		}
		else {
			if !(e.mods.has(.shift) && t.sel.active) {
				t.sel.anchor = p
			}
			t.sel.head = p
		}
	}

	t.sel.active, t.sel.dragging = true, true
	cx.capture_mouse()
	cx.stop_propagation()
}

fn (mut t Terminal) mouse_move(e &core.MouseMoveEvent, mut cx core.ViewCx[Terminal]) {
	if !e.pressed {
		if t.sel.dragging {
			t.sel.dragging = false
			if t.sel.anchor == t.sel.head {
				t.sel.active = false
			}
		}
		return
	}
	if t.sel.dragging {
		t.sel.head = t.pos_at(e.x, e.y)
		cx.notify()
	}
}

// report sends an SGR mouse event (button 0 or 2, press M or release m).
fn (mut t Terminal) report(right bool, p Pos, kind u8) {
	row := p.line - t.top_line() + 1
	btn := if right { 2 } else { 0 }
	t.send('\x1b[<${btn};${clamp(p.col, 0, t.screen.cols - 1) + 1};${row}${rune(kind)}'.bytes())
	t.send('\x1b[<${btn};${clamp(p.col, 0, t.screen.cols - 1) + 1};${row}m'.bytes())
}

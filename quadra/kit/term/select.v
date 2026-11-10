// Selection by cell over the history and the screen, and copy.
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
	block    bool // a rectangle (alt-drag) instead of running text
	expand   SelExpand
	origin   Pos // the cell a word or line selection started on
}

pub fn (mut s Selection) clear() {
	s.active, s.dragging = false, false
}

// line_span returns the lines of the two ends, in order.
pub fn (s &Selection) line_span() (int, int) {
	return if s.anchor.line <= s.head.line {
		s.anchor.line, s.head.line
	} else {
		s.head.line, s.anchor.line
	}
}

// range returns the selection with its start first.
pub fn (s &Selection) range() (Pos, Pos) {
	return if s.head.before(s.anchor) { s.head, s.anchor } else { s.anchor, s.head }
}

// cols_on returns the selected columns of a line inside the selection.
pub fn (s &Selection) cols_on(line int, cols int) (int, int) {
	a, b := s.range()
	if s.block {
		if a.col < b.col {
			return a.col, b.col
		}
		return b.col, a.col
	}
	c0 := if line == a.line { a.col } else { 0 }
	c1 := if line == b.line { b.col } else { cols }
	return c0, c1
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

// selected_text joins the selected cells; rows that wrap join without a newline.
pub fn (t &Terminal) selected_text() string {
	if !t.screen.sel.active {
		return ''
	}
	a, b := t.screen.sel.range()
	mut sb := strings.new_builder(64)
	for line in a.line .. b.line + 1 {
		row := t.row_at(line)
		c0, c1 := t.screen.sel.cols_on(line, row.cells.len)
		mut part := strings.new_builder(c1 - c0)
		mut in_tab := false
		for x in clamp(c0, 0, row.cells.len) .. clamp(c1, 0, row.cells.len) {
			c := row.cells[x]
			if c.flags & f_spacer != 0 {
				continue
			}
			if in_tab { // the blanks a tab jumped over are the tab
				if t.screen.has_stop(x) || (c.ch != 0 && c.ch != 32) {
					in_tab = false
				} else {
					continue
				}
			}
			in_tab = c.ch == 9
			part.write_rune(if c.ch == 0 { ` ` } else { rune(c.ch) })
			for m in row.marks {
				if m.col == x {
					part.write_string(m.s)
				}
			}
		}
		txt := part.str()
		joined := !t.screen.sel.block && row.wrapped && line != b.line
		sb.write_string(if joined { txt } else { txt.trim_right(' ') })
		if line != b.line && !joined {
			sb.write_u8(`\n`)
		}
	}
	return sb.str()
}

// select_all selects the history and the screen.
pub fn (mut t Terminal) select_all() {
	t.screen.sel.anchor = Pos{0, 0}
	t.screen.sel.head = Pos{t.screen.scrollback.len() + t.screen.rows - 1, t.screen.cols}
	t.screen.sel.active = true
}

// paste sends text as pasted input (bracketed when the program asked for it).
pub fn (mut t Terminal) paste(s string) {
	t.type_bytes(paste_bytes(s, t.screen.bracketed))
}

pub fn (mut t Terminal) copy(mut cx core.ViewCx[Terminal]) {
	s := t.selected_text()
	if s != '' {
		cx.clipboard_write(s)
	}
}

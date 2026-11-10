// Resize: the main screen reflows wrapped lines to the new width; the alternate screen is
// cut or padded (specs/17).
module term

struct LogicalLine {
mut:
	cells []Cell
	marks []Mark
	extra &RowExtra = unsafe { nil }
}

pub fn (mut s Screen) resize(cols int, rows int) {
	if cols < 1 || rows < 1 || (cols == s.cols && rows == s.rows) {
		return
	}
	if cols != s.cols { // a reflow changes every row under the selection
		s.sel.clear()
	}
	if s.alt_active {
		s.lines = fit(s.lines, cols, rows)
		s.main, s.main_saved.x, s.main_saved.y = s.reflow_rows(s.main, s.main_saved.x,
			s.main_saved.y, cols, rows)
		s.x, s.y = clamp(s.x, 0, cols - 1), clamp(s.y, 0, rows - 1)
	} else {
		s.lines, s.x, s.y = s.reflow_rows(s.lines, s.x, s.y, cols, rows)
	}
	s.cols, s.rows = cols, rows
	s.top, s.bottom = 0, rows - 1
	s.wrap_pending = false
	s.touch_range(0, rows - 1)
}

// fit cuts or pads rows and cells, keeping the top (the alternate screen).
fn fit(grid []Row, cols int, rows int) []Row {
	mut out := []Row{cap: rows}
	for i in 0 .. rows {
		mut r := if i < grid.len { grid[i] } else { blank_row(cols, 0) }
		if r.cells.len > cols {
			r.cells = r.cells[..cols].clone()
			r.marks = r.marks.filter(it.col < cols)
		} else if r.cells.len < cols {
			r.cells << []Cell{len: cols - r.cells.len}
		}
		r.wrapped = false
		out << r
	}
	return out
}

// reflow_rows joins the scrollback and grid into logical lines, splits them at the new
// width and keeps the last rows on screen, the cursor's line visible.
fn (mut s Screen) reflow_rows(grid []Row, cx int, cy int, cols int, rows int) ([]Row, int, int) {
	if grid.len == 0 {
		return grid, cx, cy
	}
	// Only the height changed: lines move between the grid and the scrollback.
	if grid[0].cells.len == cols {
		return s.regrow(grid, cx, cy, rows)
	}
	mut last := cy
	for i := grid.len - 1; i > cy; i-- {
		if row_used(grid[i]) {
			last = i
			break
		}
	}
	// History lines are reflowed as packed text; only the chain that runs on into the grid
	// goes through cells with the grid (it may hold the cursor).
	n := s.scrollback.len()
	mut tail := n // the chain running on into the grid starts here
	for tail > 0 && s.scrollback.at(tail - 1).wrapped {
		tail--
	}
	mut src := []Row{cap: n - tail + last + 1}
	for h in tail .. n {
		l := s.scrollback.at(h)
		src << unpack(l, l.text.len * 2 + 1)
	}
	base := src.len
	src << grid[..last + 1]
	s.reflow_history(tail, cols)
	mut out := []Row{}
	mut cur_row, mut cur_x := 0, 0
	mut line := LogicalLine{}
	mut cursor_off := -1
	for i, r in src {
		if i == base + cy {
			cursor_off = line.cells.len + cx
		}
		line.add(r)
		if r.wrapped && i < src.len - 1 {
			continue
		}
		pieces, starts := line.split(cols)
		if cursor_off >= 0 {
			mut k := 0
			for k + 1 < starts.len && starts[k + 1] <= cursor_off {
				k++
			}
			cur_row, cur_x = out.len + k, cursor_off - starts[k]
			cursor_off = -1
		}
		out << pieces
		line = LogicalLine{}
	}
	// The grid shows the last rows, and the cursor row in any case.
	mut first := if out.len > rows { out.len - rows } else { 0 }
	if cur_row < first {
		first = cur_row
	}
	end := if first + rows < out.len { first + rows } else { out.len }
	for i in 0 .. first {
		s.to_history(pack(out[i]))
	}
	mut g := out[first..end].clone()
	for g.len < rows {
		g << blank_row(cols, 0)
	}
	return g, clamp(cur_x, 0, cols - 1), clamp(cur_row - first, 0, rows - 1)
}

// regrow handles a height change at the same width.
fn (mut s Screen) regrow(grid []Row, cx int, cy int, rows int) ([]Row, int, int) {
	mut g := grid.clone()
	mut y := cy
	cols := grid[0].cells.len
	for g.len > rows { // drop blank rows below the cursor first, then push the top away
		// With the cursor on the top row, rows below it go: pushing the top away would take
		// the cursor off the grid (y went negative).
		if g.len - 1 > y && (!row_used(g.last()) || y <= 0) {
			g.pop()
			continue
		}
		s.to_history(pack(g[0]))
		g.delete(0)
		y--
	}
	for g.len < rows {
		if l := s.scrollback.pop() {
			g.insert(0, unpack(l, cols))
			y++
		} else {
			g << blank_row(cols, 0)
		}
	}
	return g, cx, y
}

fn (mut l LogicalLine) add(r Row) {
	mut end := r.cells.len
	if !r.wrapped {
		for end > 0 && is_blank(r.cells[end - 1]) {
			end--
		}
	} else {
		// Never-written cells: the pad a wide character left by wrapping early, or the
		// width an unpacked scrollback line was given. Printed spaces are 32.
		for end > 0 && r.cells[end - 1].ch == 0 && r.cells[end - 1].flags == 0 {
			end--
		}
	}
	off := l.cells.len
	l.cells << r.cells[..end]
	for m in r.marks {
		if m.col < end {
			l.marks << Mark{m.col + off, m.s}
		}
	}
	l.extra = merge_extra(l.extra, r.extra)
}

// row_used says whether a row holds anything a resize must keep.
fn row_used(r Row) bool {
	return r.extra != unsafe { nil } || pack(r).text != ''
}

// merge_extra joins the marks and images of rows that become one logical line.
fn merge_extra(a &RowExtra, b &RowExtra) &RowExtra {
	if b == unsafe { nil } {
		return a
	}
	if a == unsafe { nil } {
		return b
	}
	mut m := &RowExtra{
		marks:  a.marks | b.marks
		exit:   if b.marks & mark_exit != 0 { b.exit } else { a.exit }
		images: a.images
	}
	m.images << b.images
	return m
}

// split cuts a logical line into rows of cols cells (a wide character never straddles) and
// tells where each row starts in the line. At one column a wide character fills a row alone,
// without its spacer, as print puts it there (waiting for a second column never ended).
fn (l &LogicalLine) split(cols int) ([]Row, []int) {
	mut out := []Row{}
	mut starts := []int{}
	mut i := 0
	for {
		starts << i
		mut row := blank_row(cols, 0)
		mut x := 0
		for i < l.cells.len && x < cols {
			c := l.cells[i]
			if c.flags & f_wide != 0 && x == cols - 1 && x > 0 {
				break
			}
			row.cells[x] = c
			for m in l.marks {
				if m.col == i {
					row.marks << Mark{x, m.s}
				}
			}
			i++
			x++
			if cols == 1 && c.flags & f_wide != 0 && i < l.cells.len
				&& l.cells[i].flags & f_spacer != 0 {
				i++
			}
		}
		row.wrapped = i < l.cells.len
		if out.len == 0 {
			row.extra = l.extra
		}
		out << row
		if !row.wrapped {
			break
		}
	}
	return out, starts
}

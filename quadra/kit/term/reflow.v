// Resize: the main screen reflows wrapped lines to the new width; the alternate screen is
// cut or padded (specs/17).
module term

struct LogicalLine {
mut:
	cells []Cell
	marks []Mark
}

pub fn (mut s Screen) resize(cols int, rows int) {
	if cols < 1 || rows < 1 || (cols == s.cols && rows == s.rows) {
		return
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
		if pack(grid[i]).text != '' {
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
		s.scrollback.push(pack(out[i]))
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
		if g.len - 1 > y && pack(g.last()).text == '' {
			g.pop()
			continue
		}
		s.scrollback.push(pack(g[0]))
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
}

// split cuts a logical line into rows of cols cells (a wide character never straddles) and
// tells where each row starts in the line.
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
			if c.flags & f_wide != 0 && x == cols - 1 {
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
		}
		row.wrapped = i < l.cells.len
		out << row
		if !row.wrapped {
			break
		}
	}
	return out, starts
}

// split_history reflows the history chain i..j (wrapped lines that form one logical line).
// ASCII chains are cut as text; others go through cells.
fn (s &Screen) split_history(i int, j int, cols int) []Line {
	mut ascii := true
	for h in i .. j + 1 {
		for b in s.scrollback.at(h).text {
			if b >= 0x80 {
				ascii = false
				break
			}
		}
	}
	if !ascii {
		mut line := LogicalLine{}
		for h in i .. j + 1 {
			l := s.scrollback.at(h)
			line.add(unpack(l, l.text.len * 2 + 1))
		}
		pieces, _ := line.split(cols)
		return pieces.map(pack(it))
	}
	if i == j && s.scrollback.at(i).text.len <= cols {
		return [s.scrollback.at(i)] // fits: kept as it is
	}
	mut text := ''
	mut runs := []Run{}
	for h in i .. j + 1 {
		l := s.scrollback.at(h)
		off := text.len
		for r in l.runs {
			runs << Run{u16(int(r.col) + off), r.fg, r.bg, r.flags}
		}
		text += l.text
	}
	mut out := []Line{}
	mut from := 0
	mut ri := 0 // first run not yet started
	mut active := ?Run(none)
	for {
		end := if from + cols < text.len { from + cols } else { text.len }
		mut piece := []Run{}
		for ri < runs.len && int(runs[ri].col) <= from {
			active = runs[ri]
			ri++
		}
		if a := active {
			piece << Run{0, a.fg, a.bg, a.flags}
		}
		for x := ri; x < runs.len && int(runs[x].col) < end; x++ {
			piece << Run{u16(int(runs[x].col) - from), runs[x].fg, runs[x].bg, runs[x].flags}
		}
		out << Line{text[from..end], piece, end < text.len}
		if end >= text.len {
			break
		}
		from = end
	}
	return out
}

// reflow_history replaces the history lines before tail with their reflow at cols. It is a
// function of its own, and the result is assigned once: under -prod with boehm_full_opt,
// V pins every local array of pointerful structs around each call statement in scope, so a
// 10 000-line array held across a loop of calls costs O(n²) (specs/14).
fn (mut s Screen) reflow_history(tail int, cols int) {
	mut hist := []Line{cap: tail}
	mut h0 := 0
	for h0 < tail {
		mut j := h0
		for j < tail - 1 && s.scrollback.at(j).wrapped {
			j++
		}
		hist << s.split_history(h0, j, cols)
		h0 = j + 1
	}
	if hist.len > s.scrollback.max { // narrower rows make more lines: keep the newest
		hist = hist[hist.len - s.scrollback.max..].clone()
	}
	s.scrollback.lines, s.scrollback.head = hist, 0
}

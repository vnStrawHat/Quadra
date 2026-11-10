// Scrollback: a ring of compact lines. A row is stored as UTF-8 text plus style runs, so a
// plain 40-character line costs about 100 bytes instead of 16 bytes per cell (specs/17).
module term

import strings

// Run: the style that starts at col and lasts until the next run.
pub struct Run {
pub:
	col   u16
	fg    u32
	bg    u32
	flags u16
	link  u16
}

pub struct Line {
pub:
	text    string
	runs    []Run // empty: default style throughout
	wrapped bool
	extra   &RowExtra = unsafe { nil } // shell marks and images, rare
}

pub struct Mark {
pub:
	col int
	s   string // combining marks on the cell at col
}

pub struct Row {
pub mut:
	cells   []Cell
	marks   []Mark
	wrapped bool // the line continues on the next row
	gen     u32  // bumped on change; the view caches by it
	extra   &RowExtra = unsafe { nil } // shell marks and images, rare
}

fn blank_row(cols int, bg u32) Row {
	return Row{
		cells: []Cell{len: cols, init: Cell{
			bg: bg
		}}
	}
}

@[inline]
fn is_blank(c Cell) bool {
	return (c.ch == 0 || c.ch == 32 || c.ch == 9) && c.bg == 0
		&& c.flags & (f_inverse | f_underline | f_strike) == 0
}

// pack trims trailing blanks and stores the row as text and runs.
pub fn pack(r Row) Line {
	mut end := r.cells.len
	if r.wrapped { // spaces before a wrap belong to the line; only never-written cells go
		for end > 0 && r.cells[end - 1].ch == 0 && r.cells[end - 1].flags == 0 {
			end--
		}
	} else {
		for end > 0 && is_blank(r.cells[end - 1]) {
			end--
		}
	}
	// Count the runs first: the history keeps these arrays, so they get their exact size
	// (grown by appends they would hold up to twice that).
	mut nruns := 0
	mut prev := Cell{}
	for col in 0 .. end {
		c := r.cells[col]
		if c.flags & f_spacer == 0 && !c.same_style(prev) {
			nruns++
			prev = c
		}
	}
	mut sb := strings.new_builder(end + 1)
	mut runs := []Run{cap: nruns}
	prev = Cell{}
	mut mi := 0
	for col in 0 .. end {
		c := r.cells[col]
		if c.flags & f_spacer != 0 {
			continue
		}
		if !c.same_style(prev) {
			runs << Run{u16(col), c.fg, c.bg, c.flags & ~(f_wide | f_spacer), c.pad}
			prev = c
		}
		if c.ch == 0 {
			sb << u8(32)
		} else if c.ch < 128 {
			sb << u8(c.ch)
		} else {
			sb.write_rune(rune(c.ch))
		}
		for mi < r.marks.len && r.marks[mi].col <= col {
			if r.marks[mi].col == col {
				sb.write_string(r.marks[mi].s)
			}
			mi++
		}
	}
	// The builder is not reused, so the line text takes its buffer (NUL-terminated) as is.
	n := sb.len
	sb << u8(0)
	return Line{
		text:    unsafe { tos(sb.data, n) }
		runs:    runs
		wrapped: r.wrapped
		extra:   r.extra
	}
}

// unpack lays a line out on cols cells (extra text is cut: reflow splits lines before).
pub fn unpack(l Line, cols int) Row {
	mut row := blank_row(cols, 0)
	row.wrapped, row.extra = l.wrapped, l.extra
	mut col := 0
	mut ri := 0
	mut style := Cell{}
	for r in l.text.runes() {
		w := width(u32(r))
		if w == 0 {
			if col > 0 {
				row.add_mark(col - 1, r.str())
			}
			continue
		}
		if col + w > cols {
			break
		}
		for ri < l.runs.len && int(l.runs[ri].col) <= col {
			run := l.runs[ri]
			style = Cell{
				fg:    run.fg
				bg:    run.bg
				flags: run.flags
				pad:   run.link
			}
			ri++
		}
		row.cells[col] = Cell{
			...style
			ch:    u32(r) // spaces stay 32: cells past the text (0) are padding
			flags: style.flags | if w == 2 { f_wide } else { u16(0) }
		}
		if w == 2 {
			row.cells[col + 1] = Cell{
				...style
				flags: style.flags | f_spacer
			}
		}
		col += w
	}
	return row
}

fn (mut r Row) add_mark(col int, s string) {
	for i, m in r.marks {
		if m.col == col {
			if m.s.len < 64 { // a cluster cannot grow without bound
				r.marks[i] = Mark{col, m.s + s}
			}
			return
		}
	}
	r.marks << Mark{col, s}
	r.marks.sort(a.col < b.col)
}

pub struct Scrollback {
mut:
	lines []Line
	head  int // index of the oldest line once the ring is full
pub mut:
	max int = 10000
}

pub fn (s &Scrollback) len() int {
	return s.lines.len
}

// at returns line i, 0 being the oldest.
pub fn (s &Scrollback) at(i int) Line {
	return s.lines[(s.head + i) % s.lines.len]
}

// full says the next push evicts a line (or stores none).
pub fn (s &Scrollback) full() bool {
	return s.max <= 0 || s.lines.len >= s.max
}

pub fn (mut s Scrollback) push(l Line) {
	if s.max <= 0 {
		return
	}
	if s.lines.len < s.max {
		s.lines << l
		return
	}
	s.lines[s.head] = l
	s.head = (s.head + 1) % s.lines.len
}

// pop removes and returns the newest line (rows coming back when the screen grows).
pub fn (mut s Scrollback) pop() ?Line {
	if s.lines.len == 0 {
		return none
	}
	if s.head == 0 {
		return s.lines.pop()
	}
	// Unroll the ring so the newest line sits at the end.
	mut flat := []Line{cap: s.lines.len}
	for i in 0 .. s.lines.len {
		flat << s.at(i)
	}
	s.lines, s.head = flat, 0
	return s.lines.pop()
}

pub fn (mut s Scrollback) clear() {
	s.lines = []
	s.head = 0
}

// bytes estimates the heap the history holds: each line's record, text and runs (memory
// reports; walks every line).
pub fn (s &Scrollback) bytes() i64 {
	mut n := i64(s.lines.cap) * i64(sizeof(Line))
	for l in s.lines {
		n += l.text.len + 1 + l.runs.cap * int(sizeof(Run))
	}
	return n
}

// grid_bytes is what the visible grids hold: the shown one and the main one behind the
// alternate screen.
pub fn (s &Screen) grid_bytes() i64 {
	mut n := i64(0)
	for rows in [s.lines, s.main] {
		n += i64(rows.cap) * i64(sizeof(Row))
		for r in rows {
			n += i64(r.cells.cap) * i64(sizeof(Cell))
		}
	}
	return n
}

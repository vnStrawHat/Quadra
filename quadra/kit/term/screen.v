// Screen: the grid, cursor and modes a parser drives (specs/17).
module term

pub enum CursorShape {
	block
	underline
	bar
}

pub enum MouseMode {
	off
	click // 1000
	drag  // 1002
}

struct Saved {
mut:
	x   int
	y   int
	pen Cell
}

@[heap]
pub struct Screen {
pub mut:
	cols       int
	rows       int
	lines      []Row // the visible grid (main or alternate)
	scrollback Scrollback
	x          int // cursor column
	y          int // cursor row
	pen        Cell
	title      string
	replies    []u8    // bytes the terminal answers with (DSR, DA): the session sends them
	clipboard  ?string // set by OSC 52 when allowed
	// modes
	autowrap     bool = true
	cursor_on    bool = true
	shape        CursorShape
	app_cursor   bool
	app_keypad   bool
	bracketed    bool
	mouse        MouseMode
	mouse_sgr    bool
	allow_osc52  bool
	origin       bool
	alt_active   bool
	line_drawing bool // DEC special graphics in G0
	gen          u32  // bumped on any change
mut:
	top          int // scroll region, inclusive
	bottom       int
	wrap_pending bool
	saved        Saved
	main         []Row // the main grid while the alternate one is shown
	main_saved   Saved
}

pub fn new_screen(cols int, rows int) &Screen {
	mut s := &Screen{
		cols: cols
		rows: rows
	}
	s.lines = []Row{len: rows, init: blank_row(cols, 0)}
	s.bottom = rows - 1
	return s
}

fn (mut s Screen) touch(y int) {
	s.gen++
	s.lines[y].gen = s.gen
}

// print writes one code point at the cursor with the pen, wrapping when needed.
pub fn (mut s Screen) print(r u32) {
	mut ch := r
	if s.line_drawing && r >= 0x60 && r <= 0x7e {
		ch = u32(dec_special[r - 0x60])
	}
	w := width(ch)
	if w == 0 {
		x := if s.wrap_pending { s.x } else { s.x - 1 }
		if x >= 0 {
			s.lines[s.y].add_mark(x, rune(ch).str())
			s.touch(s.y)
		}
		return
	}
	if s.wrap_pending || (w == 2 && s.x == s.cols - 1) {
		if s.autowrap {
			s.lines[s.y].wrapped = true
			s.x = 0
			s.index()
		}
		s.wrap_pending = false
	}
	s.clear_wide_at(s.x)
	s.lines[s.y].cells[s.x] = Cell{
		...s.pen
		ch:    ch
		flags: s.pen.flags | if w == 2 { f_wide } else { u16(0) }
	}
	if w == 2 && s.x + 1 < s.cols {
		s.clear_wide_at(s.x + 1)
		s.lines[s.y].cells[s.x + 1] = Cell{
			...s.pen
			flags: s.pen.flags | f_spacer
		}
	}
	s.touch(s.y)
	if s.x + w >= s.cols {
		s.x = s.cols - 1
		s.wrap_pending = true
	} else {
		s.x += w
	}
}

// print_ascii is the fast path for runs of printable ASCII.
pub fn (mut s Screen) print_ascii(b []u8) {
	if s.line_drawing {
		for c in b {
			s.print(u32(c))
		}
		return
	}
	for c in b {
		if s.wrap_pending {
			if s.autowrap {
				s.lines[s.y].wrapped = true
				s.x = 0
				s.index()
			}
			s.wrap_pending = false
		}
		if s.lines[s.y].cells[s.x].flags & (f_wide | f_spacer) != 0 {
			s.clear_wide_at(s.x)
		}
		s.lines[s.y].cells[s.x] = Cell{
			...s.pen
			ch: u32(c)
		}
		if s.x + 1 >= s.cols {
			s.wrap_pending = true
		} else {
			s.x++
		}
	}
	s.touch(s.y)
}

// clear_wide_at blanks both halves of a wide character that overlaps column x.
fn (mut s Screen) clear_wide_at(x int) {
	c := s.lines[s.y].cells[x]
	if c.flags & f_spacer != 0 && x > 0 {
		s.lines[s.y].cells[x - 1] = Cell{
			bg: s.lines[s.y].cells[x - 1].bg
		}
	} else if c.flags & f_wide != 0 && x + 1 < s.cols {
		s.lines[s.y].cells[x + 1] = Cell{
			bg: s.lines[s.y].cells[x + 1].bg
		}
	}
}

// index moves down one line, scrolling the region at its bottom (LF, IND).
pub fn (mut s Screen) index() {
	if s.y == s.bottom {
		s.scroll_up(1)
	} else if s.y < s.rows - 1 {
		s.y++
	}
}

// reverse_index moves up one line, scrolling the region down at its top (RI).
pub fn (mut s Screen) reverse_index() {
	if s.y == s.top {
		s.scroll_down(1)
	} else if s.y > 0 {
		s.y--
	}
}

pub fn (mut s Screen) linefeed() {
	s.wrap_pending = false
	s.index()
}

pub fn (mut s Screen) carriage_return() {
	s.x = 0
	s.wrap_pending = false
}

pub fn (mut s Screen) backspace() {
	if s.wrap_pending {
		s.wrap_pending = false
	} else if s.x > 0 {
		s.x--
	}
}

pub fn (mut s Screen) tab() {
	s.x = (s.x / 8 + 1) * 8
	if s.x >= s.cols {
		s.x = s.cols - 1
	}
}

// scroll_up moves the region up n lines; lines leaving the top of a full-height main
// screen go to scrollback.
pub fn (mut s Screen) scroll_up(n int) {
	k := clamp(n, 1, s.bottom - s.top + 1)
	blank := Cell{
		bg: s.pen.bg
	}
	for _ in 0 .. k {
		mut row := s.lines[s.top]
		if s.top == 0 && !s.alt_active {
			s.scrollback.push(pack(row))
		}
		// The row that left is cleared and reused at the bottom: no allocation per line.
		if blank.bg == 0 {
			unsafe { vmemset(row.cells.data, 0, row.cells.len * int(sizeof(Cell))) }
		} else {
			for i in 0 .. row.cells.len {
				row.cells[i] = blank
			}
		}
		row.marks, row.wrapped = []Mark{}, false
		s.lines.delete(s.top)
		s.lines.insert(s.bottom, row)
	}
	s.touch_range(s.top, s.bottom)
}

pub fn (mut s Screen) scroll_down(n int) {
	k := clamp(n, 1, s.bottom - s.top + 1)
	for _ in 0 .. k {
		s.lines.delete(s.bottom)
		s.lines.insert(s.top, blank_row(s.cols, s.pen.bg))
	}
	s.touch_range(s.top, s.bottom)
}

fn (mut s Screen) touch_range(a int, b int) {
	for y in a .. b + 1 {
		s.touch(y)
	}
}

fn clamp(v int, lo int, hi int) int {
	return if v < lo {
		lo
	} else if v > hi {
		hi
	} else {
		v
	}
}

// text returns the visible grid as lines without trailing blanks (tests and copy).
pub fn (s &Screen) text() string {
	mut out := []string{cap: s.rows}
	for r in s.lines {
		out << pack(r).text
	}
	for out.len > 0 && out.last() == '' {
		out.pop()
	}
	return out.join('\n')
}

pub fn (s &Screen) line(y int) string {
	return pack(s.lines[y]).text
}

// DEC special graphics for 0x60..0x7e (line drawing in G0).
const dec_special = [`◆`, `▒`, `␉`, `␌`, `␍`, `␊`, `°`, `±`, `␤`, `␋`, `┘`,
	`┐`, `┌`, `└`, `┼`, `⎺`, `⎻`, `─`, `⎼`, `⎽`, `├`, `┤`, `┴`, `┬`,
	`│`, `≤`, `≥`, `π`, `≠`, `£`, `·`]

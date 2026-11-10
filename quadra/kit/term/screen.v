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
	any   // 1003: motion without a button too
	x10   // 9: presses only, no modifiers
}

pub enum MouseEncoding {
	normal
	utf8  // 1005
	sgr   // 1006
	urxvt // 1015
}

struct Saved {
mut:
	x        int
	y        int
	pen      Cell
	charsets [4]u8
	active   int
	shift    int = -1
	wrap     bool // the next character wraps
}

@[heap]
pub struct Screen {
pub mut:
	cols       int
	rows       int
	lines      []Row // the visible grid (main or alternate)
	scrollback Scrollback
	sel        Selection // anchored to rows: every row-moving operation updates it
	x          int       // cursor column
	y          int       // cursor row
	pen        Cell
	// Read on every printed cell: kept next to the cursor, away from the big tables below.
	top          int // scroll region, inclusive
	bottom       int
	wrap_pending bool
	title        string
	cwd          string  // from OSC 7
	replies      []u8    // bytes the terminal answers with (DSR, DA): the session sends them
	clipboard    ?string // set by OSC 52 when allowed
	// modes
	autowrap              bool = true
	cursor_on             bool = true
	shape                 CursorShape
	cursor_blink          int // 0 default, 1 on, 2 off (DECSCUSR, mode 12)
	app_cursor            bool
	app_keypad            bool
	bracketed             bool
	mouse                 MouseMode
	mouse_enc             MouseEncoding
	reverse_video         bool
	reverse_wrap          bool
	focus_reports         bool
	urgency               bool = true
	grapheme              bool // mode 2027
	sync                  bool // mode 2026
	product_name          string = 'Quadra'
	product_version       int    = 10
	modify_other          int
	keyboard              KeyboardStacks
	allow_screen_readback bool // answers DECRQCRA; off by default
	allow_osc52_read      bool
	allow_osc52           bool
	origin                bool
	alt_active            bool
	line_drawing          bool // a non-ASCII charset is in play: the print path looks
	charsets              [4]u8
	active_cs             int
	single_shift          int = -1
	insert                bool   // IRM
	stops                 []bool // tab stops once a program changes them (tabstops.v)
	last_ch               u32    // for REP
	w_ch                  u32    // the last code point print measured, and its width
	w_w                   int = 1
	gen                   u32 // bumped on any change
	// what the app reads (specs/17)
	icon_name      string
	title_stack    []string
	title_count    int // OSC 0, 2 and CSI 23 t seen
	has_over       bool
	color_queries  []ColorQuery
	events         []TerminalInfo
	progress       Progress
	program_status ?ProgramState
	status         []StatusRecord
	status_tick    u64
	prompt_count   int
	last_exit      ?int
	// images
	graphics     []&Graphic
	images_off   bool // after Terminal.close: new images are dropped
	next_graphic u32
	sweep_count  u32
	cell_w       int = 10 // device px of a cell (image placement), set by the view
	cell_h       int = 20
	extras       []CellExtra // what Cell.pad indexes
	extra_ids    map[u64]u16
	links        []LinkTarget
	over         [261]u32 // dynamic colors, see colors.v
	// grapheme-cluster state of the print path (mode 2027)
	cl_ok  bool
	cl_x   int
	cl_y   int
	cl_zwj bool
	cl_ri  bool
mut:
	saved           Saved
	bell            bool // BEL since the view last looked
	alt_scroll      bool = true // the wheel sends arrows on the alternate screen (1007)
	clipboard_query bool  // a program asked to read the clipboard
	main            []Row // the main grid while the alternate one is shown
	main_saved      Saved
	alt_saved       Saved // DECSC of the alternate screen, kept while it is not shown
}

// alt_scroll_on says whether the wheel sends arrows on the alternate screen (mode 1007).
pub fn (s &Screen) alt_scroll_on() bool {
	return s.alt_scroll
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

// backspace moves left; with reverse wrap (mode 45) column 0 goes up onto a wrapped row.
pub fn (mut s Screen) backspace() {
	if s.x == 0 {
		if s.reverse_wrap && s.y > s.top && s.lines[s.y - 1].wrapped {
			s.y--
			s.x = s.cols - 1
			s.wrap_pending = false
		}
		return
	}
	s.x--
	s.wrap_pending = false
}

// scroll_up moves the region up n lines; lines leaving the top of a full-height main
// screen go to scrollback.
pub fn (mut s Screen) scroll_up(n int) {
	k := clamp(n, 1, s.bottom - s.top + 1)
	if s.sel.active {
		if s.top == 0 && !s.alt_active {
			// Rows of the region keep their numbers; rows below it are pushed down by the
			// lines the history just gained.
			if s.bottom < s.rows - 1 {
				s.sel_shift(s.bottom + 1, s.rows - 1, k, -1, -2)
			}
		} else {
			s.sel_shift(s.top, s.bottom, -k, s.top, s.top + k - 1)
		}
	}
	blank := Cell{
		bg: s.pen.bg
	}
	for _ in 0 .. k {
		mut row := s.lines[s.top]
		if s.top == 0 && !s.alt_active {
			s.to_history(pack(row))
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
		row.extra = unsafe { nil }
		s.lines.delete(s.top)
		s.lines.insert(s.bottom, row)
	}
	s.touch_range(s.top, s.bottom)
}

pub fn (mut s Screen) scroll_down(n int) {
	k := clamp(n, 1, s.bottom - s.top + 1)
	s.sel_shift(s.top, s.bottom, k, s.bottom - k + 1, s.bottom)
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
		out << pack(r).text.replace('\t', ' ')
	}
	for out.len > 0 && out.last() == '' {
		out.pop()
	}
	return out.join('\n')
}

pub fn (s &Screen) line(y int) string {
	return pack(s.lines[y]).text
}

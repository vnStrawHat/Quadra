// Screen operations behind the CSI and ESC sequences: cursor moves, erase, insert and
// delete, scroll regions, save and restore, the alternate screen.
module term

// goto moves the cursor to (x, y), 0-based; y is relative to the region in origin mode.
pub fn (mut s Screen) goto(x int, y int) {
	s.wrap_pending = false
	s.x = clamp(x, 0, s.cols - 1)
	s.y = if s.origin { clamp(y + s.top, s.top, s.bottom) } else { clamp(y, 0, s.rows - 1) }
}

// move shifts the cursor; vertical moves stop at the region edges when inside it.
pub fn (mut s Screen) move(dx int, dy int) {
	s.wrap_pending = false
	s.x = clamp(s.x + dx, 0, s.cols - 1)
	if dy != 0 {
		inside := s.y >= s.top && s.y <= s.bottom
		lo := if inside { s.top } else { 0 }
		hi := if inside { s.bottom } else { s.rows - 1 }
		s.y = clamp(s.y + dy, lo, hi)
	}
}

fn (s &Screen) blank() Cell {
	return Cell{
		bg: s.pen.bg
	}
}

fn (mut s Screen) fill(y int, a int, b int) {
	blank := s.blank()
	for x in clamp(a, 0, s.cols) .. clamp(b, 0, s.cols) {
		s.lines[y].cells[x] = blank
	}
	s.lines[y].marks = s.lines[y].marks.filter(it.col < a || it.col >= b)
	if s.lines[y].extra != unsafe { nil } {
		s.lines[y].drop_images(a, b)
	}
	s.touch(y)
}

// erase_line: 0 to the end, 1 to the start (inclusive), 2 the whole line (EL).
pub fn (mut s Screen) erase_line(mode int) {
	s.sel_on_erase_line()
	if mode == 0 && s.wrap_pending {
		return
	}
	match mode {
		0 { s.fill(s.y, s.x, s.cols) }
		1 { s.fill(s.y, 0, s.x + 1) }
		else { s.fill(s.y, 0, s.cols) }
	}

	if mode != 1 {
		s.lines[s.y].wrapped = false
	}
}

// erase_display: 0 below, 1 above, 2 all, 3 all and the scrollback (ED).
pub fn (mut s Screen) erase_display(mode int) {
	s.sel_on_erase_display(mode)
	match mode {
		0 {
			s.erase_line(0)
			for y in s.y + 1 .. s.rows {
				s.fill(y, 0, s.cols)
				s.lines[y].wrapped = false
			}
		}
		1 {
			for y in 0 .. s.y {
				s.fill(y, 0, s.cols)
				s.lines[y].wrapped = false
			}
			s.erase_line(1)
		}
		else {
			for y in 0 .. s.rows {
				s.fill(y, 0, s.cols)
				s.lines[y].wrapped = false
			}
			if mode == 3 {
				s.scrollback.clear()
			}
		}
	}
}

// erase_chars blanks n cells from the cursor without moving the rest (ECH).
pub fn (mut s Screen) erase_chars(n int) {
	s.fill(s.y, s.x, s.x + clamp(n, 1, s.cols))
}

// insert_chars shifts the rest of the line right by n blanks (ICH).
pub fn (mut s Screen) insert_chars(n int) {
	k := clamp(n, 1, s.cols - s.x)
	blank := s.blank()
	for x := s.cols - 1; x >= s.x + k; x-- {
		s.lines[s.y].cells[x] = s.lines[s.y].cells[x - k]
	}
	for x in s.x .. s.x + k {
		s.lines[s.y].cells[x] = blank
	}
	s.touch(s.y)
	s.wrap_pending = false
}

// delete_chars removes n cells at the cursor; the rest moves left (DCH).
pub fn (mut s Screen) delete_chars(n int) {
	k := clamp(n, 1, s.cols - s.x)
	blank := s.blank()
	for x in s.x .. s.cols - k {
		s.lines[s.y].cells[x] = s.lines[s.y].cells[x + k]
	}
	for x in s.cols - k .. s.cols {
		s.lines[s.y].cells[x] = blank
	}
	s.touch(s.y)
	s.wrap_pending = false
}

// insert_lines and delete_lines work inside the region from the cursor row (IL, DL).
pub fn (mut s Screen) insert_lines(n int) {
	if s.y < s.top || s.y > s.bottom {
		return
	}
	k := clamp(n, 1, s.bottom - s.y + 1)
	s.sel_shift(s.y, s.bottom, k, s.bottom - k + 1, s.bottom)
	for _ in 0 .. k {
		s.lines.delete(s.bottom)
		s.lines.insert(s.y, blank_row(s.cols, s.pen.bg))
	}
	s.touch_range(s.y, s.bottom)
	s.x = 0
}

pub fn (mut s Screen) delete_lines(n int) {
	if s.y < s.top || s.y > s.bottom {
		return
	}
	k := clamp(n, 1, s.bottom - s.y + 1)
	s.sel_shift(s.y, s.bottom, -k, s.y, s.y + k - 1)
	for _ in 0 .. k {
		s.lines.delete(s.y)
		s.lines.insert(s.bottom, blank_row(s.cols, s.pen.bg))
	}
	s.touch_range(s.y, s.bottom)
	s.x = 0
}

// set_region sets the scroll region (1-based, inclusive; 0 = default) and homes (DECSTBM).
pub fn (mut s Screen) set_region(top int, bottom int) {
	t := if top <= 0 { 0 } else { top - 1 }
	b := if bottom <= 0 || bottom > s.rows { s.rows - 1 } else { bottom - 1 }
	if t >= b {
		return
	}
	s.top, s.bottom = t, b
	s.goto(0, 0)
}

pub fn (mut s Screen) save_cursor() {
	s.saved = s.snapshot()
}

pub fn (mut s Screen) restore_cursor() {
	s.apply_saved(s.saved)
}

// set_alt switches to the alternate screen (cleared, no scrollback) and back (1049).
pub fn (mut s Screen) set_alt(on bool) {
	if on == s.alt_active {
		return
	}
	s.sel.clear()
	if on {
		s.main_saved = s.snapshot()
		s.saved = s.alt_saved
		s.main = s.lines
		s.lines = []Row{len: s.rows, init: blank_row(s.cols, 0)}
	} else {
		s.lines = s.main
		s.main = []
		s.alt_saved = s.saved
		s.saved = s.main_saved
		s.apply_saved(s.main_saved)
	}
	s.alt_active = on
	s.keyboard.swap()
	s.top, s.bottom = 0, s.rows - 1
	s.touch_range(0, s.rows - 1)
}

// reset returns to the power-on state and drops the scrollback (RIS).
pub fn (mut s Screen) reset() {
	s.set_alt(false)
	s.sel.clear()
	s.pen = Cell{}
	s.autowrap, s.cursor_on, s.shape, s.cursor_blink = true, true, .block, 0
	s.app_cursor, s.app_keypad, s.bracketed, s.origin = false, false, false, false
	s.mouse, s.mouse_enc = .off, .normal
	s.reverse_video, s.reverse_wrap, s.focus_reports, s.grapheme, s.sync = false, false, false, false, false
	s.urgency, s.modify_other = true, 0
	s.charsets, s.active_cs, s.single_shift, s.line_drawing = [4]u8{}, 0, -1, false
	s.keyboard = KeyboardStacks{}
	s.extras, s.extra_ids, s.graphics = [], map[u64]u16{}, []
	s.title_stack, s.icon_name, s.title = [], '', ''
	s.over, s.has_over = [261]u32{}, false
	s.status, s.program_status, s.progress = [], none, Progress{}
	s.top, s.bottom = 0, s.rows - 1
	s.insert, s.last_ch = false, 0
	s.saved, s.alt_saved = Saved{}, Saved{}
	s.stops, s.links = [], []
	s.alt_scroll = true
	s.erase_display(2)
	s.scrollback.clear()
	s.goto(0, 0)
}

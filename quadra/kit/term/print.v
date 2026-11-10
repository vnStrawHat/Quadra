// The print path: charsets, combining marks, grapheme clusters (mode 2027) and wide cells.
module term

const cs_ascii = u8(0)
const cs_dec = u8(1)

// dec_special is DEC special graphics for 0x60..0x7e (line drawing).
const dec_special = [`◆`, `▒`, `␉`, `␌`, `␍`, `␊`, `°`, `±`, `␤`, `␋`, `┘`,
	`┐`, `┌`, `└`, `┼`, `⎺`, `⎻`, `─`, `⎼`, `⎽`, `├`, `┤`, `┴`, `┬`,
	`│`, `≤`, `≥`, `π`, `≠`, `£`, `·`]

// print writes one code point at the cursor with the pen, wrapping when needed.
pub fn (mut s Screen) print(r u32) {
	if r >= 0x80 && r < 0xa0 {
		return
	}
	mut ch := r
	if s.line_drawing && s.charset_now() == cs_dec {
		if r == 0x5f {
			ch = 32
		} else if r >= 0x60 && r <= 0x7e {
			ch = u32(dec_special[r - 0x60])
		}
	}
	s.last_ch = r
	if s.grapheme && s.join_cluster(ch) {
		return
	}
	if ch != s.w_ch { // runs of one character (box drawing, blocks) skip the tables
		s.w_ch, s.w_w = ch, width(ch)
	}
	w := s.w_w
	if w == 0 {
		s.attach_mark(if s.wrap_pending { s.x } else { s.x - 1 }, ch)
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
	if s.insert {
		s.insert_chars(w)
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
	s.cl_x, s.cl_y, s.cl_ok, s.cl_zwj = s.x, s.y, true, false
	s.cl_ri = ch >= 0x1f1e6 && ch <= 0x1f1ff
	if s.x + w >= s.cols {
		s.x = s.cols - 1
		s.wrap_pending = true
	} else {
		s.x += w
	}
}

// attach_mark puts a zero-width code point on the cell at x (the base of a wide pair).
fn (mut s Screen) attach_mark(x int, ch u32) {
	if x < 0 {
		return
	}
	mut at := x
	if s.lines[s.y].cells[at].flags & f_spacer != 0 && at > 0 {
		at--
	}
	s.lines[s.y].add_mark(at, rune(ch).str())
	s.touch(s.y)
}

// join_cluster handles mode 2027: a code point that continues the cluster before it stays
// on that cell. ponytail: it decides from what is on the grid, not full segmentation; ZWJ
// chains, skin tones, flags and variation selectors are what programs emit.
fn (mut s Screen) join_cluster(ch u32) bool {
	if !s.cl_ok || s.cl_y >= s.rows {
		return false
	}
	match ch {
		0x200d {
			s.attach_cluster(ch)
			s.cl_zwj = true
			return true
		}
		0xfe0f {
			s.widen_cluster()
			s.attach_cluster(ch)
			return true
		}
		0xfe0e {
			s.narrow_cluster()
			s.attach_cluster(ch)
			return true
		}
		else {}
	}

	if s.cl_zwj || (ch >= 0x1f3fb && ch <= 0x1f3ff) || width(ch) == 0 {
		s.attach_cluster(ch)
		s.cl_zwj = false
		return true
	}
	if s.cl_ri && ch >= 0x1f1e6 && ch <= 0x1f1ff {
		s.cl_ri = false
		s.widen_cluster()
		s.attach_cluster(ch)
		return true
	}
	return false
}

fn (mut s Screen) attach_cluster(ch u32) {
	s.lines[s.cl_y].add_mark(s.cl_x, rune(ch).str())
	s.touch(s.cl_y)
}

// widen_cluster makes the last printed cell two wide, when it fits.
fn (mut s Screen) widen_cluster() {
	if s.cl_x + 1 >= s.cols || s.lines[s.cl_y].cells[s.cl_x].flags & f_wide != 0 {
		return
	}
	s.lines[s.cl_y].cells[s.cl_x].flags |= f_wide
	s.lines[s.cl_y].cells[s.cl_x + 1] = Cell{
		...s.pen
		flags: s.pen.flags | f_spacer
	}
	if s.y == s.cl_y && s.x == s.cl_x + 1 && s.x < s.cols - 1 {
		s.x++
	} else if s.y == s.cl_y && s.x == s.cl_x + 1 {
		s.wrap_pending = true
	}
}

// narrow_cluster makes the last printed cell one wide (VS15 on a wide base).
fn (mut s Screen) narrow_cluster() {
	if s.lines[s.cl_y].cells[s.cl_x].flags & f_wide == 0 {
		return
	}
	s.lines[s.cl_y].cells[s.cl_x].flags &= ~f_wide
	s.lines[s.cl_y].cells[s.cl_x + 1] = Cell{}
	if s.y == s.cl_y && s.x == s.cl_x + 2 {
		s.x = s.cl_x + 1
		s.wrap_pending = false
	}
}

// charset_now is the charset for the next character; a pending single shift is used up.
fn (mut s Screen) charset_now() u8 {
	if s.single_shift >= 0 {
		g := s.charsets[s.single_shift]
		s.single_shift = -1
		s.update_charset()
		return g
	}
	return s.charsets[s.active_cs]
}

// update_charset caches whether the slow print path is needed.
fn (mut s Screen) update_charset() {
	s.line_drawing = s.single_shift >= 0 || s.charsets[s.active_cs] != cs_ascii
}

// designate selects a charset for G0 to G3 (ESC ( ) * +).
fn (mut s Screen) designate(g int, final u8) {
	s.charsets[g] = if final == `0` { cs_dec } else { cs_ascii }
	s.update_charset()
}

// print_ascii is the fast path for runs of printable ASCII.
pub fn (mut s Screen) print_ascii(b []u8) {
	s.last_ch = u32(b[b.len - 1])
	s.cl_ok = false
	if s.line_drawing || s.insert || s.grapheme {
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

// Selection tracking: the selection follows its rows through scrolls and is cleared when
// they are erased, evicted or reflowed (specs/17). Lines are absolute (history + screen).
module term

// sel_move shifts the lines in [lo, hi] by delta unless an end is in [klo, khi]: the content
// of those lines is gone and the selection goes with it.
fn (mut s Screen) sel_move(lo int, hi int, delta int, klo int, khi int) {
	if !s.sel.active {
		return
	}
	if (s.sel.anchor.line >= klo && s.sel.anchor.line <= khi)
		|| (s.sel.head.line >= klo && s.sel.head.line <= khi) {
		s.sel.clear()
		return
	}
	s.sel.anchor = shifted(s.sel.anchor, lo, hi, delta)
	s.sel.head = shifted(s.sel.head, lo, hi, delta)
	s.sel.origin = shifted(s.sel.origin, lo, hi, delta)
}

fn shifted(p Pos, lo int, hi int, delta int) Pos {
	return if p.line >= lo && p.line <= hi { Pos{p.line + delta, p.col} } else { p }
}

// sel_shift is a scroll of the region [top, bottom]: rows in it move by delta, the rows at
// [kill_a, kill_b] of the screen die.
fn (mut s Screen) sel_shift(top int, bottom int, delta int, kill_a int, kill_b int) {
	n := s.scrollback.len()
	s.sel_move(n + top, n + bottom, delta, n + kill_a, n + kill_b)
}

// to_history pushes a line; when that evicts the oldest line, every line number drops by one.
fn (mut s Screen) to_history(l Line) {
	if s.sel.active && s.scrollback.full() {
		s.sel_move(1, 1 << 30, -1, 0, 0)
	}
	s.scrollback.push(l)
}

// sel_covers says whether the selection touches the lines [a, b].
fn (s &Screen) sel_covers(a int, b int) bool {
	top, bottom := s.sel.line_span()
	return top <= b && bottom >= a
}

fn (mut s Screen) sel_on_erase_line() {
	if s.sel.active {
		c := s.scrollback.len() + s.y
		if s.sel_covers(c, c) {
			s.sel.clear()
		}
	}
}

// sel_on_erase_display applies the rules of ED: 0 from the cursor down, 1 to the cursor from
// the top of the screen, 2 and 3 always.
fn (mut s Screen) sel_on_erase_display(mode int) {
	if !s.sel.active {
		return
	}
	n := s.scrollback.len()
	match mode {
		0 {
			if s.sel_covers(n + s.y, n + s.rows - 1) {
				s.sel.clear()
			}
		}
		1 {
			top, bottom := s.sel.line_span()
			if top <= n + s.y && bottom >= n {
				s.sel.clear()
			}
		}
		else {
			s.sel.clear()
		}
	}
}

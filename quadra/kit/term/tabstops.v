// Tab stops, and the less common screen operations: DECALN, DECCOLM, REP.
module term

// has_stop says whether column x has a tab stop: every 8 by default, until a program sets
// its own (HTS, TBC).
pub fn (s &Screen) has_stop(x int) bool {
	return if x < s.stops.len { s.stops[x] } else { x % 8 == 0 }
}

// shift_charset is SO, SI, LS2 and LS3: the charset the next prints use.
fn (mut s Screen) shift_charset(g int) {
	s.active_cs = g
	s.update_charset()
}

fn (mut s Screen) ensure_stops() {
	for s.stops.len < s.cols {
		s.stops << s.stops.len % 8 == 0
	}
}

// set_tab_stop is HTS (ESC H).
pub fn (mut s Screen) set_tab_stop() {
	s.ensure_stops()
	s.stops[s.x] = true
}

// clear_tab_stops is TBC: 0 the stop at the cursor, 3 every stop.
pub fn (mut s Screen) clear_tab_stops(mode int) {
	s.ensure_stops()
	if mode == 0 {
		s.stops[s.x] = false
	} else if mode == 3 {
		for i in 0 .. s.stops.len {
			s.stops[i] = false
		}
	}
}

// tab is HT and CHT. A tab over a blank cell stores U+0009 there, so copying gives the tab
// back; painting treats it as blank. After the last column it wraps instead.
pub fn (mut s Screen) tab(count int) {
	if s.wrap_pending {
		if s.autowrap {
			s.lines[s.y].wrapped = true
			s.x = 0
			s.index()
		}
		s.wrap_pending = false
		return
	}
	for _ in 0 .. count {
		if s.x >= s.cols {
			break
		}
		if s.lines[s.y].cells[s.x].ch in [u32(0), 32] {
			s.lines[s.y].cells[s.x].ch = 9
			s.touch(s.y)
		}
		for s.x + 1 < s.cols {
			s.x++
			if s.has_stop(s.x) {
				break
			}
		}
	}
}

// back_tab is CBT.
pub fn (mut s Screen) back_tab(count int) {
	for _ in 0 .. count {
		for s.x > 0 {
			s.x--
			if s.has_stop(s.x) {
				break
			}
		}
	}
	s.wrap_pending = false
}

// align_test is DECALN (ESC # 8): the screen full of E.
pub fn (mut s Screen) align_test() {
	for y in 0 .. s.rows {
		for x in 0 .. s.cols {
			s.lines[y].cells[x] = Cell{
				ch: u32(`E`)
			}
		}
		s.lines[y].wrapped = false
		s.touch(y)
	}
}

// column_mode is DECCOLM (mode 3): the width stays; the region resets and the screen clears.
pub fn (mut s Screen) column_mode() {
	s.top, s.bottom = 0, s.rows - 1
	s.erase_display(2)
	s.goto(0, 0)
}

// repeat_last is REP (CSI b).
pub fn (mut s Screen) repeat_last(n int) {
	if s.last_ch != 0 {
		for _ in 0 .. n {
			s.print(s.last_ch)
		}
	}
}

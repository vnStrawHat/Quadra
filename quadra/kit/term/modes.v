// DEC private modes (DECSET, DECRST, DECRQM) and the soft reset.
module term

pub fn (mut s Screen) set_dec_mode(mode int, on bool) {
	match mode {
		1 {
			s.app_cursor = on
		}
		3 {
			s.column_mode()
		}
		5 {
			s.reverse_video = on
		}
		6 {
			s.origin = on
			if on {
				s.goto(0, 0)
			}
		}
		7 {
			s.autowrap = on
		}
		9 {
			s.set_mouse(MouseMode.x10, on)
		}
		12 {
			s.cursor_blink = if on { 1 } else { 2 }
		}
		25 {
			s.cursor_on = on
		}
		45 {
			s.reverse_wrap = on
		}
		47, 1047, 1049 {
			s.set_alt(on) // saves and restores the main cursor
		}
		1000 {
			s.set_mouse(MouseMode.click, on)
		}
		1002 {
			s.set_mouse(MouseMode.drag, on)
		}
		1003 {
			s.set_mouse(MouseMode.any, on)
		}
		1004 {
			s.focus_reports = on
		}
		1005 {
			s.set_encoding(MouseEncoding.utf8, on)
		}
		1006 {
			s.set_encoding(MouseEncoding.sgr, on)
		}
		1007 {
			s.alt_scroll = on
		}
		1015 {
			s.set_encoding(MouseEncoding.urxvt, on)
		}
		1042 {
			s.urgency = on
		}
		1048 {
			if on {
				s.save_cursor()
			} else {
				s.restore_cursor()
			}
		}
		2004 {
			s.bracketed = on
		}
		2026 {
			s.sync = on
			s.sync_hold = on && !s.sync_replay
			s.sync_ended = s.sync_ended || !on
			if s.sync_hold {
				s.sync_opens++
			}
		}
		2027 {
			s.grapheme = on
		}
		else {} // 9001 win32 input: conhost sends it unprompted, nothing reads it
	}

	s.gen++
}

// set_mouse: one protocol at a time; turning one off only clears that one.
fn (mut s Screen) set_mouse(m MouseMode, on bool) {
	if on {
		s.mouse = m
	} else if s.mouse == m {
		s.mouse = .off
	}
}

fn (mut s Screen) set_encoding(e MouseEncoding, on bool) {
	if on {
		s.mouse_enc = e
	} else if s.mouse_enc == e {
		s.mouse_enc = .normal
	}
}

// dec_mode_state answers DECRQM: 0 not recognized, 1 set, 2 reset. A mode nothing reads
// never answers set.
fn (s &Screen) dec_mode_state(mode int) int {
	on := match mode {
		1 { s.app_cursor }
		3 { return 0 }
		5 { s.reverse_video }
		6 { s.origin }
		7 { s.autowrap }
		9 { s.mouse == .x10 }
		12 { s.cursor_blink != 2 }
		25 { s.cursor_on }
		45 { s.reverse_wrap }
		47, 1047, 1049 { s.alt_active }
		1000 { s.mouse == .click }
		1002 { s.mouse == .drag }
		1003 { s.mouse == .any }
		1004 { s.focus_reports }
		1005 { s.mouse_enc == .utf8 }
		1006 { s.mouse_enc == .sgr }
		1007 { s.alt_scroll }
		1015 { s.mouse_enc == .urxvt }
		1042 { s.urgency }
		1048 { false }
		2004 { s.bracketed }
		2026 { s.sync }
		2027 { s.grapheme }
		9001 { false }
		else { return 0 }
	}

	return if on { 1 } else { 2 }
}

// ansi_mode_state answers DECRQM for ANSI modes; LNM (20) is tracked and never set.
fn (s &Screen) ansi_mode_state(mode int) int {
	return match mode {
		4 {
			if s.insert {
				1
			} else {
				2
			}
		}
		20 {
			2
		}
		else {
			0
		}
	}
}

// soft_reset is DECSTR: modes and the pen go back, the screen and history stay.
fn (mut s Screen) soft_reset() {
	s.origin, s.insert = false, false
	s.autowrap, s.cursor_on = true, true
	s.top, s.bottom = 0, s.rows - 1
	s.pen = Cell{
		flags: s.pen.flags & f_zone
	}
	s.goto(0, 0)
	s.saved = s.snapshot()
}

fn (s &Screen) snapshot() Saved {
	return Saved{
		x:        s.x
		y:        s.y
		pen:      s.pen
		charsets: s.charsets
		active:   s.active_cs
		shift:    s.single_shift
		wrap:     s.wrap_pending
	}
}

fn (mut s Screen) apply_saved(v Saved) {
	s.x, s.y, s.pen = clamp(v.x, 0, s.cols - 1), clamp(v.y, 0, s.rows - 1), v.pen
	s.charsets, s.active_cs, s.single_shift = v.charsets, v.active, v.shift
	s.wrap_pending = v.wrap
	s.update_charset()
}

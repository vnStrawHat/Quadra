// CSI dispatch and SGR (colors and text attributes).
module term

fn (p &Parser) arg(i int, def int) int {
	return if i < p.np && p.params[i] != 0 { p.params[i] } else { def }
}

fn (p &Parser) is_sub(i int) bool {
	return i < 32 && p.sub & (u32(1) << i) != 0
}

fn (mut p Parser) csi(mut s Screen, f u8) {
	if p.private == `?` {
		if f == `h` || f == `l` {
			for i in 0 .. p.np {
				s.set_dec_mode(p.params[i], f == `h`)
			}
		}
		return
	}
	if p.private == `>` {
		if f == `c` {
			s.replies << '\x1b[>0;10;1c'.bytes()
		}
		return
	}
	if p.private != 0 {
		return
	}
	n := p.arg(0, 1)
	if p.inter == ` ` && f == `q` { // DECSCUSR
		s.shape = match p.arg(0, 0) {
			3, 4 { CursorShape.underline }
			5, 6 { CursorShape.bar }
			else { CursorShape.block }
		}

		return
	}
	if p.inter != 0 {
		return
	}
	match f {
		`A` { s.move(0, -n) }
		`B`, `e` { s.move(0, n) }
		`C`, `a` { s.move(n, 0) }
		`D` { s.move(-n, 0) }
		`E` { s.move(-s.cols, n) }
		`F` { s.move(-s.cols, -n) }
		`G`, 0x60 { s.move(n - 1 - s.x, 0) }
		`H`, `f` { s.goto(p.arg(1, 1) - 1, p.arg(0, 1) - 1) }
		`d` { s.goto(s.x, n - 1) }
		`J` { s.erase_display(p.arg(0, 0)) }
		`K` { s.erase_line(p.arg(0, 0)) }
		`X` { s.erase_chars(n) }
		`@` { s.insert_chars(n) }
		`P` { s.delete_chars(n) }
		`L` { s.insert_lines(n) }
		`M` { s.delete_lines(n) }
		`S` { s.scroll_up(n) }
		`T` { s.scroll_down(n) }
		`r` { s.set_region(p.arg(0, 0), p.arg(1, 0)) }
		`m` { p.sgr(mut s.pen) }
		`s` { s.save_cursor() }
		`u` { s.restore_cursor() }
		`n` { p.status_report(mut s) }
		`c` { s.replies << '\x1b[?62;22c'.bytes() }
		else {}
	}
}

fn (p &Parser) status_report(mut s Screen) {
	match p.arg(0, 0) {
		5 {
			s.replies << '\x1b[0n'.bytes()
		}
		6 {
			y := if s.origin { s.y - s.top } else { s.y }
			s.replies << '\x1b[${y + 1};${s.x + 1}R'.bytes()
		}
		else {}
	}
}

pub fn (mut s Screen) set_dec_mode(mode int, on bool) {
	match mode {
		1 {
			s.app_cursor = on
		}
		6 {
			s.origin = on
			s.goto(0, 0)
		}
		7 {
			s.autowrap = on
		}
		25 {
			s.cursor_on = on
		}
		47, 1047 {
			s.set_alt(on)
		}
		1049 {
			if on {
				s.save_cursor()
				s.set_alt(true)
			} else {
				s.set_alt(false)
				s.restore_cursor()
			}
		}
		1000 {
			s.mouse = if on { MouseMode.click } else { MouseMode.off }
		}
		1002, 1003 {
			s.mouse = if on { MouseMode.drag } else { MouseMode.off }
		}
		1006 {
			s.mouse_sgr = on
		}
		2004 {
			s.bracketed = on
		}
		else {}
	}

	s.gen++
}

fn (p &Parser) sgr(mut pen Cell) {
	if p.np == 0 {
		pen = Cell{}
		return
	}
	mut i := 0
	for i < p.np {
		v := p.params[i]
		match v {
			0 {
				pen = Cell{}
			}
			1 {
				pen.flags |= f_bold
			}
			2 {
				pen.flags |= f_faint
			}
			3 {
				pen.flags |= f_italic
			}
			4 {
				pen.flags &= ~(f_underline | f_double | f_curly)
				style := if p.is_sub(i + 1) { p.params[i + 1] } else { 1 }
				if p.is_sub(i + 1) {
					i++
				}
				if style == 2 {
					pen.flags |= f_underline | f_double
				} else if style == 3 {
					pen.flags |= f_underline | f_curly
				} else if style != 0 {
					pen.flags |= f_underline
				}
			}
			5, 6 {
				pen.flags |= f_blink
			}
			7 {
				pen.flags |= f_inverse
			}
			8 {
				pen.flags |= f_hidden
			}
			9 {
				pen.flags |= f_strike
			}
			21 {
				pen.flags |= f_underline | f_double
			}
			22 {
				pen.flags &= ~(f_bold | f_faint)
			}
			23 {
				pen.flags &= ~f_italic
			}
			24 {
				pen.flags &= ~(f_underline | f_double | f_curly)
			}
			25 {
				pen.flags &= ~f_blink
			}
			27 {
				pen.flags &= ~f_inverse
			}
			28 {
				pen.flags &= ~f_hidden
			}
			29 {
				pen.flags &= ~f_strike
			}
			30...37 {
				pen.fg = palette(v - 30)
			}
			39 {
				pen.fg = 0
			}
			40...47 {
				pen.bg = palette(v - 40)
			}
			49 {
				pen.bg = 0
			}
			90...97 {
				pen.fg = palette(v - 90 + 8)
			}
			100...107 {
				pen.bg = palette(v - 100 + 8)
			}
			38, 48, 58 {
				c, used := p.ext_color(i)
				if v == 38 {
					pen.fg = c
				} else if v == 48 {
					pen.bg = c
				}
				// 58: underline color, not drawn
				i += used
			}
			else {}
		}

		i++
	}
}

// ext_color reads 5;n or 2;r;g;b after params[i] (also the colon forms, with or without
// a color space id). It returns the color and how many params it used.
fn (p &Parser) ext_color(i int) (u32, int) {
	if i + 1 >= p.np {
		return 0, 0
	}
	match p.params[i + 1] {
		5 {
			if i + 2 < p.np {
				return palette(p.params[i + 2] & 255), 2
			}
		}
		2 {
			mut k := 0 // sub-parameters after the 2
			for i + 2 + k < p.np && p.is_sub(i + 2 + k) {
				k++
			}
			skip := if k >= 4 { 1 } else { 0 } // 38:2:<space>:r:g:b
			j := i + 2 + skip
			if j + 2 < p.np {
				return rgb_color(p.params[j], p.params[j + 1], p.params[j + 2]), 4 + skip
			}
		}
		else {}
	}

	return 0, 1
}

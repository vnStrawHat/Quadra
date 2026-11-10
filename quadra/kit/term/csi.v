// CSI dispatch (SGR is in sgr.v, the DEC modes in modes.v).
module term

fn (p &Parser) arg(i int, def int) int {
	return if i < p.np && p.params[i] != 0 { p.params[i] } else { def }
}

fn (p &Parser) is_sub(i int) bool {
	return i < 32 && p.sub & (u32(1) << i) != 0
}

fn (mut p Parser) csi(mut s Screen, f u8) {
	s.cl_ok = false
	if p.private != 0 || p.inter != 0 {
		p.csi_other(mut s, f)
		return
	}
	n := p.arg(0, 1)
	match f {
		`A` {
			s.move(0, -n)
		}
		`B`, `e` {
			s.move(0, n)
		}
		`C`, `a` {
			s.move(n, 0)
		}
		`D` {
			s.move(-n, 0)
		}
		`E` {
			s.move(-s.cols, n)
		}
		`F` {
			s.move(-s.cols, -n)
		}
		`G`, 0x60 {
			s.move(n - 1 - s.x, 0)
		}
		`H`, `f` {
			s.goto(p.arg(1, 1) - 1, p.arg(0, 1) - 1)
		}
		`d` {
			s.goto(s.x, n - 1)
		}
		`J` {
			s.erase_display(p.arg(0, 0))
		}
		`K` {
			s.erase_line(p.arg(0, 0))
		}
		`X` {
			s.erase_chars(n)
		}
		`@` {
			s.insert_chars(n)
		}
		`P` {
			s.delete_chars(n)
		}
		`L` {
			s.insert_lines(n)
		}
		`M` {
			s.delete_lines(n)
		}
		`S` {
			s.scroll_up(n)
		}
		`T` {
			s.scroll_down(n)
		}
		`r` {
			s.set_region(p.arg(0, 0), p.arg(1, 0))
		}
		`g` {
			s.clear_tab_stops(p.arg(0, 0))
		}
		`I` {
			s.tab(n)
		}
		`Z` {
			s.back_tab(n)
		}
		`b` {
			s.repeat_last(n)
		}
		`h`, `l` {
			p.ansi_modes(mut s, f == `h`)
		}
		`m` {
			p.sgr(mut s.pen)
			if p.ul_op != 0 || p.over_op != 0 {
				s.sgr_extras(p.ul_op, p.ul_val, p.over_op)
			}
		}
		`s` {
			s.save_cursor()
		}
		`u` {
			s.restore_cursor()
		}
		`n` {
			p.status_report(mut s, false)
		}
		`c` {
			s.identify(0)
		}
		`t` {
			p.window_ops(mut s)
		}
		else {}
	}
}

// csi_other handles private prefixes and intermediates.
fn (mut p Parser) csi_other(mut s Screen, f u8) {
	if p.inter == `$` && f == `p` {
		code := p.arg(0, 0)
		if p.private == `?` {
			s.replies << '\x1b[?${code};${s.dec_mode_state(code)}\$y'.bytes()
		} else if p.private == 0 {
			s.replies << '\x1b[${code};${s.ansi_mode_state(code)}\$y'.bytes()
		}
		return
	}
	if p.private == 0 {
		if p.inter == `!` && f == `p` {
			s.soft_reset()
		} else if p.inter == ` ` && f == `q` {
			s.cursor_style(p.arg(0, 0))
		} else if p.inter == `*` && f == `y` {
			s.decrqcra(&p)
		}
		return
	}
	if p.inter != 0 {
		return
	}
	match p.private {
		`?` {
			match f {
				`h`, `l` {
					for i in 0 .. p.np {
						s.set_dec_mode(p.params[i], f == `h`)
					}
				}
				`n` {
					p.status_report(mut s, true)
				}
				`W` {
					if p.arg(0, 0) == 5 {
						s.stops = []
					}
				}
				`u` {
					s.replies << '\x1b[?${s.keyboard.active.live}u'.bytes()
				}
				`m` {
					if p.arg(0, 0) == 4 {
						s.replies << '\x1b[>4;${s.modify_other}m'.bytes()
					}
				}
				else {}
			}
		}
		`>` {
			match f {
				`c` {
					if p.arg(0, 0) == 0 {
						s.identify(`>`)
					}
				}
				`q` {
					if p.arg(0, 0) == 0 {
						s.replies << '\x1bP>|${s.product_name}\x1b\\'.bytes()
					}
				}
				`u` {
					s.keyboard.active.push(u8(p.arg(0, 0)) & kitty_mask)
				}
				`m` {
					if p.arg(0, 1) == 4 && p.arg(1, 0) <= 2 {
						s.modify_other = p.arg(1, 0)
					}
				}
				else {}
			}
		}
		`=` {
			if f == `c` && p.arg(0, 0) == 0 {
				s.identify(`=`)
			} else if f == `u` {
				s.keyboard.active.apply(u8(p.arg(0, 0)) & kitty_mask, p.arg(1, 1))
			}
		}
		`<` {
			if f == `u` {
				s.keyboard.active.pop(p.arg(0, 1))
			}
		}
		else {}
	}
}

// ansi_modes: IRM (4) changes printing, LNM (20) is tracked and read by nothing.
fn (p &Parser) ansi_modes(mut s Screen, on bool) {
	for i in 0 .. p.np {
		if p.params[i] == 4 {
			s.insert = on
		}
	}
}

fn (p &Parser) status_report(mut s Screen, private bool) {
	match p.arg(0, 0) {
		5 {
			if !private {
				s.replies << '\x1b[0n'.bytes()
			}
		}
		6 {
			y := if s.origin { s.y - s.top } else { s.y }
			if private {
				s.replies << '\x1b[?${y + 1};${s.x + 1};1R'.bytes()
			} else {
				s.replies << '\x1b[${y + 1};${s.x + 1}R'.bytes()
			}
		}
		else {}
	}
}

fn (p &Parser) window_ops(mut s Screen) {
	match p.arg(0, 1) {
		14 {
			s.replies << '\x1b[4;${s.rows * s.cell_h};${s.cols * s.cell_w}t'.bytes()
		}
		18 {
			s.replies << '\x1b[8;${s.rows};${s.cols}t'.bytes()
		}
		22 {
			if s.title_stack.len >= 16 {
				s.title_stack.delete(0)
			}
			s.title_stack << s.title
		}
		23 {
			if s.title_stack.len > 0 {
				s.title = s.title_stack.pop()
				s.title_count++
			}
		}
		else {}
	}
}

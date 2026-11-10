// SGR: colors and text attributes.
module term

// sgr applies the parameters to pen. Style_spans has no Screen, so the two extras SGR can
// change (underline color, overline) are left in ul_op and over_op for csi to intern.
fn (mut p Parser) sgr(mut pen Cell) {
	p.ul_op, p.over_op, p.ul_val = 0, 0, 0
	if p.np == 0 {
		p.sgr_reset(mut pen)
		return
	}
	mut i := 0
	for i < p.np {
		v := p.params[i]
		match v {
			// V tests the arms in order: the extended colors, most common, go first
			38, 48, 58 {
				c, used := p.ext_color(i)
				if used >= 2 {
					if v == 38 {
						pen.fg = c
					} else if v == 48 {
						pen.bg = c
					} else {
						p.ul_val, p.ul_op = c, 1
					}
				}
				i += used
			}
			0 {
				p.sgr_reset(mut pen)
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
				pen.flags &= ~underline_styles
				style := if p.is_sub(i + 1) { p.params[i + 1] } else { 1 }
				if p.is_sub(i + 1) {
					i++
				}
				if style == 2 {
					pen.flags |= f_underline | f_double
				} else if style == 3 {
					pen.flags |= f_underline | f_curly
				} else if style == 4 {
					pen.flags |= f_underline | f_dotted
				} else if style == 5 {
					pen.flags |= f_underline | f_dashed
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
				pen.flags &= ~f_bold
			}
			22 {
				pen.flags &= ~(f_bold | f_faint)
			}
			23 {
				pen.flags &= ~f_italic
			}
			24 {
				pen.flags &= ~underline_styles
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
			53 {
				p.over_op = 1
			}
			55 {
				p.over_op = 2
			}
			59 {
				p.ul_op = 2
			}
			90...97 {
				pen.fg = palette(v - 90 + 8)
			}
			100...107 {
				pen.bg = palette(v - 100 + 8)
			}
			else {}
		}

		i++
	}
}

// sgr_reset is SGR 0: the zone and the link are not attributes.
fn (mut p Parser) sgr_reset(mut pen Cell) {
	pen = Cell{
		pad:   pen.pad
		flags: pen.flags & f_zone
	}
	p.ul_op, p.over_op = 2, 2
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

// sgr_extras interns what SGR changed in the pen's extras (underline color, overline).
fn (mut s Screen) sgr_extras(ul_op int, ul u32, over_op int) {
	mut e := s.extra_of(s.pen.pad)
	if ul_op == 1 {
		e = CellExtra{
			...e
			ul: ul
		}
	} else if ul_op == 2 {
		e = CellExtra{
			...e
			ul: 0
		}
	}
	if over_op != 0 {
		e = CellExtra{
			...e
			over: over_op == 1
		}
	}
	s.pen.pad = s.intern_extra(e)
}

// color_params spells a color as SGR parameters; base is 30, 40 or 50 (58 for underline).
fn color_params(c u32, base int) string {
	ext := base + 8 // 38, 48, 58
	if c & true_color != 0 {
		return '${ext};2;${(c >> 16) & 255};${(c >> 8) & 255};${c & 255}'
	}
	i := int(c - 1)
	if base != 50 && i < 16 {
		return if i < 8 { '${base + i}' } else { '${base + 60 + i - 8}' }
	}
	return '${ext};5;${i}'
}

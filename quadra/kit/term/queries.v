// Answers to programs reading terminal state back: DA, DECRQSS, XTGETTCAP, DECRQCRA.
module term

enum DcsQuery {
	none
	decrqss
	xtgettcap
}

// identify answers DA1 (0, also ESC Z), DA2 (`>`) and DA3 (`=`).
fn (mut s Screen) identify(which u8) {
	match which {
		`>` { s.replies << '\x1b[>0;${s.product_version};1c'.bytes() }
		`=` { s.replies << '\x1bP!|00000000\x1b\\'.bytes() }
		else { s.replies << '\x1b[?62;4;22c'.bytes() }
	}
}

// cursor_style is DECSCUSR: odd ids blink, even ones are steady, 0 is the default.
fn (mut s Screen) cursor_style(id int) {
	if id == 0 {
		s.shape, s.cursor_blink = .block, 0
		return
	}
	if id > 6 {
		return
	}
	s.shape = match id {
		3, 4 { CursorShape.underline }
		5, 6 { CursorShape.bar }
		else { CursorShape.block }
	}

	s.cursor_blink = if id % 2 == 1 { 1 } else { 2 }
}

fn (mut p Parser) dcs_entry(mut s Screen, b u8) {
	if b >= `0` && b <= `9` {
		if p.np == 0 {
			p.np = 1
		}
		p.params[p.np - 1] = p.params[p.np - 1] * 10 + int(b - `0`)
	} else if b == `;` {
		if p.np < p.params.len {
			p.np++
		}
	} else if b >= 0x20 && b <= 0x2f {
		p.inter = b
	} else if b >= 0x3c && b <= 0x3f {
		p.private = b
	} else if b >= 0x40 && b <= 0x7e {
		p.dcs_hook(mut s, b)
	}
}

fn (mut p Parser) dcs_hook(mut s Screen, final u8) {
	s.cl_ok = false
	p.dcs_len, p.esc_st = 0, false
	p.query = .none
	if final == `q` && p.inter == 0 && p.private == 0 {
		p.drop_sixel()
		p.sixel = new_sixel()
		p.state = .dcs
	} else if final == `q` && p.inter == `$` {
		p.query = .decrqss
		p.payload.clear()
		p.state = .dcs
	} else if final == `q` && p.inter == `+` {
		p.query = .xtgettcap
		p.payload.clear()
		p.state = .dcs
	} else {
		p.state = .skip_string
	}
}

fn (mut p Parser) dcs_put(b u8) {
	p.dcs_len++
	if p.dcs_len > max_dcs {
		p.drop_sixel()
		p.query = .none
		return
	}
	if p.sixel != unsafe { nil } {
		p.sixel.put(b)
	} else if p.query != .none && p.payload.len < max_query {
		p.payload << b
	}
}

fn (mut p Parser) dcs_end(mut s Screen) {
	if p.sixel != unsafe { nil } {
		p.sixel.flush()
		s.place_sixel(mut p.sixel) // takes the pixels; drop_sixel frees what it left
		p.drop_sixel()
		return
	}
	if p.query == .none || p.payload.len >= max_query {
		return
	}
	req := p.payload.bytestr()
	if p.query == .decrqss {
		s.decrqss(req)
	} else {
		s.xtgettcap(req)
	}
	p.query = .none
}

fn (mut s Screen) decrqss(req string) {
	value := match req {
		'm' {
			s.sgr_parameters()
		}
		'r' {
			'${s.top + 1};${s.bottom + 1}'
		}
		' q' {
			base := match s.shape {
				.underline { 3 }
				.bar { 5 }
				.block { 1 }
			}

			(base + if s.cursor_blink == 2 {
				1
			} else {
				0
			}).str()
		}
		'"q' {
			'0'
		}
		'"p' {
			'62;1'
		}
		else {
			''
		}
	}

	if value == '' {
		s.replies << '\x1bP0\$r\x1b\\'.bytes()
	} else {
		s.replies << '\x1bP1\$r${value}${req}\x1b\\'.bytes()
	}
}

fn (s &Screen) sgr_parameters() string {
	mut out := ['0']
	pen := s.pen
	f := pen.flags
	if f & f_bold != 0 {
		out << '1'
	}
	if f & f_faint != 0 {
		out << '2'
	}
	if f & f_italic != 0 {
		out << '3'
	}
	if f & f_blink != 0 {
		out << '5'
	}
	if f & f_inverse != 0 {
		out << '7'
	}
	if f & f_hidden != 0 {
		out << '8'
	}
	if f & f_strike != 0 {
		out << '9'
	}
	if f & f_underline != 0 {
		out << match true {
			f & f_double != 0 { '4:2' }
			f & f_curly != 0 { '4:3' }
			f & f_dotted != 0 { '4:4' }
			f & f_dashed != 0 { '4:5' }
			else { '4' }
		}
	}
	e := s.extra_of(pen.pad)
	if e.over {
		out << '53'
	}
	if pen.fg != 0 {
		out << color_params(pen.fg, 30)
	}
	if pen.bg != 0 {
		out << color_params(pen.bg, 40)
	}
	if e.ul != 0 {
		out << color_params(e.ul, 50)
	}
	return out.join(';')
}

const xtgettcap_max_names = 16
const xtgettcap_max_name = 128
const capabilities = {
	'TN':      'xterm-256color'
	'Co':      '256'
	'colors':  '256'
	'RGB':     ''
	'Tc':      ''
	'setrgbf': '\x1b[38:2::%p1%d:%p2%d:%p3%dm'
	'setrgbb': '\x1b[48:2::%p1%d:%p2%d:%p3%dm'
	'Su':      ''
	'Ms':      '\x1b]52;%p1%s;%p2%s\x07'
	'Ss':      '\x1b[%p1%d q'
	'Se':      '\x1b[2 q'
	'smcup':   '\x1b[?1049h'
	'rmcup':   '\x1b[?1049l'
	'bel':     '\x07'
	'clear':   '\x1b[H\x1b[2J'
	'cr':      '\r'
	'cub1':    '\x08'
	'cud1':    '\n'
	'cuf1':    '\x1b[C'
	'cuu1':    '\x1b[A'
	'ed':      '\x1b[J'
	'el':      '\x1b[K'
	'home':    '\x1b[H'
}

fn (mut s Screen) xtgettcap(req string) {
	for i, hex in req.split(';') {
		if i >= xtgettcap_max_names || hex.len > xtgettcap_max_name * 2 {
			break
		}
		name := hex_string(hex) or { '' }
		mut value := ?string(none)
		if v := capabilities[name] {
			value = v
			if name == 'TN' && s.product_name != '' {
				value = s.product_name.all_before('(').trim_space()
			}
		}
		if v := value {
			mut out := '\x1bP1+r${hex}='
			out += v.bytes().hex().to_upper()
			s.replies << '${out}\x1b\\'.bytes()
		} else {
			echo := if is_hex(hex) { hex } else { '' }
			s.replies << '\x1bP0+r${echo}\x1b\\'.bytes()
		}
	}
}

fn hex_string(h string) ?string {
	if h == '' || h.len % 2 != 0 || !is_hex(h) {
		return none
	}
	mut out := []u8{cap: h.len / 2}
	for i := 0; i < h.len; i += 2 {
		out << u8(hex_val(h[i]) * 16 + hex_val(h[i + 1]))
	}
	return out.bytestr()
}

// decrqcra is CSI ... * y, the checksum of a rectangle (off unless allow_screen_readback).
fn (mut s Screen) decrqcra(p &Parser) {
	id := p.arg(0, 0)
	lo, hi := if s.origin { s.top, s.bottom + 1 } else { 0, s.rows }
	top := clamp(p.arg(2, 1) - 1 + lo, lo, hi)
	left := clamp(p.arg(3, 1) - 1, 0, s.cols)
	bottom := clamp(p.arg(4, s.rows) + lo, lo, hi)
	right := clamp(p.arg(5, s.cols), 0, s.cols)
	if !s.allow_screen_readback {
		return
	}
	mut sum := u32(0)
	y_end := if bottom > top { bottom } else { top }
	x_end := if right > left { right } else { left }
	for y in top .. y_end {
		row := s.lines[y]
		for x in left .. x_end {
			sum += if row.cells[x].ch == 0 { u32(32) } else { row.cells[x].ch }
			for m in row.marks {
				if m.col == x {
					for r in m.s.runes() {
						sum += u32(r)
					}
				}
			}
		}
	}
	s.replies << '\x1bP${id}!~${sum & 0xffff:04X}\x1b\\'.bytes()
}

// Parser: a VT500-style state machine over bytes that drives a Screen (specs/17). One byte
// loop, no allocation per byte; runs of printable ASCII take a fast path.
module term

import encoding.base64

enum State {
	ground
	escape
	esc_inter
	csi
	osc
	skip_string // DCS, SOS, PM, APC: consumed until ST
}

pub struct Parser {
mut:
	state   State
	params  [16]int
	sub     u32 // bit i: param i followed a ':' (sub-parameter)
	np      int // params seen (0 = none)
	private u8  // '?', '>', '<' or '='
	inter   u8  // intermediate byte
	ignore  bool
	osc     []u8
	esc_st  bool // ESC seen inside a string: ST ends it
	acc     u32  // UTF-8 decoding
	need    int
}

pub const max_osc = 1 << 20

// feed parses data into s.
pub fn (mut p Parser) feed(mut s Screen, data []u8) {
	mut i := 0
	for i < data.len {
		b := data[i]
		if p.state == .ground && p.need == 0 && b >= 0x20 && b < 0x7f {
			mut j := i + 1
			for j < data.len && data[j] >= 0x20 && data[j] < 0x7f {
				j++
			}
			s.print_ascii(data[i..j])
			i = j
			continue
		}
		p.step(mut s, b)
		i++
	}
}

fn (mut p Parser) step(mut s Screen, b u8) {
	if p.state == .osc || p.state == .skip_string {
		p.string_byte(mut s, b)
		return
	}
	if b == 0x1b {
		p.need = 0
		p.state, p.inter = .escape, 0
		return
	}
	if b == 0x18 || b == 0x1a {
		p.state = .ground
		return
	}
	if b < 0x20 {
		p.execute(mut s, b)
		return
	}
	match p.state {
		.ground { p.utf8(mut s, b) }
		.escape, .esc_inter { p.escape(mut s, b) }
		.csi { p.csi_byte(mut s, b) }
		else {}
	}
}

fn (mut p Parser) execute(mut s Screen, b u8) {
	match b {
		0x08 { s.backspace() }
		0x09 { s.tab() }
		0x0a, 0x0b, 0x0c { s.linefeed() }
		0x0d { s.carriage_return() }
		else {} // BEL, SO, SI and the rest: nothing to draw
	}
}

fn (mut p Parser) utf8(mut s Screen, b u8) {
	if p.need > 0 {
		if b & 0xc0 == 0x80 {
			p.acc = (p.acc << 6) | u32(b & 0x3f)
			p.need--
			if p.need == 0 {
				s.print(p.acc)
			}
			return
		}
		p.need = 0
		s.print(0xfffd)
		p.step(mut s, b)
		return
	}
	if b < 0x80 {
		if b != 0x7f {
			s.print(u32(b))
		}
	} else if b & 0xe0 == 0xc0 {
		p.acc, p.need = u32(b & 0x1f), 1
	} else if b & 0xf0 == 0xe0 {
		p.acc, p.need = u32(b & 0x0f), 2
	} else if b & 0xf8 == 0xf0 {
		p.acc, p.need = u32(b & 0x07), 3
	} else {
		s.print(0xfffd)
	}
}

fn (mut p Parser) escape(mut s Screen, b u8) {
	if b >= 0x20 && b <= 0x2f {
		p.inter = b
		p.state = .esc_inter
		return
	}
	p.state = .ground
	if p.inter == `(` {
		s.line_drawing = b == `0`
		return
	}
	if p.inter != 0 {
		return
	}
	match b {
		`[` {
			p.state, p.np, p.sub, p.private, p.inter, p.ignore = .csi, 0, 0, 0, 0, false
			p.params = [16]int{}
		}
		`]` {
			p.state, p.esc_st = .osc, false
			p.osc.clear()
		}
		`P`, `X`, `^`, `_` {
			p.state, p.esc_st = .skip_string, false
		}
		`7` {
			s.save_cursor()
		}
		`8` {
			s.restore_cursor()
		}
		`D` {
			s.linefeed()
		}
		`E` {
			s.carriage_return()
			s.linefeed()
		}
		`M` {
			s.reverse_index()
		}
		`c` {
			s.reset()
		}
		`=` {
			s.app_keypad = true
		}
		`>` {
			s.app_keypad = false
		}
		else {}
	}
}

fn (mut p Parser) csi_byte(mut s Screen, b u8) {
	if b >= `0` && b <= `9` {
		if p.np == 0 {
			p.np = 1
		}
		v := p.params[p.np - 1] * 10 + int(b - `0`)
		p.params[p.np - 1] = if v > 65535 { 65535 } else { v }
	} else if b == `;` || b == `:` {
		if p.np == 0 {
			p.np = 1
		}
		if p.np >= 16 {
			p.ignore = true
			return
		}
		p.np++
		if b == `:` {
			p.sub |= u32(1) << (p.np - 1)
		}
	} else if b >= 0x3c && b <= 0x3f {
		p.private = b
	} else if b >= 0x20 && b <= 0x2f {
		p.inter = b
	} else if b >= 0x40 && b <= 0x7e {
		p.state = .ground
		if !p.ignore {
			p.csi(mut s, b)
		}
	}
}

fn (mut p Parser) string_byte(mut s Screen, b u8) {
	if p.esc_st {
		p.end_string(mut s)
		if b != `\\` {
			p.state, p.inter = .escape, 0
			p.step(mut s, b)
		}
		return
	}
	if b == 0x1b {
		p.esc_st = true
	} else if b == 0x07 {
		p.end_string(mut s)
	} else if p.state == .osc && p.osc.len < max_osc {
		p.osc << b
	}
}

fn (mut p Parser) end_string(mut s Screen) {
	if p.state == .osc {
		p.osc_dispatch(mut s)
	}
	p.state, p.esc_st = .ground, false
}

// osc_dispatch: 0 and 2 set the title, 52 writes the clipboard when allowed; 8 (links) and
// the rest are consumed.
fn (mut p Parser) osc_dispatch(mut s Screen) {
	body := p.osc.bytestr()
	semi := body.index(';') or { return }
	code := body[..semi]
	arg := body[semi + 1..]
	match code {
		'0', '2' {
			s.title = arg
		}
		'52' {
			data := arg.all_after(';')
			if s.allow_osc52 && data != '?' {
				s.clipboard = base64.decode_str(data)
			}
		}
		else {}
	}
}

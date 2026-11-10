// Parser: a VT500-style state machine over bytes that drives a Screen (specs/17). One byte
// loop, no allocation per byte; runs of printable ASCII take a fast path.
module term

enum State {
	ground
	escape
	esc_inter
	csi
	osc
	dcs_entry   // DCS parameters and final byte
	dcs         // DCS payload, for a sink (Sixel, DECRQSS, XTGETTCAP)
	skip_string // SOS, PM, APC and unknown DCS: consumed until ST
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
	bel     bool // the string that just ended used BEL (a reply uses the same)
	ul_op   int  // what SGR did to the underline color: 1 set, 2 reset
	ul_val  u32
	over_op int // and to overline: 1 on, 2 off
	sixel   &Sixel = unsafe { nil }
	query   DcsQuery
	payload []u8 // a DCS query's body
	dcs_len int
	held    []u8 // bytes after a synchronized update opened (mode 2026)
	held_on bool
	acc     u32 // UTF-8 decoding
	need    int
}

pub const max_osc = 1 << 20
pub const max_osc52 = 256 * 1024
pub const max_dcs = 16 << 20
const max_query = 8 * 1024

// feed parses data into s.
pub fn (mut p Parser) feed(mut s Screen, data []u8) {
	if p.held_on {
		p.held << data
		p.release_held(mut s)
		return
	}
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
		if p.state == .ground && p.need == 0 {
			mut next := -1
			if b >= 0xc0 {
				next = p.fast_utf8(mut s, data, i)
			} else if b == 0x1b && i + 1 < data.len && data[i + 1] == `[` {
				next = p.fast_csi(mut s, data, i)
			}
			if next >= 0 {
				i = next
				if s.sync {
					p.held_on = true
					p.feed(mut s, data[i..])
					return
				}
				continue
			}
		}
		p.step(mut s, b)
		i++
		if s.sync && p.state == .ground {
			p.held_on = true
			p.feed(mut s, data[i..])
			return
		}
	}
	if s.graphics.len > 0 {
		s.sweep_count++
		if s.sweep_count & 63 == 0 {
			s.sweep_graphics()
		}
	}
}

// release_held parses the held bytes when the end of the update has arrived.
fn (mut p Parser) release_held(mut s Screen) {
	if p.held.bytestr().contains('[?2026l') {
		buf := p.held
		p.held = []u8{}
		p.held_on = false
		p.feed(mut s, buf)
	}
}

fn (mut p Parser) step(mut s Screen, b u8) {
	if p.state == .osc || p.state == .skip_string || p.state == .dcs {
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
	if p.state == .dcs_entry {
		p.dcs_entry(mut s, b)
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
		0x09 { s.tab(1) }
		0x0a, 0x0b, 0x0c { s.linefeed() }
		0x0d { s.carriage_return() }
		0x07 { s.bell = true }
		0x0e { s.shift_charset(1) }
		0x0f { s.shift_charset(0) }
		else {} // the rest: nothing to draw
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
	s.cl_ok = false
	if p.inter >= `(` && p.inter <= `+` {
		s.designate(int(p.inter - `(`), b)
		return
	}
	if p.inter == `#` && b == `8` {
		s.align_test()
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
		`P` {
			p.state, p.esc_st, p.np, p.sub, p.private, p.inter = .dcs_entry, false, 0, 0, 0, 0
			p.params = [16]int{}
			p.ignore = false
		}
		`X`, `^`, `_` {
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
		`H` {
			s.set_tab_stop()
		}
		`Z` {
			s.identify(0)
		}
		`n` {
			s.shift_charset(2)
		}
		`o` {
			s.shift_charset(3)
		}
		`N` {
			s.single_shift = 2
			s.update_charset()
		}
		`O` {
			s.single_shift = 3
			s.update_charset()
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

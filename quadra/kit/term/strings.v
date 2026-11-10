// String sequences (OSC, DCS, APC, SOS, PM): payload bytes until ST, BEL or an abort.
module term

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
	} else if b == 0x18 || b == 0x1a {
		p.abort_string()
	} else if b == 0x07 && p.state != .dcs {
		p.bel = true
		p.end_string(mut s)
	} else if p.state == .osc {
		if p.osc.len < max_osc {
			p.osc << b
		}
	} else if p.state == .dcs {
		p.dcs_put(b)
	}
}

fn (mut p Parser) end_string(mut s Screen) {
	s.cl_ok = false
	if p.state == .osc {
		p.osc_dispatch(mut s)
	} else if p.state == .dcs {
		p.dcs_end(mut s)
	}
	p.state, p.esc_st, p.bel = .ground, false, false
}

// abort_string is CAN or SUB inside a string: the partial payload is discarded.
fn (mut p Parser) abort_string() {
	p.sixel = unsafe { nil }
	p.query = .none
	p.state, p.esc_st = .ground, false
}

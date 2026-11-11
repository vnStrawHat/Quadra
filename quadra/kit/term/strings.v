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
		p.osc_put(b)
	} else if p.state == .dcs {
		p.dcs_put(b)
	}
}

// osc_start readies the payload; the buffer of a large one is not kept for the session.
fn (mut p Parser) osc_start() {
	if p.osc.cap > osc_inline {
		p.osc = []u8{cap: osc_inline}
	} else {
		p.osc.clear()
	}
	p.osc_big, p.osc_sep, p.osc_cut = false, 0, false
}

// osc_put adds a payload byte: the cap is 2048 bytes, or 8 MiB once the first `;` shows a
// number that needs it. Past the cap bytes are lost and the string still dispatches. The
// separators of the first 16 parameters are structure: they do not count and are never lost.
fn (mut p Parser) osc_put(b u8) {
	if b == `;` && p.osc_sep < max_osc_params - 1 {
		if p.osc_sep == 0 {
			code := p.osc.bytestr()
			p.osc_big = code == '52' || code == '7501'
		}
		p.osc_sep++
		p.osc << b
		return
	}
	if p.osc.len - p.osc_sep >= if p.osc_big {
		osc_large
	} else {
		osc_inline
	} {
		p.osc_cut = true
		return
	}
	p.osc << b
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
	p.drop_sixel()
	p.query = .none
	p.state, p.esc_st = .ground, false
}

// drop_sixel ends a Sixel decode and frees its pixels.
fn (mut p Parser) drop_sixel() {
	if p.sixel != unsafe { nil } {
		p.sixel.free()
		p.sixel = unsafe { nil }
	}
}

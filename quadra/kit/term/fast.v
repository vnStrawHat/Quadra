// Fast paths of the ground state (specs/17, Parser): a CSI sequence or a UTF-8 sequence that
// lies whole in the chunk is handled in one call instead of a state-machine step per byte.
// Each leaves the parser exactly as the byte loop would; anything unusual (a control inside a
// CSI, a split sequence) falls back to that loop.
module term

// fast_csi parses `ESC [ ... final` at data[at] when it is complete and holds only parameter,
// intermediate and final bytes. It returns the index after the sequence, or -1.
fn (mut p Parser) fast_csi(mut s Screen, data []u8, at int) int {
	mut j := at + 2
	for j < data.len && data[j] >= 0x20 && data[j] <= 0x3f {
		j++
	}
	if j >= data.len || data[j] < 0x40 || data[j] > 0x7e {
		return -1
	}
	p.need = 0
	p.np, p.sub, p.private, p.inter, p.ignore = 0, 0, 0, 0, false
	p.params = [16]int{}
	mut v := 0 // the parameter being read
	for k in at + 2 .. j {
		b := data[k]
		if b >= `0` && b <= `9` {
			if p.np == 0 {
				p.np = 1
			}
			v = v * 10 + int(b - `0`)
			if v > 65535 {
				v = 65535
			}
			p.params[p.np - 1] = v
		} else if b == `;` || b == `:` {
			if p.np == 0 {
				p.np = 1
			}
			if p.np >= 16 {
				p.ignore = true
				continue
			}
			p.np++
			v = 0
			if b == `:` {
				p.sub |= u32(1) << (p.np - 1)
			}
		} else if b >= 0x3c {
			p.private = b
		} else {
			p.inter = b // 0x20..0x2f
		}
	}
	p.state = .ground
	s.cl_ok = false // as escape() does on the `[`
	if p.ignore {
		return j + 1
	}
	if data[j] == `m` && p.private == 0 && p.inter == 0 { // SGR, ahead of csi's long match
		p.sgr(mut s.pen)
		if p.ul_op != 0 || p.over_op != 0 {
			s.sgr_extras(p.ul_op, p.ul_val, p.over_op)
		}
	} else {
		p.csi(mut s, data[j])
	}
	return j + 1
}

// fast_utf8 prints the multi-byte sequence starting at data[at] when it is complete and
// well formed. It returns the index after it, or -1.
@[direct_array_access]
fn (mut p Parser) fast_utf8(mut s Screen, data []u8, at int) int {
	b := data[at]
	mut r := u32(0)
	mut n := 0
	if b & 0xe0 == 0xc0 {
		r, n = u32(b & 0x1f), 2
	} else if b & 0xf0 == 0xe0 {
		r, n = u32(b & 0x0f), 3
	} else if b & 0xf8 == 0xf0 {
		r, n = u32(b & 0x07), 4
	} else {
		return -1
	}
	if at + n > data.len {
		return -1
	}
	for k in at + 1 .. at + n {
		c := data[k]
		if c & 0xc0 != 0x80 {
			return -1
		}
		r = (r << 6) | u32(c & 0x3f)
	}
	s.print(r)
	return at + n
}

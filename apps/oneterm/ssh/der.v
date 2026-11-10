// A small DER reader for the PKCS#1, SEC1 and PKCS#8 key formats.
module ssh

import quadra.native.cng

struct Der {
	buf []u8
mut:
	pos int
}

fn (mut d Der) has_more() bool {
	return d.pos < d.buf.len
}

// next returns the tag and contents of the next element.
fn (mut d Der) next() !(u8, []u8) {
	if d.pos + 2 > d.buf.len {
		return error('truncated DER')
	}
	tag := d.buf[d.pos]
	mut n := int(d.buf[d.pos + 1])
	d.pos += 2
	if n & 0x80 != 0 {
		count := n & 0x7f
		if count == 0 || count > 4 || d.pos + count > d.buf.len {
			return error('bad DER length')
		}
		n = 0
		for _ in 0 .. count {
			n = int(u32(n) << 8) | int(d.buf[d.pos])
			d.pos++
		}
	}
	if n < 0 || d.pos + n > d.buf.len {
		return error('truncated DER')
	}
	body := d.buf[d.pos..d.pos + n]
	d.pos += n
	return tag, body
}

fn (mut d Der) expect(tag u8) ![]u8 {
	got, body := d.next()!
	if got != tag {
		return error('unexpected DER tag ${got}')
	}
	return body
}

fn (mut d Der) seq() !Der {
	return Der{
		buf: d.expect(0x30)!
	}
}

fn (mut d Der) int_bytes() ![]u8 {
	return cng.strip_zeros(d.expect(0x02)!)
}

fn (mut d Der) int_val() !int {
	mut v := 0
	for b in d.int_bytes()! {
		v = int(u32(v) << 8) | int(b)
	}
	return v
}

fn (mut d Der) oid() ![]u8 {
	return d.expect(0x06)!
}

fn (mut d Der) octets() ![]u8 {
	return d.expect(0x04)!
}

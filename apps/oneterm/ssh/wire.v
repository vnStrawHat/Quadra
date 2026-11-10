// SSH wire encoding (RFC 4251 section 5).
module ssh

import encoding.binary

struct Writer {
mut:
	buf []u8
}

fn new_writer(cap int) Writer {
	return Writer{
		buf: []u8{cap: cap}
	}
}

fn (mut w Writer) put_u8(v u8) {
	w.buf << v
}

fn (mut w Writer) put_bool(v bool) {
	if v {
		w.buf << u8(1)
	} else {
		w.buf << u8(0)
	}
}

fn (mut w Writer) put_u32(v u32) {
	w.buf << u8(v >> 24)
	w.buf << u8(v >> 16)
	w.buf << u8(v >> 8)
	w.buf << u8(v)
}

fn (mut w Writer) put_u64(v u64) {
	w.put_u32(u32(v >> 32))
	w.put_u32(u32(v))
}

fn (mut w Writer) put_raw(b []u8) {
	w.buf << b
}

fn (mut w Writer) put_bytes(b []u8) {
	w.put_u32(u32(b.len))
	w.buf << b
}

fn (mut w Writer) put_str(s string) {
	w.put_bytes(s.bytes())
}

fn (mut w Writer) put_names(names []string) {
	w.put_str(names.join(','))
}

// put_mpint writes unsigned big-endian bytes as an mpint: no leading zeros, one 0x00 if the
// high bit is set.
fn (mut w Writer) put_mpint(b []u8) {
	mut start := 0
	for start < b.len && b[start] == 0 {
		start++
	}
	if start == b.len {
		w.put_u32(0)
		return
	}
	mut n := b.len - start
	if b[start] & 0x80 != 0 {
		n++
		w.put_u32(u32(n))
		w.buf << u8(0)
	} else {
		w.put_u32(u32(n))
	}
	w.buf << b[start..]
}

struct Reader {
	buf []u8
mut:
	pos int
}

fn new_reader(buf []u8) Reader {
	return Reader{
		buf: buf
	}
}

fn (r Reader) left() int {
	return r.buf.len - r.pos
}

fn (mut r Reader) need(n int) ! {
	// n > left, not pos + n > len: a length near the int maximum wrapped the sum negative
	if n < 0 || n > r.buf.len - r.pos {
		return error('truncated packet')
	}
}

fn (mut r Reader) get_u8() !u8 {
	r.need(1)!
	v := r.buf[r.pos]
	r.pos++
	return v
}

fn (mut r Reader) get_bool() !bool {
	return r.get_u8()! != 0
}

fn (mut r Reader) get_u32() !u32 {
	r.need(4)!
	v := binary.big_endian_u32(r.buf[r.pos..r.pos + 4])
	r.pos += 4
	return v
}

fn (mut r Reader) get_u64() !u64 {
	r.need(8)!
	v := binary.big_endian_u64(r.buf[r.pos..r.pos + 8])
	r.pos += 8
	return v
}

// get_raw returns a slice of the buffer, not a copy.
fn (mut r Reader) get_raw(n int) ![]u8 {
	r.need(n)!
	v := r.buf[r.pos..r.pos + n]
	r.pos += n
	return v
}

fn (mut r Reader) get_bytes() ![]u8 {
	n := int(r.get_u32()!)
	return r.get_raw(n)
}

fn (mut r Reader) get_str() !string {
	return r.get_bytes()!.bytestr()
}

fn (mut r Reader) get_names() ![]string {
	s := r.get_str()!
	if s == '' {
		return []
	}
	return s.split(',')
}

// get_mpint returns the unsigned big-endian magnitude.
fn (mut r Reader) get_mpint() ![]u8 {
	b := r.get_bytes()!
	if b.len > 0 && b[0] & 0x80 != 0 {
		return error('negative mpint')
	}
	mut start := 0
	for start < b.len && b[start] == 0 {
		start++
	}
	return b[start..]
}

// Buffer: a piece table over the original text and an append-only add buffer, with the
// byte offset of every line start kept up to date (specs/18).
module code

struct Piece {
	add   bool // in the add buffer (else the original text)
	start int
	len   int
}

@[heap]
pub struct Buffer {
mut:
	orig   string
	add    []u8
	pieces []Piece
	offs   []int // document offset where each piece starts
	starts []int // byte offset of every line start; starts[0] == 0
	total  int
pub mut:
	crlf bool // the file used CRLF: text is LF inside, written back with CRLF
}

pub fn new_buffer(s string) &Buffer {
	crlf := s.contains('\r\n')
	text := if crlf { s.replace('\r\n', '\n') } else { s }
	mut b := &Buffer{
		orig:  text
		crlf:  crlf
		total: text.len
	}
	if text.len > 0 {
		b.pieces << Piece{false, 0, text.len}
	}
	b.rebuild_offs()
	b.starts = [0]
	b.starts << newline_starts(text.str, text.len, 0)
	return b
}

// newline_starts returns off + i + 1 for every '\n' at i in the n bytes at p.
fn newline_starts(p &u8, n int, off int) []int {
	mut out := []int{}
	for i in 0 .. n {
		if unsafe { p[i] } == `\n` {
			out << off + i + 1
		}
	}
	return out
}

fn (mut b Buffer) rebuild_offs() {
	b.offs = []int{len: b.pieces.len}
	mut o := 0
	for i, p in b.pieces {
		b.offs[i] = o
		o += p.len
	}
}

fn (b &Buffer) ptr(p Piece) &u8 {
	return if p.add { unsafe { &u8(b.add.data) + p.start } } else { unsafe { b.orig.str + p.start } }
}

// piece_at returns the index of the piece holding offset off (pieces.len at the end).
fn (b &Buffer) piece_at(off int) int {
	mut lo, mut hi := 0, b.pieces.len - 1
	for lo <= hi {
		m := (lo + hi) / 2
		if off < b.offs[m] {
			hi = m - 1
		} else if off >= b.offs[m] + b.pieces[m].len {
			lo = m + 1
		} else {
			return m
		}
	}
	return b.pieces.len
}

// split_at makes a piece boundary at off and returns the index of the piece starting there.
fn (mut b Buffer) split_at(off int) int {
	i := b.piece_at(off)
	if i == b.pieces.len || b.offs[i] == off {
		return i
	}
	p := b.pieces[i]
	k := off - b.offs[i]
	b.pieces[i] = Piece{p.add, p.start, k}
	b.pieces.insert(i + 1, Piece{p.add, p.start + k, p.len - k})
	b.rebuild_offs()
	return i + 1
}

pub fn (b &Buffer) len() int {
	return b.total
}

pub fn (b &Buffer) line_count() int {
	return b.starts.len
}

pub fn (b &Buffer) line_start(n int) int {
	return b.starts[n]
}

// line_end is the offset of line n's newline (or the end of the text).
pub fn (b &Buffer) line_end(n int) int {
	return if n + 1 < b.starts.len { b.starts[n + 1] - 1 } else { b.total }
}

pub fn (b &Buffer) line(n int) string {
	return b.slice(b.starts[n], b.line_end(n))
}

// line_of returns the line holding offset off.
pub fn (b &Buffer) line_of(off int) int {
	mut lo, mut hi := 0, b.starts.len - 1
	for lo < hi {
		m := (lo + hi + 1) / 2
		if b.starts[m] <= off {
			lo = m
		} else {
			hi = m - 1
		}
	}
	return lo
}

// slice copies the text between offsets a and b.
pub fn (b &Buffer) slice(a int, e int) string {
	if e <= a {
		return ''
	}
	mut out := []u8{cap: e - a + 1}
	mut i := b.piece_at(a)
	mut off := a
	for off < e && i < b.pieces.len {
		p := b.pieces[i]
		from := off - b.offs[i]
		n := if b.offs[i] + p.len < e { p.len - from } else { e - b.offs[i] - from }
		unsafe { out.push_many(b.ptr(p) + from, n) }
		off += n
		i++
	}
	out << 0
	return unsafe { tos(&u8(out.data), out.len - 1) }
}

pub fn (b &Buffer) text() string {
	return b.slice(0, b.total)
}

// byte_at returns the byte at off (0 past the end).
pub fn (b &Buffer) byte_at(off int) u8 {
	i := b.piece_at(off)
	if i == b.pieces.len {
		return 0
	}
	return unsafe { b.ptr(b.pieces[i])[off - b.offs[i]] }
}

pub fn (mut b Buffer) insert(off int, s string) {
	if s.len == 0 {
		return
	}
	from := b.add.len
	unsafe { b.add.push_many(s.str, s.len) }
	i := b.split_at(off)
	if i > 0 && b.pieces[i - 1].add && b.pieces[i - 1].start + b.pieces[i - 1].len == from {
		p := b.pieces[i - 1] // typing at the end of the last insert: grow that piece
		b.pieces[i - 1] = Piece{true, p.start, p.len + s.len}
	} else {
		b.pieces.insert(i, Piece{true, from, s.len})
	}
	b.rebuild_offs()
	b.total += s.len
	line := b.line_of(off)
	for k in line + 1 .. b.starts.len {
		b.starts[k] += s.len
	}
	added := newline_starts(s.str, s.len, off)
	if added.len > 0 {
		b.starts.insert(line + 1, added)
	}
}

pub fn (mut b Buffer) delete(a int, e int) {
	if e <= a {
		return
	}
	i := b.split_at(a)
	j := b.split_at(e)
	b.pieces.delete_many(i, j - i)
	b.rebuild_offs()
	b.total -= e - a
	first := b.line_of(a) + 1
	mut last := first // starts in (a, e] are removed
	for last < b.starts.len && b.starts[last] <= e {
		last++
	}
	if last > first {
		b.starts.delete_many(first, last - first)
	}
	for k in first .. b.starts.len {
		b.starts[k] -= e - a
	}
}

// file_text is the text as written back to disk (CRLF restored).
pub fn (b &Buffer) file_text() string {
	t := b.text()
	return if b.crlf { t.replace('\n', '\r\n') } else { t }
}

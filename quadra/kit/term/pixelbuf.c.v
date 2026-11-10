// PixelBuf: image pixels in the C heap (specs/17, Images memory). On the conservative
// collector a block of up to 64 MiB stays alive while any stale word points into it, and
// such blocks fragment the heap; this one is freed when its owner says so.
module term

// PixelBuf is h rows of w RGBA pixels (w is the row stride) in a C block of cap bytes. The
// bytes past len are always zero, so growth makes transparent pixels.
//
// Ownership: one owner at a time, and that owner frees it. Never copy a PixelBuf, or a
// struct holding one, by value: move it with `take` (the old place is left empty). The
// decoder (Sixel.pixels) owns it while decoding, flush moves it to Sixel.rgba, place_sixel to
// the Graphic, and the view to the atlas upload (`Atlas.insert_owned`), which the window
// frees once the device has copied it (`gpu.free_owned`). `bytes` is a view for reading,
// valid until the next free, take or growth. Freeing twice is a no-op.
struct PixelBuf {
mut:
	data &u8 = unsafe { nil }
	len  int // bytes in use: w * h * 4
	cap  int // bytes allocated
	w    int // pixels a row
	h    int // rows
}

// free gives the block back to the C heap and leaves the buffer empty.
fn (mut b PixelBuf) free() {
	if b.data != unsafe { nil } {
		unsafe { C.free(b.data) }
	}
	b.data, b.len, b.cap, b.w, b.h = unsafe { nil }, 0, 0, 0, 0
}

// take moves the block out: the result owns it, b is left empty (not freed).
fn (mut b PixelBuf) take() PixelBuf {
	out := PixelBuf{b.data, b.len, b.cap, b.w, b.h}
	b.data, b.len, b.cap, b.w, b.h = unsafe { nil }, 0, 0, 0, 0
	return out
}

// bytes is the pixels in use, lent (see the ownership rule above).
fn (b &PixelBuf) bytes() []u8 {
	if b.len == 0 {
		return []u8{}
	}
	return unsafe { b.data.vbytes(b.len) }
}

// reserve grows the block to at least n bytes (realloc), zeroing what is new.
fn (mut b PixelBuf) reserve(n int) {
	if n <= b.cap {
		return
	}
	p := unsafe { &u8(C.realloc(b.data, usize(n))) }
	if p == unsafe { nil } {
		panic('term: out of memory for ${n} bytes of image pixels')
	}
	unsafe { C.memset(p + b.cap, 0, usize(n - b.cap)) }
	b.data, b.cap = p, n
}

// shrink gives back the bytes past len.
fn (mut b PixelBuf) shrink() {
	if b.len == 0 {
		b.free()
		return
	}
	if b.cap > b.len {
		p := unsafe { &u8(C.realloc(b.data, usize(b.len))) }
		if p != unsafe { nil } {
			b.data, b.cap = p, b.len
		}
	}
}

// restride lays the pixels out nw a row with `rows` rows, in place: rows past the old ones
// are transparent, columns and rows past the new size are cut.
fn (mut b PixelBuf) restride(nw int, rows int) {
	ow, old := b.w, b.len
	keep := if b.h < rows { b.h } else { rows }
	need := rows * nw * 4
	b.reserve(need)
	if keep > 0 && nw > ow {
		// Wider rows move down: the last row first, so none is overwritten before it moves.
		for y := keep - 1; y >= 0; y-- {
			unsafe {
				dst := b.data + y * nw * 4
				C.memmove(dst, b.data + y * ow * 4, usize(ow * 4))
				C.memset(dst + ow * 4, 0, usize((nw - ow) * 4))
			}
		}
	} else if keep > 0 && nw < ow {
		for y in 1 .. keep { // narrower rows move up: the first row first
			unsafe { C.memmove(b.data + y * nw * 4, b.data + y * ow * 4, usize(nw * 4)) }
		}
	}
	end := keep * nw * 4
	if old > end { // what the old layout left past the kept rows
		unsafe { C.memset(b.data + end, 0, usize(old - end)) }
	}
	b.w, b.h, b.len = nw, rows, need
}

// fill paints n pixels from pixel index i with c (RGBA, red in the low byte). Input picks
// both, so the span is checked.
fn (mut b PixelBuf) fill(i int, n int, c u32) {
	if i < 0 || n <= 0 || i + n > b.len / 4 {
		return
	}
	px := unsafe { &u32(b.data) }
	for k in i .. i + n {
		unsafe {
			px[k] = c
		}
	}
}

// scaled is a new w x h buffer, nearest neighbour from b. The caller owns both.
fn (b &PixelBuf) scaled(w int, h int) PixelBuf {
	mut out := PixelBuf{}
	if w <= 0 || h <= 0 || b.len == 0 {
		return out
	}
	out.reserve(w * h * 4)
	out.w, out.h, out.len = w, h, w * h * 4
	src := unsafe { &u32(b.data) }
	dst := unsafe { &u32(out.data) }
	for y in 0 .. h {
		row := (y * b.h / h) * b.w
		for x in 0 .. w {
			unsafe {
				dst[y * w + x] = src[row + x * b.w / w]
			}
		}
	}
	return out
}

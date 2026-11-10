// Single-line layout and caret mapping.
module text

import math

pub struct PositionedGlyph {
pub:
	face    FaceId
	id      u16
	x       f32 // from the line start
	y       f32 // vertical offset from the baseline (positive = up)
	cluster int // UTF-8 byte offset in the line text
}

pub struct CaretStop {
pub:
	index int // grapheme boundary, UTF-8 byte offset
	x     f32
}

// LineLayout is a laid-out line. A line cut from a longer one (cut) shares its glyphs and
// stops: their x is measured from `origin` and their byte offsets from `base`. Read-only
// outside this module; wrap_number refills a layout a text element owns.
pub struct LineLayout {
pub mut:
	text    string
	glyphs  []PositionedGlyph
	stops   []CaretStop
	width   f32
	ascent  f32
	descent f32
	size    f32
	origin  f32
	base    int
}

pub fn (mut ts TextSystem) layout_line(s string, st TextStyle) &LineLayout {
	return ts.line(s, st, true)
}

// line lays out one line. s may view a larger string (owned false): a cached layout keeps
// a copy.
fn (mut ts TextSystem) line(s string, st TextStyle, owned bool) &LineLayout {
	face := ts.face_for(st)
	key := pair(u32(face), st.size)
	if l := ts.lines.get(s, key, 0) {
		ts.stats.line_hits++
		return l
	}
	text := if owned { s } else { s.clone() }
	l := ts.build_line(text, st)
	ts.lines.put(text, key, 0, l)
	return l
}

// layout_mono is layout_line on a monospace grid of cell_w (specs/04): every grapheme starts
// at display_width(s[..i]) × cell_w and a non-ASCII one is centred in its cells, so fallback
// glyphs never shift the columns after them. A pure ASCII line is the plain layout.
pub fn (mut ts TextSystem) layout_mono(s string, st TextStyle, cell_w f32) &LineLayout {
	if cell_w <= 0 || s.is_pure_ascii() {
		return ts.line(s, st, true)
	}
	key := pair(u32(ts.face_for(st)), st.size)
	k2 := u64(math.f32_bits(cell_w)) | u64(1) << 32 // never 0, the plain layout's slot
	if l := ts.lines.get(s, key, k2) {
		ts.stats.line_hits++
		return l
	}
	mut l := ts.build_line(s, st)
	snap_to_grid(mut l, cell_w)
	ts.lines.put(s, key, k2, l)
	return l
}

// snap_to_grid moves l's stops onto columns and each grapheme's glyphs with its first stop.
fn snap_to_grid(mut l LineLayout, cell_w f32) {
	s := l.text
	mut col := 0
	mut gi := 0
	for k in 0 .. l.stops.len - 1 {
		a, b := l.stops[k], l.stops[k + 1]
		w := grapheme_width(s, a.index, b.index)
		x := f32(col) * cell_w
		mut dx := x - a.x
		if b.index - a.index > 1 || s[a.index] >= 0x80 { // not a lone ASCII byte: centre it
			dx += (f32(w) * cell_w - (b.x - a.x)) / 2
		}
		for gi < l.glyphs.len && l.glyphs[gi].cluster < b.index {
			g := l.glyphs[gi]
			l.glyphs[gi] = PositionedGlyph{g.face, g.id, g.x + dx, g.y, g.cluster}
			gi++
		}
		l.stops[k] = CaretStop{a.index, x}
		col += w
	}
	l.width = f32(col) * cell_w
	l.stops[l.stops.len - 1] = CaretStop{s.len, l.width}
}

fn (mut ts TextSystem) build_line(s string, st TextStyle) &LineLayout {
	mut l := &LineLayout{
		glyphs: []PositionedGlyph{cap: s.len}
		stops:  []CaretStop{cap: s.len + 1}
	}
	ts.fill_line(mut l, s, st)
	return l
}

// fill_line lays out s into l, reusing l's glyph and stop arrays (cleared). l must not be
// shared with cut lines.
fn (mut ts TextSystem) fill_line(mut l LineLayout, s string, st TextStyle) {
	face := ts.face_for(st)
	mut glyphs := unsafe { l.glyphs }
	glyphs.clear()
	mut x := f32(0)
	unset := f32(-1.0e30)
	if ts.first_x.len < s.len {
		ts.first_x = []f32{len: s.len * 2}
	}
	mut first_x := unsafe { ts.first_x } // shares the scratch: no array per line
	for k in 0 .. s.len {
		first_x[k] = unset
	}
	mut i := 0
	for i < s.len {
		e := token_end(s, i, s.len)
		w := ts.word(s.substr_unsafe(i, e), face, st.size)
		for g in w.glyphs {
			cl := i + g.cluster
			if first_x[cl] == unset {
				first_x[cl] = x + g.x
			}
			glyphs << PositionedGlyph{g.face, g.id, x + g.x, g.y, cl}
		}
		x += w.width
		i = e
	}
	// Caret stops at grapheme boundaries; boundaries inside a ligature reuse the previous x.
	mut stops := unsafe { l.stops }
	stops.clear()
	mut last := f32(0)
	mut b := 0
	for {
		bx := if b >= s.len {
			x
		} else if first_x[b] != unset {
			first_x[b]
		} else {
			last
		}
		stops << CaretStop{b, bx}
		last = bx
		if b >= s.len {
			break
		}
		b = next_grapheme(s, b)
	}
	asc, desc := ts.line_metrics(st)
	l.text, l.glyphs, l.stops, l.width, l.ascent, l.descent, l.size = s, glyphs, stops, x, asc, desc, st.size
	l.origin, l.base = 0, 0
	ts.stats.lines_built++
}

// x_for_index returns the caret x for a byte offset (snapped to the grapheme at or before it).
pub fn (l &LineLayout) x_for_index(i int) f32 {
	mut x := l.origin
	for s in l.stops {
		if s.index > i + l.base {
			break
		}
		x = s.x
	}
	return x - l.origin
}

// index_for_x returns the grapheme boundary nearest to x.
pub fn (l &LineLayout) index_for_x(x f32) int {
	if l.stops.len == 0 {
		return 0
	}
	for k in 1 .. l.stops.len {
		mid := (l.stops[k - 1].x + l.stops[k].x) / 2
		if x + l.origin < mid {
			return l.stops[k - 1].index - l.base
		}
	}
	return l.stops[l.stops.len - 1].index - l.base
}

// cut returns bytes [a, e) of the line as a view sharing its glyphs and stops. a and e must
// be grapheme boundaries (line breaks are). A paragraph rewrapped at a new width so
// allocates only the views, never glyphs.
fn (l &LineLayout) cut(a int, e int) &LineLayout {
	g0, g1 := l.glyph_from(a + l.base), l.glyph_from(e + l.base)
	s0, s1 := l.stop_at(a + l.base), l.stop_at(e + l.base)
	return &LineLayout{
		text:    l.text.substr_unsafe(a, e)
		glyphs:  l.glyphs[g0..g1]
		stops:   l.stops[s0..s1 + 1]
		width:   l.stops[s1].x - l.stops[s0].x
		ascent:  l.ascent
		descent: l.descent
		size:    l.size
		origin:  l.stops[s0].x
		base:    a + l.base
	}
}

// glyph_from is the first glyph whose cluster is at or after byte i (glyphs are in order).
fn (l &LineLayout) glyph_from(i int) int {
	mut lo, mut hi := 0, l.glyphs.len
	for lo < hi {
		mid := (lo + hi) / 2
		if l.glyphs[mid].cluster < i {
			lo = mid + 1
		} else {
			hi = mid
		}
	}
	return lo
}

// stop_at is the stop at byte i, or the last one before it.
fn (l &LineLayout) stop_at(i int) int {
	mut lo, mut hi := 0, l.stops.len - 1
	for lo < hi {
		mid := (lo + hi + 1) / 2
		if l.stops[mid].index <= i {
			lo = mid
		} else {
			hi = mid - 1
		}
	}
	return lo
}

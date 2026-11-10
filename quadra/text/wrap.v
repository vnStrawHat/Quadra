// Line wrapping on cached word advances: O(words), no shaping in the loop.
module text

import math

pub struct WrappedLine {
pub:
	start  int // byte range in the source text
	end    int
	y      f32 // top of the line box
	layout &LineLayout
}

pub struct WrappedText {
pub mut:
	// read-only outside this module
	lines       []WrappedLine
	width       f32 // widest line, without trailing spaces
	height      f32
	line_height f32
	// width_breaks: some line ends because the next word did not fit (not at a newline).
	// Without them the same lines come out at any width from `width` up.
	width_breaks bool
}

pub fn (mut ts TextSystem) wrap(s string, st TextStyle, max_w f32) &WrappedText {
	face := ts.face_for(st)
	key := pair(u32(face), st.size)
	key2 := u64(math.f32_bits(st.line_height)) << 32 | u64(math.f32_bits(max_w))
	if w := ts.wraps.get(s, key, key2) {
		return w
	}
	mut ranges := []int{cap: 4} // start, end of each line
	mut para := 0
	mut paragraphs := 0
	for i := 0; i <= s.len; i++ {
		if i == s.len || s[i] == `\n` {
			// The slack absorbs float error in layout sums (68.004 - 24 = 44.003998 for a word
			// 44.004 wide), which would otherwise break a word that was measured to fit.
			ts.break_paragraph(s, para, i, face, st.size, max_w + 0.01, mut ranges)
			para = i + 1
			paragraphs++
		}
	}
	lh := st.line_px()
	n := ranges.len / 2
	mut lines := []WrappedLine{cap: n}
	mut width := f32(0)
	mut ps, mut pe := 0, -1 // the paragraph of the current line
	mut pl := &LineLayout(unsafe { nil })
	for k in 0 .. n {
		a, e := ranges[2 * k], ranges[2 * k + 1]
		if a > pe { // the first line of the next paragraph
			ps, pe = a, a
			for pe < s.len && s[pe] != `\x0a` {
				pe++
			}
			pl = unsafe { nil }
		}
		whole := ps == 0 && pe == s.len
		// A text that is one line keeps its layout here only, not also in the line cache. The
		// lines of a longer paragraph are cut from its layout, which survives rewraps.
		layout := if a == 0 && e == s.len && n == 1 {
			ts.build_line(s, st)
		} else if a == ps && e == pe {
			ts.line(if whole { s } else { s.substr_unsafe(a, e) }, st, whole)
		} else {
			if pl == unsafe { nil } {
				pl = ts.line(if whole { s } else { s.substr_unsafe(ps, pe) }, st, whole)
			}
			pl.cut(a - ps, e - ps)
		}
		lines << WrappedLine{a, e, f32(k) * lh, layout}
		mut t := e
		for t > a && (s[t - 1] == ` ` || s[t - 1] == `\t`) {
			t--
		}
		visible := layout.x_for_index(t - a)
		if visible > width {
			width = visible
		}
	}
	w := &WrappedText{lines, width, f32(n) * lh, lh, n > paragraphs}
	ts.wraps.put(s, key, key2, w)
	return w
}

// wrap_number lays out a number that fits on one line, as wrap does, into reuse (or a new
// WrappedText when reuse is nil), reusing its arrays and line layout: a text element keeps
// one (ui TextEl), so a number that changes every frame allocates nothing. Numbers are not
// cached. It returns none, leaving reuse alone, when s is not a number, the face does not
// compose numbers (numeric.v), or s does not fit max_w: then call wrap.
pub fn (mut ts TextSystem) wrap_number(s string, st TextStyle, max_w f32, reuse &WrappedText) ?&WrappedText {
	if s.len == 0 || !is_numeric(s) {
		return none
	}
	face := ts.face_for(st)
	if !ts.numeric_ok(face) {
		return none
	}
	lim := max_w + 0.01 // break_paragraph's slack
	if ts.compose_numeric(s, face, st.size).width > lim && lim > 0 {
		return none
	}
	mut w := if reuse != unsafe { nil } { unsafe { reuse } } else { &WrappedText{} }
	mut l := &LineLayout(unsafe { nil })
	if w.lines.len == 1 {
		l = unsafe { w.lines[0].layout }
	} else {
		l = &LineLayout{}
	}
	ts.fill_line(mut l, s, st)
	lh := st.line_px()
	w.lines.clear()
	w.lines << WrappedLine{0, s.len, 0, l}
	w.width, w.height, w.line_height, w.width_breaks = l.x_for_index(s.len), lh, lh, false
	return w
}

fn (mut ts TextSystem) break_paragraph(s string, start int, end int, face FaceId, size f32, max_w f32, mut out []int) {
	mut line_start := start
	mut x := f32(0)
	mut has_content := false
	mut i := start
	for i < end {
		e := token_end(s, i, end)
		w := ts.word(s.substr_unsafe(i, e), face, size)
		ts_ := i
		i = e
		if s[ts_] == ` ` || s[ts_] == `\t` {
			x += w.width // spaces hang past the edge instead of breaking
			continue
		}
		if has_content && x + w.width > max_w {
			out << line_start
			out << ts_
			line_start, x, has_content = ts_, 0, false
		}
		if w.width > max_w && max_w > 0 {
			// A word wider than the line breaks between graphemes.
			mut gx := f32(0)
			mut seg_start := ts_
			for g in w.glyphs {
				if gx + g.advance > max_w && ts_ + g.cluster > seg_start {
					out << line_start
					out << ts_ + g.cluster
					line_start = ts_ + g.cluster
					seg_start = line_start
					gx = 0
				}
				gx += g.advance
			}
			x = gx
		} else {
			x += w.width
		}
		has_content = true
	}
	out << line_start
	out << end
}

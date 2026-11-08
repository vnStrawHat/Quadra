// Rich text: runs of differently styled text wrapped as one paragraph flow. Widths come
// from the word cache and each piece of a line is a cached line layout, so painting and
// caret math work as for plain text.
module text

pub struct Run {
pub:
	text  string
	style TextStyle
	// box_w > 0 makes the run an inline object of that width (text is ignored), extending
	// box_asc above the baseline and box_desc below it. It wraps like a word.
	box_w    f32
	box_asc  f32
	box_desc f32
}

// RichPiece is the part of one run on one line: bytes [start, end) of the run's text.
pub struct RichPiece {
pub:
	run    int
	start  int
	end    int
	x      f32
	layout &LineLayout // empty for an inline object
	width  f32         // the layout's width, or the object's
}

pub struct RichLine {
pub:
	pieces   []RichPiece
	y        f32
	height   f32
	baseline f32 // from the line top
	width    f32 // without trailing spaces
}

pub struct RichText {
pub:
	lines        []RichLine
	width        f32
	height       f32
	width_breaks bool // as WrappedText.width_breaks
}

struct RichToken {
	run   int
	start int
	end   int
	width f32
	kind  u8 // 0 word, 1 space, 2 newline
}

struct RichFlow {
mut:
	lines  []RichLine
	ranges []int // run, start, end of the pieces of the line being filled
	y      f32
	width  f32
	broke  bool
}

// layout_rich wraps runs at max_w. Lines break at spaces: words glued across runs
// ("**bo**ld") stay together, and a newline in a run ends the line. A word wider than the
// line (a path, a URL) breaks inside, after a `/` `.` `-` `_` when one fits.
// ponytail: no cache across frames; add one when rich text holds large documents.
pub fn (mut ts TextSystem) layout_rich(runs []Run, max_w f32) &RichText {
	mut toks := []RichToken{cap: 16}
	for r, run in runs {
		if run.box_w > 0 {
			toks << RichToken{r, 0, 0, run.box_w, 0}
			continue
		}
		face := ts.face_for(run.style)
		s := run.text
		mut i := 0
		for i < s.len {
			e := token_end(s, i, s.len)
			kind := match s[i] {
				` `, `\t` { u8(1) }
				`\n` { u8(2) }
				else { u8(0) }
			}

			w := if kind == 2 {
				f32(0)
			} else {
				ts.word(s.substr_unsafe(i, e), face, run.style.size).width
			}
			toks << RichToken{r, i, e, w, kind}
			i = e
		}
	}
	mut f := RichFlow{}
	mut x, mut content_x := f32(0), f32(0)
	mut last_run := 0
	mut k := 0
	for k < toks.len {
		t := toks[k]
		last_run = t.run
		if t.kind == 2 {
			ts.finish_rich_line(runs, mut f, content_x, t.run)
			x, content_x = 0, 0
			k++
			continue
		}
		if t.kind == 1 {
			f.add(t)
			x += t.width
			k++
			continue
		}
		mut e := k // the unit: words up to the next space or newline
		mut uw := f32(0)
		for e < toks.len && toks[e].kind == 0 {
			uw += toks[e].width
			e++
		}
		if content_x > 0 && x + uw > max_w {
			ts.finish_rich_line(runs, mut f, content_x, t.run)
			f.broke = true
			x = 0
		}
		if uw > max_w && max_w > 0 {
			for j in k .. e {
				x = ts.split_token(runs, toks[j], mut f, x, max_w)
			}
			content_x = x
			k = e
			continue
		}
		for j in k .. e {
			f.add(toks[j])
		}
		x += uw
		content_x = x
		k = e
	}
	ts.finish_rich_line(runs, mut f, content_x, last_run)
	return &RichText{f.lines, f.width, f.y, f.broke}
}

// split_token puts a word on lines starting at x, breaking it where it passes max_w: after
// the last `/` `\` `.` `-` `_` that fits, else between glyphs. It returns the end x.
fn (mut ts TextSystem) split_token(runs []Run, t RichToken, mut f RichFlow, x0 f32, max_w f32) f32 {
	if x0 + t.width <= max_w || runs[t.run].box_w > 0 {
		f.add(t)
		return x0 + t.width
	}
	st := runs[t.run].style
	s := runs[t.run].text
	w := ts.word(s.substr_unsafe(t.start, t.end), ts.face_for(st), st.size)
	mut x := x0
	mut a := t.start // the start of the piece on this line
	mut px := f32(0) // its width so far
	mut soft, mut soft_w := -1, f32(0) // the last break after a separator, and the width to it
	for g in w.glyphs {
		at := t.start + g.cluster
		if at > a && x + px + g.advance > max_w {
			mut cut, mut cw := at, px
			if soft > a {
				cut, cw = soft, soft_w
			}
			f.add(RichToken{t.run, a, cut, cw, 0})
			ts.finish_rich_line(runs, mut f, x + cw, t.run)
			f.broke = true
			x, px, a, soft = 0, px - cw, cut, -1
		}
		px += g.advance
		if s[at] in [`/`, `\\`, `.`, `-`, `_`] {
			soft, soft_w = at + 1, px
		}
	}
	f.add(RichToken{t.run, a, t.end, px, 0})
	return x + px
}

// add appends a token to the line, extending the last piece when it continues it.
fn (mut f RichFlow) add(t RichToken) {
	n := f.ranges.len
	if n > 0 && f.ranges[n - 3] == t.run && f.ranges[n - 1] == t.start {
		f.ranges[n - 1] = t.end
		return
	}
	f.ranges << t.run
	f.ranges << t.start
	f.ranges << t.end
}

// finish_rich_line lays out the pieces of the line and starts the next one. run gives
// an empty line its metrics.
fn (mut ts TextSystem) finish_rich_line(runs []Run, mut f RichFlow, width f32, run int) {
	mut pieces := []RichPiece{cap: f.ranges.len / 3}
	mut x := f32(0)
	mut base, mut below := f32(0), f32(0)
	for p := 0; p < f.ranges.len; p += 3 {
		r, a, e := f.ranges[p], f.ranges[p + 1], f.ranges[p + 2]
		st := runs[r].style
		if runs[r].box_w > 0 {
			pieces << RichPiece{r, a, e, x, ts.line('', st, false), runs[r].box_w}
			x += runs[r].box_w
			b, d := runs[r].box_asc, runs[r].box_desc
			base, below = if b > base { b } else { base }, if d > below { d } else { below }
			continue
		}
		l := ts.line(runs[r].text.substr_unsafe(a, e), st, false)
		pieces << RichPiece{r, a, e, x, l, l.width}
		x += l.width
		b, d := ts.rich_metrics(st)
		base, below = if b > base { b } else { base }, if d > below { d } else { below }
	}
	if pieces.len == 0 && runs.len > 0 {
		base, below = ts.rich_metrics(runs[run].style)
	}
	f.lines << RichLine{pieces, f.y, base + below, base, width}
	f.y += base + below
	if width > f.width {
		f.width = width
	}
	f.ranges.clear()
}

// rich_metrics splits a style's line box at its baseline: the part above, the part below.
fn (mut ts TextSystem) rich_metrics(st TextStyle) (f32, f32) {
	asc, desc := ts.line_metrics(st)
	lead := (st.line_px() - asc - desc) / 2
	return lead + asc, desc + lead
}

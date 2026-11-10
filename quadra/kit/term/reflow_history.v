// Reflow of the scrollback at a new width (specs/17, Resize).
module term

// split_history reflows the history chain i..j (wrapped lines that form one logical line).
// ASCII chains are cut as text; others go through cells.
fn (s &Screen) split_history(i int, j int, cols int) []Line {
	mut ascii := true
	for h in i .. j + 1 {
		for b in s.scrollback.at(h).text {
			if b >= 0x80 {
				ascii = false
				break
			}
		}
	}
	if !ascii {
		mut line := LogicalLine{}
		for h in i .. j + 1 {
			l := s.scrollback.at(h)
			line.add(unpack(l, l.text.len * 2 + 1))
		}
		pieces, _ := line.split(cols)
		return pieces.map(pack(it))
	}
	if i == j && s.scrollback.at(i).text.len <= cols {
		return [s.scrollback.at(i)] // fits: kept as it is
	}
	mut text := ''
	mut runs := []Run{}
	mut extra := unsafe { &RowExtra(nil) }
	for h in i .. j + 1 {
		l := s.scrollback.at(h)
		extra = unsafe { merge_extra(extra, l.extra) } // rows share extras; reflow only reads them
		off := text.len
		for r in l.runs {
			runs << Run{u16(int(r.col) + off), r.fg, r.bg, r.flags, r.link}
		}
		text += l.text
	}
	mut out := []Line{}
	mut from := 0
	mut ri := 0 // first run not yet started
	mut active := ?Run(none)
	for {
		end := if from + cols < text.len { from + cols } else { text.len }
		mut piece := []Run{}
		for ri < runs.len && int(runs[ri].col) <= from {
			active = runs[ri]
			ri++
		}
		if a := active {
			piece << Run{0, a.fg, a.bg, a.flags, a.link}
		}
		for x := ri; x < runs.len && int(runs[x].col) < end; x++ {
			piece << Run{u16(int(runs[x].col) - from), runs[x].fg, runs[x].bg, runs[x].flags, runs[x].link}
		}
		mut first := unsafe { &RowExtra(nil) }
		if out.len == 0 {
			first = extra
		}
		out << Line{
			text:    text[from..end]
			runs:    piece
			wrapped: end < text.len
			extra:   first
		}
		if end >= text.len {
			break
		}
		from = end
	}
	return out
}

// reflow_history replaces the history lines before tail with their reflow at cols. It is a
// function of its own, and the result is assigned once: under -prod with boehm_full_opt,
// V pins every local array of pointerful structs around each call statement in scope, so a
// 10 000-line array held across a loop of calls costs O(n²) (specs/14).
fn (mut s Screen) reflow_history(tail int, cols int) {
	mut hist := []Line{cap: tail}
	mut h0 := 0
	for h0 < tail {
		mut j := h0
		for j < tail - 1 && s.scrollback.at(j).wrapped {
			j++
		}
		hist << s.split_history(h0, j, cols)
		h0 = j + 1
	}
	if hist.len > s.scrollback.max { // narrower rows make more lines: keep the newest
		hist = hist[hist.len - s.scrollback.max..].clone()
	}
	s.scrollback.lines, s.scrollback.head = hist, 0
}

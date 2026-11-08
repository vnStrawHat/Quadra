// Selection commands: clicks, select all/line/word, extra cursors, matching brackets.
module code

// set_cursor replaces all cursors with one (a click); add keeps the others (ctrl-click).
pub fn (mut d Doc) set_cursor(anchor int, head int, add bool) {
	c := Cursor{
		anchor: clamp(anchor, 0, d.buf.len())
		head:   clamp(head, 0, d.buf.len())
	}
	if add {
		d.cursors << c
	} else {
		d.cursors = [c]
	}
	d.normalize()
	d.break_group()
}

pub fn (mut d Doc) select_all() {
	d.cursors = [Cursor{
		anchor: 0
		head:   d.buf.len()
	}]
}

// select_line extends each cursor to whole lines, including the newline.
pub fn (mut d Doc) select_line() {
	for mut c in d.cursors {
		a := d.buf.line_start(d.buf.line_of(c.start()))
		el := d.buf.line_of(c.end())
		e := if el + 1 < d.buf.line_count() { d.buf.line_start(el + 1) } else { d.buf.len() }
		c.anchor, c.head = a, e
	}
	d.normalize()
}

// word_at returns the word around off (empty when off is not in a word).
pub fn (d &Doc) word_at(off int) (int, int) {
	mut a, mut e := off, off
	for a > 0 && class_of(d.buf.byte_at(a - 1)) == .word {
		a--
	}
	for e < d.buf.len() && class_of(d.buf.byte_at(e)) == .word {
		e++
	}
	return a, e
}

// add_cursor_vertical adds a cursor on the line above or below the first or last one,
// keeping the column (column selection keeps the anchor column too).
pub fn (mut d Doc) add_cursor_vertical(down bool) {
	edge := if down { d.cursors.last() } else { d.cursors[0] }
	line := d.buf.line_of(edge.head)
	target := if down { line + 1 } else { line - 1 }
	if target < 0 || target >= d.buf.line_count() {
		return
	}
	acol := d.col_of(edge.anchor)
	hcol := if edge.goal >= 0 { edge.goal } else { d.col_of(edge.head) }
	mut c := Cursor{
		anchor: d.off_at_col(target, acol)
		head:   d.off_at_col(target, hcol)
		goal:   hcol
	}
	if d.buf.line_of(edge.anchor) != line {
		c.anchor = c.head
	}
	d.cursors << c
	d.normalize()
}

// select_next selects the word at the primary cursor, or adds the next occurrence of the
// selected text as a new cursor (ctrl-d).
pub fn (mut d Doc) select_next() {
	p := d.cursors.last()
	if p.empty() {
		a, e := d.word_at(p.head)
		if a < e {
			d.cursors[d.cursors.len - 1] = Cursor{
				anchor: a
				head:   e
			}
		}
		return
	}
	needle := d.buf.slice(p.start(), p.end())
	hay := d.buf.text()
	mut i := hay.index_after(needle, d.cursors.last().end()) or { -1 } // cursors are sorted
	if i < 0 {
		i = hay.index(needle) or { -1 }
	}
	for i >= 0 && d.cursors.any(it.start() == i) {
		i = hay.index_after(needle, i + 1) or { -1 }
	}
	if i >= 0 {
		d.cursors << Cursor{
			anchor: i
			head:   i + needle.len
		}
		d.normalize()
	}
}

const bracket_open = '([{'
const bracket_close = ')]}'

// bracket_pair returns the bracket at or before the primary cursor and its match, or -1.
// ponytail: scans at most 20 000 bytes and ignores strings and comments.
pub fn (d &Doc) bracket_pair() (int, int) {
	h := d.primary().head
	for off in [h, h - 1] {
		if off < 0 || off >= d.buf.len() {
			continue
		}
		c := d.buf.byte_at(off)
		ko, kc := bracket_open.index_u8(c), bracket_close.index_u8(c)
		m := if ko >= 0 {
			d.scan_bracket(off, 1, c, bracket_close[ko])
		} else if kc >= 0 {
			d.scan_bracket(off, -1, c, bracket_open[kc])
		} else {
			-1
		}
		if m >= 0 {
			return off, m
		}
	}
	return -1, -1
}

fn (d &Doc) scan_bracket(from int, dir int, same u8, other u8) int {
	mut depth := 0
	mut i := from
	for _ in 0 .. 20000 {
		if i < 0 || i >= d.buf.len() {
			return -1
		}
		c := d.buf.byte_at(i)
		if c == same {
			depth++
		} else if c == other {
			depth--
			if depth == 0 {
				return i
			}
		}
		i += dir
	}
	return -1
}

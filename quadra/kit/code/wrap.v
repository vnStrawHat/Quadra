// Soft wrap (specs/18, Soft wrap): with Editor.wrap on, a line shows as one or more visual
// rows broken at the view width. Rows holds where every row starts; the helpers below map
// lines, rows and y, and work the same with wrap off (one row per line).
module code

import math
import quadra.text

struct Rows {
mut:
	doc     voidptr
	version int = -1
	cols    int
	tab     int
	first   []int // the first row of each line, len line_count + 1
	starts  []int // display offset (in the line) where each row starts
}

// row_breaks appends the display offsets where the rows of line s start (0 first), breaking
// after the last space that fits in cols, or inside a word longer than a row. Spaces may
// hang past the edge. Columns are display widths (a wide character takes two).
fn row_breaks(s string, cols int, mut out []int) {
	out << 0
	if cols <= 0 {
		return
	}
	mut start, mut col, mut brk, mut since := 0, 0, 0, 0
	mut i := 0
	for i < s.len {
		j, w := cell_step(s, i)
		if s[i] == ` ` {
			col++
			brk, since = j, 0
			i = j
			continue
		}
		if col + w > cols && col > 0 {
			if brk > start {
				start, col = brk, since
			} else {
				start, col, since = i, 0, 0
			}
			brk = start
			out << start
		}
		col += w
		since += w
		i = j
	}
}

// cell_step is the end of the grapheme at i and its display width (one byte per ASCII
// character not followed by a combining mark).
fn cell_step(s string, i int) (int, int) {
	if s[i] < 0x80 && (i + 1 >= s.len || s[i + 1] < 0x80) {
		return i + 1, 1
	}
	j := text.next_grapheme(s, i)
	return j, text.grapheme_width(s, i, j)
}

// sync_rows rebuilds the row table when the text, the width or the tab size changed.
// ponytail: a full pass over the text per edit (no layout, one copy); track Doc.dirty if
// huge files make it show.
fn (mut e Editor) sync_rows() {
	if !e.wrap || e.cell.width <= 0 {
		return
	}
	d := e.doc
	cols := math.max(8, int(e.bounds.width / e.cell.width) - 1)
	if e.rows.doc == voidptr(d) && e.rows.version == d.version && e.rows.cols == cols
		&& e.rows.tab == d.tab_size {
		return
	}
	t := d.buf.text()
	n := d.buf.line_count()
	e.rows.first = []int{cap: n + 1}
	e.rows.starts.clear()
	for k in 0 .. n {
		a, b := d.buf.line_start(k), d.buf.line_end(k)
		e.rows.first << e.rows.starts.len
		dl := display_line(unsafe { tos(t.str + a, b - a) }, d.tab_size)
		row_breaks(dl.shown, cols, mut e.rows.starts)
	}
	e.rows.first << e.rows.starts.len
	e.rows.doc, e.rows.version, e.rows.cols, e.rows.tab = voidptr(d), d.version, cols, d.tab_size
}

fn (e &Editor) wrapped() bool {
	return e.wrap && e.rows.first.len > 0
}

fn (e &Editor) row_count() int {
	return if e.wrapped() { e.rows.first.last() } else { e.doc.buf.line_count() }
}

// first_row is the first visual row of line n (n == line_count gives the row count).
fn (e &Editor) first_row(n int) int {
	if !e.wrapped() {
		return n
	}
	f := e.rows.first
	return if n < f.len { f[n] } else { f.last() + n - f.len + 1 }
}

// row_line is the line that row r belongs to.
fn (e &Editor) row_line(r int) int {
	last := e.doc.buf.line_count() - 1
	if !e.wrapped() {
		return clamp(r, 0, last)
	}
	f := e.rows.first
	mut lo, mut hi := 0, f.len - 2
	for lo < hi {
		mid := (lo + hi + 1) / 2
		if f[mid] <= r {
			lo = mid
		} else {
			hi = mid - 1
		}
	}
	return clamp(lo, 0, last)
}

// row_span is the display range of row r of line n (max_int: to the end of the line).
fn (e &Editor) row_span(n int, r int) (int, int) {
	if !e.wrapped() {
		return 0, max_int
	}
	s := e.rows.starts
	a := if r < s.len { s[r] } else { 0 }
	b := if r + 1 < e.first_row(n + 1) && r + 1 < s.len { s[r + 1] } else { max_int }
	return a, b
}

// row_at_col is the row of line n that shows display offset k.
fn (e &Editor) row_at_col(n int, k int) int {
	mut r := e.first_row(n)
	end := e.first_row(n + 1)
	for r + 1 < end && r + 1 < e.rows.starts.len && e.rows.starts[r + 1] <= k {
		r++
	}
	return r
}

fn (e &Editor) row_y(r int) f32 {
	return e.bounds.y + f32(r) * e.cell.height - e.scroll.y
}

fn (e &Editor) row_at_y(y f32) int {
	if e.cell.height <= 0 {
		return 0
	}
	return clamp(int(math.floor((y - e.bounds.y + e.scroll.y) / e.cell.height)), 0,
		e.row_count() - 1)
}

// caret is the x of offset off from the start of its row (before scrolling) and the row.
fn (e &Editor) caret(ts &text.TextSystem, off int) (f32, int) {
	d := e.doc
	n := d.buf.line_of(off)
	dl := display_line(d.buf.line(n), d.tab_size)
	mut sys := unsafe { ts }
	l := sys.layout_mono(dl.shown, e.style, e.cell.width)
	k := dl.to_shown(off - d.buf.line_start(n))
	r := e.row_at_col(n, k)
	rs, _ := e.row_span(n, r)
	return l.x_for_index(k) - l.x_for_index(rs), r
}

// top_line is the line at the top of the view, plus the part of its rows scrolled past.
// It refreshes the row table first: the text may have changed since the last paint.
pub fn (mut e Editor) top_line() f32 {
	if e.cell.height <= 0 {
		return 0
	}
	e.sync_rows()
	pos := e.scroll.y / e.cell.height
	n := e.row_line(int(pos))
	r0, r1 := e.first_row(n), e.first_row(n + 1)
	return f32(n) + frac((pos - f32(r0)) / f32(math.max(1, r1 - r0)))
}

// scroll_to_line scrolls so that line n's first row is at the top (clamped).
pub fn (mut e Editor) scroll_to_line(n int) {
	e.sync_rows()
	k := clamp(n, 0, e.doc.buf.line_count() - 1)
	e.scroll.y = f32(e.first_row(k)) * e.cell.height
	e.clamp_scroll()
}

fn frac(v f32) f32 {
	return if v < 0 {
		0
	} else if v > 0.999 {
		0.999
	} else {
		v
	}
}

// move_rows moves every cursor by dn visual rows, keeping its column inside the row.
fn (mut e Editor) move_rows(dn int, extend bool) {
	e.sync_rows()
	mut d := e.doc
	last := e.row_count() - 1
	for mut c in d.cursors {
		n := d.buf.line_of(c.head)
		dl := display_line(d.buf.line(n), d.tab_size)
		k := dl.to_shown(c.head - d.buf.line_start(n))
		r := e.row_at_col(n, k)
		if c.goal < 0 {
			rs, _ := e.row_span(n, r)
			c.goal = text.display_width(dl.shown.substr_unsafe(rs, k))
		}
		t := r + dn
		c.head = if t < 0 {
			0
		} else if t > last {
			d.buf.len()
		} else {
			e.off_in_row(t, c.goal)
		}
		if !extend {
			c.anchor = c.head
		}
	}
	d.normalize()
	d.break_group()
}

// off_in_row is the offset at column col of row r (before the row's end).
fn (e &Editor) off_in_row(r int, col int) int {
	d := e.doc
	n := e.row_line(r)
	dl := display_line(d.buf.line(n), d.tab_size)
	rs, re := e.row_span(n, r)
	end := if re < dl.shown.len { text.prev_grapheme(dl.shown, re) } else { dl.shown.len }
	mut i, mut c := rs, 0
	for i < end && c < col {
		j, w := cell_step(dl.shown, i)
		if c + w > col && col - c <= w / 2 { // the half rule of off_at_col
			break
		}
		i, c = j, c + w
	}
	return d.buf.line_start(n) + dl.to_src(i)
}

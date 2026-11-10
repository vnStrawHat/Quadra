// URL detection: the click-time scan of a wrapped row group, and the coloring mask (specs/22).
// The rules are the shared detector's (quadra/text, url.v), the same as the editor's and
// Markdown's: a deliberate departure from the original (specs/22, Links).
module term

import quadra.text

const url_window = 5 // rows searched on either side of the pointer for a wrapped URL

// is_white is char::is_whitespace of the original.
fn is_white(r rune) bool {
	return text.is_white(r)
}

fn visible_rune(c Cell) rune {
	return if c.ch == 0 || c.ch == 9 { ` ` } else { rune(c.ch) }
}

// plain_url_at finds the plain-text URL through a cell, following soft wraps.
fn (t &Terminal) plain_url_at(p Pos) ?Link {
	last := t.screen.scrollback.len() + t.screen.rows - 1
	lo := if p.line - url_window > 0 { p.line - url_window } else { 0 }
	hi := if p.line + url_window < last { p.line + url_window } else { last }
	mut first := p.line
	for first > lo && t.row_at(first - 1).wrapped {
		first--
	}
	mut chars := []rune{cap: t.screen.cols * 2}
	mut cols := []Pos{cap: t.screen.cols * 2}
	mut line := first
	for line <= hi {
		row := t.row_at(line)
		for x, c in row.cells {
			if c.flags & f_spacer == 0 {
				chars << visible_rune(c)
				cols << Pos{line, x}
			}
		}
		if !row.wrapped {
			break
		}
		line++
	}
	mut click := -1
	for i, q in cols {
		if q.line == p.line && q.col == p.col {
			click = i
			break
		}
	}
	if click < 0 || is_white(chars[click]) {
		return none
	}
	for begin := click; begin >= 0; begin-- {
		plen, run := text.url_run(chars, begin)
		if plen == 0 || click >= run {
			continue
		}
		end := text.url_trim(chars, begin + plen, run)
		if end <= begin + plen {
			continue
		}
		a, b := cols[begin], cols[end - 1]
		return Link{
			line:     a.line
			c0:       a.col
			c1:       b.col + 1
			url:      text.url_text(chars[begin..end])
			end_line: b.line
		}
	}
	return none
}

// may_link says whether a row can hold a link: an OSC 8 cell, or the ':' or '.' that every
// URL prefix has. Most rows have neither, and then scanning them allocates nothing.
fn (t &Terminal) may_link(row Row) bool {
	for c in row.cells {
		if c.ch == `:` || c.ch == `.` || (c.pad != 0 && t.screen.link_id(c) != none) {
			return true
		}
	}
	return false
}

// scan_row marks the row's OSC 8 cells and its plain-text URLs, not yet extended over wraps
// or stripped of trailing punctuation; empty when the row has none.
fn (t &Terminal) scan_row(row Row) []bool {
	if !t.may_link(row) {
		return []bool{}
	}
	cells := row.cells
	n := cells.len
	mut mask := []bool{len: n}
	mut chars := []rune{len: n}
	for x, c in cells {
		chars[x] = if c.flags & f_spacer != 0 { rune(` `) } else { visible_rune(c) }
		mask[x] = c.flags & f_spacer == 0 && t.screen.link_id(c) != none
	}
	mut i := 0
	for i < n {
		if mask[i] {
			i++
			continue
		}
		plen, end := text.url_run(chars, i)
		if plen > 0 && end > i + plen {
			for k in i .. end {
				mask[k] = true
			}
			i = end
		} else {
			i++
		}
	}
	return mask
}

// link_cols lists the column ranges of the links in a wrap run, one Link per row (`line` is
// the row's index in `rows`), for an app's URL coloring.
pub fn (t &Terminal) link_cols(rows []Row) []Link {
	mut masks := rows.map(t.scan_row(it))
	// A URL reaching the end of a wrapped row continues on the next.
	for i in 0 .. rows.len - 1 {
		if !rows[i].wrapped || masks[i].len == 0 || !masks[i].last() {
			continue
		}
		mut current := i + 1
		for current < rows.len {
			mut stop := false
			for x, c in rows[current].cells {
				if c.flags & f_spacer != 0 {
					continue
				}
				if text.url_stop(visible_rune(c)) {
					stop = true
					break
				}
				if masks[current].len == 0 {
					masks[current] = []bool{len: rows[current].cells.len}
				}
				masks[current][x] = true
			}
			if stop || !rows[current].wrapped {
				break
			}
			current++
		}
	}
	mut out := []Link{}
	mut carry := 0 // the parentheses a URL running on to the next row left open
	for r, row in rows {
		mut open := carry
		carry = 0
		mask := masks[r]
		n := mask.len
		mut col := 0
		for col < n {
			if !mask[col] {
				col++
				continue
			}
			begin := col
			for col < n && mask[col] {
				col++
			}
			end := col
			if begin > 0 {
				open = 0
			}
			if end >= n && row.wrapped { // the URL runs on: its end is on a later row
				carry = open + cell_parens(row.cells, begin, end)
				out << Link{r, begin, end, '', r, '', false}
				continue
			}
			mut stripped := end
			for stripped > begin + 1 {
				c := row.cells[stripped - 1]
				ch := visible_rune(c)
				if c.flags & f_spacer == 0 && (!text.url_trailing(ch) || (ch == `)`
					&& open + cell_parens(row.cells, begin, stripped - 1) > 0)) {
					break
				}
				stripped--
			}
			open = 0
			out << Link{r, begin, stripped, '', r, '', false}
		}
	}
	return out
}

// cell_parens is text.paren_depth over cells a..b: a `)` closing a `(` stays in the URL.
fn cell_parens(cells []Cell, a int, b int) int {
	mut depth := 0
	for c in cells[a..b] {
		if c.ch == `(` {
			depth++
		} else if c.ch == `)` {
			depth--
		}
	}
	return depth
}

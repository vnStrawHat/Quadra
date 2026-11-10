// URL detection: the click-time scan of a wrapped row group, and the coloring mask (specs/22).
module term

const url_window = 5 // rows searched on either side of the pointer for a wrapped URL
const url_prefixes = ['https://', 'http://', 'ftp://', 'www.']

fn is_trailing_punct(r rune) bool {
	return r in [`)`, `]`, `}`, `>`, `.`, `,`, `;`, `:`, `!`, `?`, `'`, `"`, `\``]
}

fn visible_rune(c Cell) rune {
	return if c.ch == 0 || c.ch == 9 { ` ` } else { rune(c.ch) }
}

fn starts_with_at(chars []rune, at int, p string) bool {
	if at + p.len > chars.len {
		return false
	}
	for i in 0 .. p.len {
		if chars[at + i] != rune(p[i]) {
			return false
		}
	}
	return true
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
		for prefix in url_prefixes {
			if !starts_with_at(chars, begin, prefix) {
				continue
			}
			mut end := begin + prefix.len
			for end < chars.len && !is_white(chars[end]) {
				end++
			}
			if click >= end {
				continue
			}
			for end > begin + prefix.len && is_trailing_punct(chars[end - 1]) {
				end--
			}
			if end <= begin + prefix.len {
				continue
			}
			mut url := chars[begin..end].string()
			if url.starts_with('www.') {
				url = 'https://${url}'
			}
			a, b := cols[begin], cols[end - 1]
			return Link{
				line:     a.line
				c0:       a.col
				c1:       b.col + 1
				url:      url
				end_line: b.line
			}
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
		mut matched := false
		for prefix in url_prefixes {
			if !starts_with_at(chars, i, prefix) {
				continue
			}
			mut end := i + prefix.len
			for end < n && !is_white(chars[end]) {
				end++
			}
			if end > i + prefix.len {
				for k in i .. end {
					mask[k] = true
				}
				i = end
				matched = true
				break
			}
		}
		if !matched {
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
				if is_white(visible_rune(c)) {
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
	for r, row in rows {
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
			if end >= n && row.wrapped { // the URL runs on: its end is on a later row
				out << Link{r, begin, end, '', r, '', false}
				continue
			}
			mut stripped := end
			for stripped > begin + 1 {
				c := row.cells[stripped - 1]
				if c.flags & f_spacer != 0 || is_trailing_punct(visible_rune(c)) {
					stripped--
				} else {
					break
				}
			}
			out << Link{r, begin, stripped, '', r, '', false}
		}
	}
	return out
}

// Search over the history and the screen (specs/22).
module term

import encoding.utf8

pub struct Hit {
pub:
	line int
	col  int
	len  int // cells
}

pub struct Finder {
pub mut:
	query          string
	case_sensitive bool
	whole_word     bool
	hits           []Hit
	current        int = -1
	due            bool // a refresh after output is scheduled
}

fn fold(ch u32) u32 {
	if ch < 128 {
		return if ch >= `A` && ch <= `Z` { ch + 32 } else { ch }
	}
	r := utf8.to_lower(rune(ch).str()).runes()
	return if r.len == 1 { u32(r[0]) } else { ch }
}

fn word_rune(ch u32) bool {
	return ch > 127 || (ch >= `0` && ch <= `9`) || (ch >= `a` && ch <= `z`)
		|| (ch >= `A` && ch <= `Z`) || ch == `_`
}

// Each row is matched on its own, as in the original. ponytail: a whole-history scan per refresh;
// index the packed lines if long histories make this show up in a profile.
fn (t &Terminal) find_hits() []Hit {
	f := t.find
	mut q := f.query.runes().map(u32(it))
	if !f.case_sensitive {
		q = q.map(fold(it))
	}
	mut out := []Hit{}
	if q.len == 0 {
		return out
	}
	mut chars := []u32{cap: t.screen.cols}
	mut cols := []int{cap: t.screen.cols}
	for line in 0 .. t.screen.scrollback.len() + t.screen.rows {
		chars.clear()
		cols.clear()
		for x, c in t.row_at(line).cells {
			if c.flags & f_spacer == 0 {
				ch := if c.ch == 0 || c.ch == 9 { u32(32) } else { c.ch }
				chars << if f.case_sensitive { ch } else { fold(ch) }
				cols << x
			}
		}
		for i := 0; i + q.len <= chars.len; i++ {
			if !same_at(chars, i, q) {
				continue
			}
			if f.whole_word && ((i > 0 && word_rune(chars[i - 1]))
				|| (i + q.len < chars.len && word_rune(chars[i + q.len]))) {
				continue
			}
			end := if i + q.len < cols.len { cols[i + q.len] } else { t.screen.cols }
			out << Hit{line, cols[i], end - cols[i]}
			i += q.len - 1
		}
	}
	return out
}

fn same_at(chars []u32, at int, q []u32) bool {
	for k in 0 .. q.len {
		if chars[at + k] != q[k] {
			return false
		}
	}
	return true
}

// search starts a new search at the first match (the oldest; matches run top to bottom) and
// shows it.
pub fn (mut t Terminal) search(query string, case_sensitive bool, whole_word bool) {
	t.find.query, t.find.case_sensitive, t.find.whole_word = query, case_sensitive, whole_word
	t.find.hits = t.find_hits()
	t.find.current = if t.find.hits.len > 0 { 0 } else { -1 }
	t.reveal_hit()
}

// search_step moves to the next (+1) or previous (-1) match, wrapping around.
pub fn (mut t Terminal) search_step(delta int) {
	n := t.find.hits.len
	if n > 0 {
		if t.find.current < 0 {
			t.find.current = 0
		} else {
			t.find.current = (t.find.current + delta + n) % n
		}
		t.reveal_hit()
	}
}

// search_refresh finds the matches again after output, keeping the current index when it is
// still in range (the original drops it otherwise; the counter then reads 1).
fn (mut t Terminal) search_refresh() {
	t.find.hits = t.find_hits()
	if t.find.current >= t.find.hits.len {
		t.find.current = -1
	}
}

pub fn (mut t Terminal) search_clear() {
	t.find = Finder{}
}

// reveal_hit centres the current match.
fn (mut t Terminal) reveal_hit() {
	if t.find.current >= 0 {
		n := t.screen.scrollback.len()
		t.offset = clamp(n - (t.find.hits[t.find.current].line - t.screen.rows / 2), 0, n)
	}
}

// Find and replace over a Doc (specs/18): plain or regex, match case, whole word.
module code

import regex

pub struct Find {
pub mut:
	query string
	case  bool // match case
	word  bool // whole word
	regex bool
mut:
	key     string
	version int = -1
	hits    []int // start, end pairs, sorted
	err     string
}

// matches returns the match bounds as start, end pairs, recomputed when the query, the
// options or the document change.
// ponytail: a full rescan per document version; shift the old hits by the edit if typing
// with the bar open on huge files shows in profiles.
pub fn (mut f Find) matches(d &Doc) []int {
	key := '${f.case}${f.word}${f.regex}:${f.query}'
	if key == f.key && d.version == f.version {
		return f.hits
	}
	f.key, f.version = key, d.version
	f.hits = f.search(d.text())
	return f.hits
}

// error is the regex compile error of the current query, or ''.
pub fn (f &Find) error() string {
	return f.err
}

fn (mut f Find) search(hay string) []int {
	f.err = ''
	if f.query == '' {
		return []int{}
	}
	mut raw := []int{}
	if f.regex {
		mut re := regex.regex_opt(f.query) or {
			f.err = err.msg()
			return []int{}
		}
		if !f.case {
			re.flag |= regex.f_ci
		}
		raw = re.find_all(hay)
	} else {
		h := if f.case { hay } else { ascii_lower(hay) }
		q := if f.case { f.query } else { ascii_lower(f.query) }
		mut i := h.index_after(q, 0) or { -1 }
		for i >= 0 {
			raw << i
			raw << i + q.len
			i = h.index_after(q, i + q.len) or { -1 }
		}
	}
	if !f.word {
		return raw
	}
	mut out := []int{cap: raw.len}
	for k := 0; k + 1 < raw.len; k += 2 {
		a, e := raw[k], raw[k + 1]
		if (a == 0 || class_of(hay[a - 1]) != .word) && (e >= hay.len || class_of(hay[e]) != .word) {
			out << a
			out << e
		}
	}
	return out
}

fn ascii_lower(s string) string {
	mut b := s.bytes()
	for i, c in b {
		if c >= `A` && c <= `Z` {
			b[i] = c + 32
		}
	}
	return b.bytestr()
}

// current is the index of the match the primary cursor selects, or -1.
pub fn (mut f Find) current(d &Doc) int {
	p := d.primary()
	hits := f.matches(d)
	for k := 0; k + 1 < hits.len; k += 2 {
		if hits[k] == p.start() && hits[k + 1] == p.end() {
			return k / 2
		}
	}
	return -1
}

// next selects the match after the primary cursor (before it when back), wrapping.
pub fn (mut f Find) next(mut d Doc, back bool) bool {
	hits := f.matches(d)
	if hits.len == 0 {
		return false
	}
	p := d.primary()
	n := hits.len / 2
	mut pick := if back { n - 1 } else { 0 }
	for k in 0 .. n {
		if back {
			if hits[2 * (n - 1 - k)] < p.start() {
				pick = n - 1 - k
				break
			}
		} else if hits[2 * k] >= p.end() && !(hits[2 * k] == p.start()
			&& hits[2 * k + 1] == p.end()) {
			pick = k
			break
		}
	}
	d.set_cursor(hits[2 * pick], hits[2 * pick + 1], false)
	return true
}

// replace replaces the selected match (if the primary cursor selects one) and moves on.
pub fn (mut f Find) replace(mut d Doc, with string) {
	if f.current(d) >= 0 {
		p := d.primary()
		d.cursors = [p]
		d.apply([
			Edit{
				a:     p.start()
				e:     p.end()
				text:  with
				caret: with.len
			},
		], 'replace')
	}
	f.next(mut d, false)
}

// replace_all replaces every match as one undo group and returns the count.
pub fn (mut f Find) replace_all(mut d Doc, with string) int {
	hits := f.matches(d).clone()
	mut edits := []Edit{cap: hits.len / 2}
	for k := 0; k + 1 < hits.len; k += 2 {
		edits << Edit{
			a:    hits[k]
			e:    hits[k + 1]
			text: with
		}
	}
	d.apply(edits, 'replace-all')
	return edits.len
}

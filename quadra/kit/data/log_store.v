// Log storage and filtering (specs/19, Log view): lines packed in one byte buffer with an
// offset and a timestamp each, and a matcher for the text or regex filter.
module data

import regex

pub struct LogEntry {
pub:
	at_ms i64 // unix ms; 0 = no timestamp
	text  string
}

pub struct LogStore {
mut:
	bytes []u8
	offs  []u32 = [u32(0)] // line i is bytes[offs[i]..offs[i + 1]]
	ts    []i64
}

pub fn (s &LogStore) len() int {
	return s.ts.len
}

pub fn (mut s LogStore) push(at_ms i64, text string) {
	unsafe { s.bytes.push_many(text.str, text.len) }
	s.offs << u32(s.bytes.len)
	s.ts << at_ms
}

pub fn (s &LogStore) line(i int) string {
	return s.bytes[int(s.offs[i])..int(s.offs[i + 1])].bytestr()
}

pub fn (s &LogStore) at(i int) i64 {
	return s.ts[i]
}

pub fn (mut s LogStore) clear() {
	s.bytes.clear()
	s.offs = [u32(0)]
	s.ts.clear()
}

pub enum FilterMode {
	hide // only matching lines are listed
	dim  // every line is listed; the others are faint
}

// Matcher tests lines against the filter: a case-insensitive substring, or a regex.
struct Matcher {
mut:
	query  string // lowered
	use_re bool
	re     regex.RE
	active bool
}

// new_matcher compiles a filter; a bad regex returns an error and matches everything.
fn new_matcher(query string, use_re bool) !Matcher {
	if query == '' {
		return Matcher{}
	}
	if use_re {
		mut re := regex.regex_opt(query)!
		re.flag |= regex.f_ci
		return Matcher{
			use_re: true
			re:     re
			active: true
		}
	}
	return Matcher{
		query:  query.to_lower()
		active: true
	}
}

// matches tests the visible text of a line (escape sequences are not matched).
fn (mut m Matcher) matches(line string) bool {
	if !m.active {
		return true
	}
	s := if line.contains_u8(0x1b) { strip_escapes(line) } else { line }
	if m.use_re {
		a, _ := m.re.find(s)
		return a >= 0
	}
	return s.to_lower().contains(m.query)
}

// strip_escapes drops ESC [ ... final sequences (enough for matching).
fn strip_escapes(s string) string {
	mut out := []u8{cap: s.len}
	mut i := 0
	for i < s.len {
		if s[i] == 0x1b && i + 1 < s.len && s[i + 1] == `[` {
			i += 2
			for i < s.len && !(s[i] >= 0x40 && s[i] <= 0x7e) {
				i++
			}
			i++
			continue
		}
		out << s[i]
		i++
	}
	return out.bytestr()
}

// clock formats unix ms as hh:mm:ss.mmm (UTC, so tests read the same everywhere).
pub fn clock(ms i64) string {
	day := ms % 86_400_000
	h, m, s, f := day / 3_600_000, day / 60_000 % 60, day / 1000 % 60, day % 1000
	return '${h:02}:${m:02}:${s:02}.${f:03}'
}

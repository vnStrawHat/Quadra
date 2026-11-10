// Grapheme cluster boundaries: what one caret step or one backspace covers.
// ponytail: rules cover combining marks, variation selectors, ZWJ emoji sequences,
// skin tones, regional-indicator flags and CRLF; Hangul/Indic conjuncts are post-v1.
module text

fn is_extend(r rune) bool {
	return (r >= 0x0300 && r <= 0x036F) || (r >= 0x1AB0 && r <= 0x1AFF)
		|| (r >= 0x1DC0 && r <= 0x1DFF) || (r >= 0x20D0 && r <= 0x20FF)
		|| (r >= 0xFE20 && r <= 0xFE2F) || (r >= 0xFE00 && r <= 0xFE0F)
		|| (r >= 0xE0100 && r <= 0xE01EF) || (r >= 0x1F3FB && r <= 0x1F3FF)
		|| r == 0x200C
}

fn is_regional(r rune) bool {
	return r >= 0x1F1E6 && r <= 0x1F1FF
}

// decode returns the rune at byte offset i and its length.
// It runs for every byte of every new line layout, so well-formed UTF-8 is decoded in place
// (no substring, no rune array); anything else takes the general path.
fn decode(s string, i int) (rune, int) {
	c := s[i]
	if c < 0x80 {
		return rune(c), 1
	}
	n := utf8_char_len(c)
	if n >= 2 && i + n <= s.len {
		mut r := u32(c) & (u32(0x7f) >> n)
		mut ok := true
		for k in 1 .. n {
			b := s[i + k]
			ok = ok && b & 0xc0 == 0x80
			r = r << 6 | (u32(b) & 0x3f)
		}
		if ok {
			return rune(r), n
		}
	}
	end := if i + n > s.len { s.len } else { i + n }
	r := s[i..end].runes()
	return if r.len > 0 { r[0] } else { rune(s[i]) }, end - i
}

// next_grapheme returns the byte offset of the boundary after the cluster starting at i.
pub fn next_grapheme(s string, i int) int {
	if i >= s.len {
		return s.len
	}
	first, n := decode(s, i)
	mut j := i + n
	if first == `\r` && j < s.len && s[j] == `\n` {
		return j + 1
	}
	mut prev := first
	mut regional := if is_regional(first) { 1 } else { 0 }
	for j < s.len {
		r, m := decode(s, j)
		if is_extend(r) || r == 0x200D {
			prev = r
			j += m
			continue
		}
		if prev == 0x200D {
			prev = r
			j += m
			continue
		}
		if is_regional(r) && regional == 1 {
			regional = 2
			prev = r
			j += m
			continue
		}
		break
	}
	return j
}

// boundaries lists every grapheme boundary of s, including 0 and s.len.
pub fn boundaries(s string) []int {
	mut out := [0]
	mut i := 0
	for i < s.len {
		i = next_grapheme(s, i)
		out << i
	}
	return out
}

// prev_grapheme returns the start of the cluster ending at byte offset i.
pub fn prev_grapheme(s string, i int) int {
	if i <= 0 {
		return 0
	}
	mut last := 0
	mut j := 0
	for j < i {
		last = j
		j = next_grapheme(s, j)
	}
	return last
}

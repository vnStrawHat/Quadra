// Small string helpers: Unicode whitespace, ASCII case folding.
module completion

fn is_ws(r rune) bool {
	return match r {
		9...13, 0x20, 0x85, 0xA0, 0x1680, 0x2000...0x200A, 0x2028, 0x2029, 0x202F, 0x205F, 0x3000 { true }
		else { false }
	}
}

// decode_rune returns the rune at byte `i` and its byte width.
fn decode_rune(s string, i int) (rune, int) {
	b := s[i]
	if b < 0x80 {
		return rune(b), 1
	}
	w := utf8_char_len(b)
	if i + w > s.len {
		return rune(0xFFFD), 1
	}
	return s[i..i + w].runes()[0], w
}

fn split_ws(s string) []string {
	mut out := []string{}
	mut start := -1
	mut i := 0
	for i < s.len {
		r, w := decode_rune(s, i)
		if is_ws(r) {
			if start >= 0 {
				out << s[start..i]
				start = -1
			}
		} else if start < 0 {
			start = i
		}
		i += w
	}
	if start >= 0 {
		out << s[start..]
	}
	return out
}

// first_field is split_ws(s)[0] without the array.
fn first_field(s string) string {
	mut start := -1
	mut i := 0
	for i < s.len {
		r, w := decode_rune(s, i)
		if is_ws(r) {
			if start >= 0 {
				return s[start..i]
			}
		} else if start < 0 {
			start = i
		}
		i += w
	}
	return if start >= 0 { s[start..] } else { '' }
}

fn lower_ascii(c u8) u8 {
	return if c >= `A` && c <= `Z` { c + 32 } else { c }
}

fn eq_fold(a string, b string) bool {
	if a.len != b.len {
		return false
	}
	for i in 0 .. a.len {
		if lower_ascii(a[i]) != lower_ascii(b[i]) {
			return false
		}
	}
	return true
}

// prefix_match is a family-aware prefix test (byte compare, safe on UTF-8).
fn prefix_match(haystack string, prefix string, case_insensitive bool) bool {
	if prefix.len > haystack.len {
		return false
	}
	if case_insensitive {
		return eq_fold(haystack[..prefix.len], prefix)
	}
	return haystack.starts_with(prefix)
}

fn is_ascii(s string) bool {
	for c in s {
		if c >= 0x80 {
			return false
		}
	}
	return true
}

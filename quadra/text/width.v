// Display widths in monospace cells: what a terminal grid and a column-aligned text (a
// Markdown table) count.
module text

fn in_ranges(r u32, t []u32) bool {
	if r < t[0] || r > t[t.len - 1] {
		return false
	}
	mut lo, mut hi := 0, t.len / 2 - 1
	for lo <= hi {
		m := (lo + hi) / 2
		if r < t[2 * m] {
			hi = m - 1
		} else if r > t[2 * m + 1] {
			lo = m + 1
		} else {
			return true
		}
	}
	return false
}

// rune_width is the number of cells a code point takes: 0 for combining marks, 2 for East
// Asian wide characters and emoji, 1 otherwise.
pub fn rune_width(r u32) int {
	if r < 0x300 {
		return 1
	}
	if in_ranges(r, zero_ranges) {
		return 0
	}
	return if in_ranges(r, wide_ranges) { 2 } else { 1 }
}

// display_width is the number of cells s takes: each grapheme counts as its widest code
// point, so a letter with combining marks is 1 and a ZWJ emoji sequence is 2.
pub fn display_width(s string) int {
	mut w := 0
	mut i := 0
	for i < s.len {
		if s[i] < 0x80 { // ASCII fast path (a following combining mark still joins below)
			if i + 1 >= s.len || s[i + 1] < 0x80 {
				w++
				i++
				continue
			}
		}
		j := next_grapheme(s, i)
		w += grapheme_width(s, i, j)
		i = j
	}
	return w
}

// grapheme_width is the cells of the grapheme s[i..j]: its widest code point.
pub fn grapheme_width(s string, i int, j int) int {
	mut g := 0
	mut k := i
	for k < j {
		r, n := decode(s, k)
		g = if rune_width(u32(r)) > g { rune_width(u32(r)) } else { g }
		k += n
	}
	return g
}

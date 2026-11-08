// NFC normalization for the Latin blocks (enough for Vietnamese and European text).
// Characters outside the tables pass through unchanged.
module text

struct NfcTables {
mut:
	compose map[u64]rune
	ccc     map[rune]int
	base    map[rune]rune // composed letter -> letter without its last mark
}

const tables = build_nfc()

fn hex_values(src string) []u32 {
	return src.fields().map(u32(it.parse_uint(16, 32) or { 0 }))
}

fn build_nfc() NfcTables {
	mut t := NfcTables{}
	pairs := hex_values(nfc_pairs_src)
	for i := 0; i + 2 < pairs.len; i += 3 {
		t.compose[(u64(pairs[i]) << 21) | u64(pairs[i + 1])] = rune(pairs[i + 2])
		t.base[rune(pairs[i + 2])] = rune(pairs[i])
	}
	marks := hex_values(nfc_marks_src)
	for i := 0; i + 1 < marks.len; i += 2 {
		t.ccc[rune(marks[i])] = int(marks[i + 1])
	}
	return t
}

fn ccc(r rune) int {
	return tables.ccc[r] or { 0 }
}

// is_mark reports whether r is a combining mark known to the tables.
pub fn is_mark(r rune) bool {
	return ccc(r) > 0
}

// normalize_nfc reorders combining marks by class and composes them onto their base.
pub fn normalize_nfc(s string) string {
	if s.is_ascii() {
		return s
	}
	runes := s.runes()
	mut out := []rune{cap: runes.len}
	mut i := 0
	for i < runes.len {
		mut base := runes[i]
		i++
		mut marks := []rune{}
		for i < runes.len && ccc(runes[i]) > 0 {
			marks << runes[i]
			i++
		}
		if marks.len == 0 {
			out << base
			continue
		}
		marks.sort_with_compare(fn (a &rune, b &rune) int {
			return ccc(*a) - ccc(*b)
		})
		// Compose each mark that is not blocked by an earlier uncomposed mark of the same class.
		mut rest := []rune{}
		mut last_class := -1
		for m in marks {
			c := ccc(m)
			if composed := tables.compose[(u64(base) << 21) | u64(m)] {
				if last_class < c {
					base = composed
					continue
				}
			}
			rest << m
			last_class = c
		}
		out << base
		out << rest
	}
	return out.string()
}

// fold_diacritics strips accents for search ("Hải Phòng" -> "Hai Phong", "đ" -> "d"), so
// users can type without diacritics. Combining marks are dropped too.
pub fn fold_diacritics(s string) string {
	if s.is_ascii() {
		return s
	}
	mut out := []rune{cap: s.len}
	for r0 in s.runes() {
		mut r := r0
		for {
			b := tables.base[r] or { break }
			r = b
		}
		r = match r {
			`đ` { `d` }
			`Đ` { `D` }
			else { r }
		}

		if ccc(r) == 0 {
			out << r
		}
	}
	return out.string()
}

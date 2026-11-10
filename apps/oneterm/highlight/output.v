// Output-mode scanning: the flat matcher pipeline, first match wins per cell.
module highlight

fn scan_output(chars []rune, mut classes []u8, profile ShellProfile, mut stack []int) {
	if chars.len == 0 {
		return
	}
	quote_scan(chars, mut classes)
	tag_permission_block(chars, mut classes)
	keyword_scan(chars, mut classes)
	structural_scan(chars, mut classes, mut stack)
	ipv4_probe(chars, mut classes)
	path_probe(chars, mut classes, profile, chars.len)
	number_probe(chars, mut classes)
	option_probe(chars, mut classes, profile)
	single_char_scan(chars, mut classes)
}

// quote_scan tags '...' and "..." spans; an unclosed quote runs to the end.
fn quote_scan(chars []rune, mut classes []u8) {
	n := chars.len
	mut i := 0
	for i < n {
		q := chars[i]
		if q != `'` && q != `"` {
			i++
			continue
		}
		start := i
		i++
		for i < n && chars[i] != q {
			i++
		}
		end := if i < n { i + 1 } else { n }
		for j in start .. end {
			classes[j] = u8(Class.quoted)
		}
		i = end
	}
}

// tag_permission_block tags the 10-char `ls -l` prefix, per char.
fn tag_permission_block(chars []rune, mut classes []u8) {
	if chars.len < 10 {
		return
	}
	if chars[0] !in [`b`, `c`, `d`, `l`, `p`, `s`, `-`] {
		return
	}
	for j in 1 .. 10 {
		if chars[j] !in [`r`, `w`, `x`, `s`, `t`, `S`, `T`, `-`] {
			return
		}
	}
	classes[0] = u8(Class.perm_type)
	for j in 1 .. 10 {
		classes[j] = match chars[j] {
			`r` { u8(Class.perm_read) }
			`w` { u8(Class.perm_write) }
			`x` { u8(Class.perm_exec) }
			`s`, `S`, `t`, `T` { u8(Class.perm_special) }
			else { u8(Class.perm_none) }
		}
	}
}

// keyword_scan: leftmost-longest, non-overlapping; a match counts only on word boundaries.
fn keyword_scan(chars []rune, mut classes []u8) {
	n := chars.len
	mut i := 0
	for i < n {
		end, class := global_rules.longest_keyword(chars, i)
		if end < 0 {
			i++
			continue
		}
		before_ok := i == 0 || !is_word_char(chars[i - 1])
		after_ok := end >= n || !is_word_char(chars[end])
		if before_ok && after_ok {
			for j in i .. end {
				if classes[j] == u8(Class.plain) {
					classes[j] = class
				}
			}
		}
		i = end
	}
}

// structural_scan runs MAC, IPv6 and datetime; a match lands only on all-plain cells.
fn structural_scan(chars []rune, mut classes []u8, mut stack []int) {
	mut colons := 0
	mut double_colon := false
	for i, c in chars {
		if c == `:` {
			colons++
			double_colon = double_colon || (i > 0 && chars[i - 1] == `:`)
		}
	}
	if colons >= 5 {
		apply_structural(&global_rules.mac, u8(Class.mac), chars, mut classes, mut stack)
	}
	if double_colon || colons >= 7 {
		apply_structural(&global_rules.ipv6, u8(Class.ip), chars, mut classes, mut stack)
	}
	apply_structural(&global_rules.datetime, u8(Class.date_time), chars, mut classes, mut stack)
}

fn apply_structural(re &Re, class u8, chars []rune, mut classes []u8, mut stack []int) {
	mut from := 0
	for from < chars.len {
		start, end := re.find(chars, from, mut stack)
		if start < 0 {
			return
		}
		from = end
		// IPv6-like strings must not start or end inside an identifier.
		if class == u8(Class.ip) && ((start > 0 && is_word_char(chars[start - 1]))
			|| (end < chars.len && is_word_char(chars[end]))) {
			continue
		}
		mut all_plain := true
		for j in start .. end {
			if classes[j] != u8(Class.plain) {
				all_plain = false
				break
			}
		}
		if all_plain {
			for j in start .. end {
				classes[j] = class
			}
		}
	}
}

// option_probe tags `--flag`, `-x`, `/flag` tokens still plain (e.g. `curl --help`).
fn option_probe(chars []rune, mut classes []u8, profile ShellProfile) {
	n := chars.len
	mut i := 0
	for i < n {
		if classes[i] != u8(Class.plain) || !profile.is_option_prefix(chars[i]) {
			i++
			continue
		}
		if (i > 0 && is_word_char(chars[i - 1]))
			|| (chars[i] == `-` && i + 1 < n && is_digit(chars[i + 1])) {
			i++
			continue
		}
		start := i
		i++
		if i < n && chars[i] == `-` {
			i++
			had_word := i < n && is_word_char(chars[i])
			for i < n && (is_word_char(chars[i]) || chars[i] in [`-`, `=`, `.`]) {
				i++
			}
			if had_word {
				tag_plain(mut classes, start, i, u8(Class.option))
			}
			continue
		}
		if i < n && is_word_char(chars[i]) {
			for i < n && is_word_char(chars[i]) {
				i++
			}
			if i < n && chars[i] == `=` {
				i++
				for i < n && (is_word_char(chars[i]) || chars[i] == `.`) {
					i++
				}
			}
			tag_plain(mut classes, start, i, u8(Class.option))
		}
	}
}

fn tag_plain(mut classes []u8, start int, end int, class u8) {
	for j in start .. end {
		if classes[j] == u8(Class.plain) {
			classes[j] = class
		}
	}
}

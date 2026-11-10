// Hand-written probes: IPv4, paths, numbers, single-char operators and brackets. They tag
// only cells that are still plain.
module highlight

fn ipv4_probe(chars []rune, mut classes []u8) {
	n := chars.len
	mut i := 0
	for i < n {
		if is_digit(chars[i]) && classes[i] == u8(Class.plain) {
			end := try_ipv4(chars, i)
			if end >= 0 {
				for j in i .. end {
					classes[j] = u8(Class.ip)
				}
				i = end
				continue
			}
		}
		i++
	}
}

// try_ipv4 returns the end of dotted octets at `start`, -1 if none.
fn try_ipv4(chars []rune, start int) int {
	n := chars.len
	mut pos := start
	mut octets := 0
	for octets < 4 {
		digit_start := pos
		for pos < n && is_digit(chars[pos]) {
			pos++
		}
		digit_len := pos - digit_start
		if digit_len == 0 || digit_len > 3 {
			return -1
		}
		mut octet := 0
		for j in digit_start .. pos {
			octet = octet * 10 + int(chars[j] - `0`)
		}
		if octet > 255 {
			return -1
		}
		octets++
		if octets < 4 {
			if pos >= n || chars[pos] != `.` {
				return -1
			}
			pos++
		}
	}
	if pos < n && (is_digit(chars[pos]) || chars[pos] == `.`) {
		return -1
	}
	if start > 0 && (is_digit(chars[start - 1]) || chars[start - 1] == `.`) {
		return -1
	}
	return pos
}

// path_probe scans chars[..limit] for paths: from an anchor (`/`, `\`, `~`, a drive), it
// extends back over a segment and forward over path chars. Absolute paths must not follow a
// word char; relative ones need 2+ separators or an extension (not `link/ether`, `3/5`).
fn path_probe(chars []rune, mut classes []u8, profile ShellProfile, limit int) {
	n := limit
	unix := profile.path_is_unix()
	mut i := 0
	for i < n {
		c := chars[i]
		is_anchor := if unix {
			c == `/` || (c == `~` && i + 1 < n && chars[i + 1] == `/`)
		} else {
			c == `/` || c == `\\` || c == `~` || (i + 2 < n && is_ascii_alpha(c)
				&& chars[i + 1] == `:` && (chars[i + 2] == `\\` || chars[i + 2] == `/`))
		}
		if !is_anchor || classes[i] != u8(Class.plain) {
			i++
			continue
		}
		mut start := i
		if c == `/` || c == `\\` {
			mut b := i
			for b > 0 && is_path_segment_char(chars[b - 1]) && classes[b - 1] == u8(Class.plain) {
				b--
			}
			start = b
		}
		mut end := i
		for end < n && (is_alnum(chars[end]) || chars[end] in [`/`, `\\`, `.`, `-`, `_`, `~`, `:`]) {
			if chars[end] == `:` && end != start + 1 {
				break
			}
			end++
		}
		mut sep_count := 0
		for j in start .. end {
			if chars[j] == `/` || chars[j] == `\\` {
				sep_count++
			}
		}
		span_len := end - start
		if sep_count == 0 || span_len < 2 {
			i = end
			continue
		}
		first := chars[start]
		is_absolute := first == `/` || first == `\\` || first == `~`
			|| (span_len > 2 && is_ascii_alpha(first) && chars[start + 1] == `:`)
		should_tag := if is_absolute {
			start == 0 || !is_word_char(chars[start - 1])
		} else {
			sep_count >= 2 || path_has_extension(chars, start, end)
		}
		if should_tag {
			for j in start .. end {
				if classes[j] == u8(Class.plain) {
					classes[j] = u8(Class.path)
				}
			}
		}
		i = end
	}
}

fn is_path_segment_char(c rune) bool {
	return is_alnum(c) || c == `.` || c == `-` || c == `_`
}

fn path_has_extension(chars []rune, start int, end int) bool {
	mut seg := start
	for j in start .. end {
		if chars[j] == `/` || chars[j] == `\\` {
			seg = j + 1
		}
	}
	for j in seg .. end {
		if chars[j] == `.` {
			return j > seg
		}
	}
	return false
}

// number_probe matches integers, decimals, hex, exponents, a trailing `%`, with boundaries.
fn number_probe(chars []rune, mut classes []u8) {
	n := chars.len
	mut i := 0
	for i < n {
		if classes[i] != u8(Class.plain) {
			i++
			continue
		}
		if chars[i] == `0` && i + 1 < n && (chars[i + 1] == `x` || chars[i + 1] == `X`) {
			start := i
			i += 2
			for i < n && is_hexdigit(chars[i]) {
				i++
			}
			if i > start + 2 && (i >= n || !is_word_char(chars[i])) {
				for j in start .. i {
					classes[j] = u8(Class.number)
				}
			}
			continue
		}
		negative := chars[i] == `-` && i + 1 < n && is_digit(chars[i + 1])
			&& (i == 0 || is_number_boundary(chars[i - 1]))
		if !(is_digit(chars[i])
			|| (chars[i] == `.` && i + 1 < n && is_digit(chars[i + 1])) || negative) {
			i++
			continue
		}
		start := i
		if negative {
			i++
		}
		for i < n && is_digit(chars[i]) {
			i++
		}
		if i < n && chars[i] == `.` {
			i++
			for i < n && is_digit(chars[i]) {
				i++
			}
		}
		if i < n && (chars[i] == `e` || chars[i] == `E`) {
			exp_start := i
			i++
			if i < n && (chars[i] == `+` || chars[i] == `-`) {
				i++
			}
			exp_digits := i
			for i < n && is_digit(chars[i]) {
				i++
			}
			if i == exp_digits {
				i = exp_start
			}
		}
		if i < n && chars[i] == `%` {
			i++
		}
		before_ok := start == 0 || is_number_boundary(chars[start - 1])
		after_ok := i >= n || is_number_boundary(chars[i])
		if before_ok && after_ok && i > start {
			for j in start .. i {
				classes[j] = u8(Class.number)
			}
		}
	}
}

fn single_char_scan(chars []rune, mut classes []u8) {
	for i, c in chars {
		if classes[i] != u8(Class.plain) {
			continue
		}
		if c in [`=`, `;`, `|`, `?`, `*`, `$`, `<`, `>`, `&`, `+`, `-`, `:`, `!`, `~`, `^`] {
			classes[i] = u8(Class.operator)
		} else if c in [`(`, `)`, `[`, `]`, `{`, `}`] {
			classes[i] = u8(Class.bracket)
		}
	}
}

fn is_number_boundary(c rune) bool {
	return is_ws_rune(c)
		|| c in [`(`, `)`, `[`, `]`, `{`, `}`, `,`, `;`, `|`, `<`, `>`, `"`, `'`, `+`, `*`, `\\`, `=`, `!`, `?`, `@`, `#`, `$`, `%`, `^`, `&`]
}

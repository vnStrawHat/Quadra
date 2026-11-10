// The single-pass line scanner: one class per rune of a logical line (specs/24).
module highlight

// ScanScratch holds the scanner's working buffers, reused across lines by the caller so a
// steady-state scan allocates nothing.
pub struct ScanScratch {
mut:
	chars []rune
	stack []int
}

// scan_line classifies one logical line: `role` from OSC 133 marks, `input_at` the rune index
// where the typed command begins (-1 unknown, honoured only for prompts).
pub fn scan_line(line string, profile ShellProfile, role RowRole, input_at int) []u8 {
	mut scratch := ScanScratch{}
	mut out := []u8{}
	scan_line_into(line, profile, role, input_at, mut scratch, mut out)
	return out
}

// scan_line_into is scan_line into caller-owned buffers.
pub fn scan_line_into(line string, profile ShellProfile, role RowRole, input_at int, mut scratch ScanScratch, mut out []u8) {
	decode_runes(line, mut scratch.chars)
	scan_runes_into(scratch.chars, profile, role, input_at, mut scratch, mut out)
}

// scan_runes_into is scan_line_into for a line already decoded (the terminal's cells).
pub fn scan_runes_into(chars []rune, profile ShellProfile, role RowRole, input_at int, mut scratch ScanScratch, mut out []u8) {
	n := chars.len
	out.clear()
	if n > 0 {
		unsafe {
			out.grow_len(n)
			vmemset(out.data, 0, n)
		}
	}
	match role {
		.prompt {
			scan_prompt_line(chars, mut out, profile, marked_sign(chars, input_at))
		}
		.command {
			scan_command_mode(chars, mut out, profile, 0)
		}
		.output {
			scan_output(chars, mut out, profile, mut scratch.stack)
		}
		.unmarked {
			sign := prompt_sign(chars, profile, mut scratch.stack)
			if sign >= 0 {
				scan_prompt_line(chars, mut out, profile, sign)
			} else {
				scan_output(chars, mut out, profile, mut scratch.stack)
			}
		}
	}
}

fn decode_runes(s string, mut out []rune) {
	out.clear()
	mut i := 0
	for i < s.len {
		b := s[i]
		if b < 0x80 {
			out << rune(b)
			i++
			continue
		}
		w := utf8_char_len(b)
		if w < 2 || i + w > s.len {
			out << rune(0xFFFD)
			i++
			continue
		}
		mut r := rune(b & (0xFF >> (w + 1)))
		for k in 1 .. w {
			r = (r << 6) | rune(s[i + k] & 0x3F)
		}
		out << r
		i += w
	}
}

// prompt_sign returns the sign's index on an unmarked line, -1 when it is not a prompt.
fn prompt_sign(chars []rune, profile ShellProfile, mut stack []int) int {
	mut end := profile.prompt_regex().match_at(chars, 0, mut stack)
	if end < 0 {
		end = global_rules.prompt_any.match_at(chars, 0, mut stack)
	}
	if end < 0 {
		return -1
	}
	for i := end - 1; i >= 0; i-- {
		if is_prompt_sign(chars[i]) {
			return i
		}
	}
	return -1
}

// marked_sign finds the sign of a marked prompt: the last prompt glyph in the region before
// `input_at`, else the last non-space char there; -1 for a blank region.
fn marked_sign(chars []rune, input_at int) int {
	end := if input_at < 0 || input_at > chars.len { chars.len } else { input_at }
	mut last_ns := -1
	for i := end - 1; i >= 0; i-- {
		if is_prompt_sign(chars[i]) {
			return i
		}
		if last_ns < 0 && !is_ws_rune(chars[i]) {
			last_ns = i
		}
	}
	return last_ns
}

// tint_prompt_sign re-tags a prompt sign with the exit code of the command it launched.
pub fn tint_prompt_sign(mut classes []u8, exit_code int) {
	tint := if exit_code == 0 { u8(Class.success) } else { u8(Class.error) }
	for i in 0 .. classes.len {
		if classes[i] == u8(Class.prompt_sign) {
			classes[i] = tint
		}
	}
}

// windows_prompt_path: where the path starts when the prompt region is a Windows prompt
// (a drive or UNC root, optionally behind `PS `); -1 otherwise.
fn windows_prompt_path(chars []rune, head_len int) int {
	start := if head_len >= 3 && chars[0] == `P` && chars[1] == `S` && chars[2] == ` ` {
		3
	} else {
		0
	}
	if head_len - start >= 3 && is_ascii_alpha(chars[start]) && chars[start + 1] == `:`
		&& (chars[start + 2] == `\\` || chars[start + 2] == `/`) {
		return start
	}
	if head_len - start >= 2 && chars[start] == `\\` && chars[start + 1] == `\\` {
		return start
	}
	return -1
}

// scan_prompt_line tags the sign and the path before it, then the rest as a command.
fn scan_prompt_line(chars []rune, mut classes []u8, profile ShellProfile, sign_in int) {
	n := chars.len
	mut sign := sign_in
	if sign < 0 || sign >= n {
		// The last resort: the first glyph before the first space.
		sign = -1
		for i, c in chars {
			if is_prompt_sign(c) {
				sign = i
				break
			}
			if c == ` ` && i > 0 {
				break
			}
		}
		if sign < 0 {
			return
		}
	}
	classes[sign] = u8(Class.prompt_sign)
	if sign > 0 {
		start := windows_prompt_path(chars, sign)
		if start >= 0 {
			for i in start .. sign {
				classes[i] = u8(Class.path)
			}
		} else {
			path_probe(chars, mut classes, profile, sign)
		}
	}
	mut rest := sign + 1
	for rest < n && chars[rest] == ` ` {
		rest++
	}
	if rest < n {
		scan_command_mode(chars, mut classes, profile, rest)
	}
}

// scan_command_mode tags from `from` to the end: the first token and any after a
// separator is the command, option tokens are options, arguments stay plain.
fn scan_command_mode(chars []rune, mut classes []u8, profile ShellProfile, from int) {
	n := chars.len
	mut i := from
	mut expect_command := true
	for i < n {
		if chars[i] == ` ` {
			i++
			continue
		}
		if chars[i] == `;` || chars[i] == `|` {
			classes[i] = u8(Class.operator)
			if chars[i] == `|` && i + 1 < n && chars[i + 1] == `|` {
				classes[i + 1] = u8(Class.operator)
				i += 2
			} else {
				i++
			}
			expect_command = true
			continue
		}
		if chars[i] == `&` && i + 1 < n && chars[i + 1] == `&` {
			classes[i] = u8(Class.operator)
			classes[i + 1] = u8(Class.operator)
			i += 2
			expect_command = true
			continue
		}
		start := i
		for i < n && chars[i] != ` ` && chars[i] != `;` && chars[i] != `|` {
			i++
		}
		if expect_command {
			for j in start .. i {
				classes[j] = u8(Class.command)
			}
			expect_command = false
		} else if profile.is_option_prefix(chars[start]) {
			for j in start .. i {
				classes[j] = u8(Class.option)
			}
		}
	}
}

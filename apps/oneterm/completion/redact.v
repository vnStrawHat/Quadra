// Secret-value redaction of a command line, before it enters the history. Command names and
// option flags survive; secret values do not (specs/24). `contains_secret` runs again at
// suggestion time, as a second layer.
module completion

import strings

const max_line_len = 4096

// redact returns a form of `line` safe to store and suggest.
pub fn redact(line string) string {
	kept, _ := scrub_tokens(split_ws(line))
	return hygiene(kept.join(' '))
}

// contains_secret tells whether redact would change `line`.
pub fn contains_secret(line string) bool {
	_, changed := scrub_tokens(split_ws(line))
	return changed
}

fn is_flag(tok string) bool {
	return tok.starts_with('-') || tok.starts_with('/')
}

// scrub_tokens returns the kept tokens and whether anything was dropped or rewritten.
fn scrub_tokens(tokens []string) ([]string, bool) {
	mut out := []string{cap: tokens.len}
	mut changed := false
	mut drop_next := false
	command, subcommand := command_and_subcommand(tokens)
	letter := short_secret_flag(command, subcommand)

	for tok in tokens {
		if drop_next {
			// The secret value after a secret flag.
			drop_next = false
			changed = true
			continue
		}
		if is_flag(tok) {
			name := inline_flag_name(tok)
			if name != '' {
				// `--flag=value`, `/FLAG:value`, `-p=value`
				if is_secret_flag(name) || is_command_secret_long_flag(command, name)
					|| is_short_secret_flag(name, letter) {
					out << name
					changed = true
				} else {
					out << tok
				}
			} else if attached := attached_short_secret(tok, letter) {
				// `-pSECRET`, `-uuser:pass`: keep the flag, drop the value.
				out << attached
				changed = true
			} else if is_secret_flag(tok) || is_command_secret_long_flag(command, tok)
				|| is_short_secret_flag(tok, letter) {
				out << tok
				drop_next = true
			} else {
				out << tok
			}
			continue
		}
		if is_assignment(tok) {
			eq := tok.index_u8(`=`)
			if is_secret_key(tok[..eq]) || looks_like_secret(tok[eq + 1..]) {
				changed = true
				continue
			}
			out << tok
			continue
		}
		if stripped := strip_url_userinfo(tok) {
			changed = true
			out << stripped
			continue
		}
		if looks_like_secret(tok) {
			changed = true
			continue
		}
		out << tok
	}
	return out, changed
}

// command_and_subcommand skips leading assignments and privilege wrappers.
fn command_and_subcommand(tokens []string) (string, string) {
	mut i := 0
	for i < tokens.len && (is_assignment(tokens[i]) || tokens[i] == 'sudo' || tokens[i] == 'doas') {
		i++
	}
	if i >= tokens.len {
		return '', ''
	}
	command := command_name(tokens[i])
	for tok in tokens[i + 1..] {
		if !is_flag(tok) {
			return command, tok.to_lower()
		}
	}
	return command, ''
}

// is_short_secret_flag: a single-dash flag whose only letter is the per-command secret flag.
fn is_short_secret_flag(tok string, letter u8) bool {
	return letter != 0 && tok.len == 2 && tok[0] == `-` && tok[1] == letter
}

// attached_short_secret returns the bare flag of `-pSECRET`; long options never attach.
fn attached_short_secret(tok string, letter u8) ?string {
	if letter == 0 || tok.len < 3 || tok[0] != `-` || tok[1] != letter {
		return none
	}
	return tok[..2]
}

// inline_flag_name returns the name of `--flag=value` or `/FLAG:value`, else ''. A colon
// separates only for `/`-flags.
fn inline_flag_name(tok string) string {
	eq := tok.index_u8(`=`)
	if eq >= 0 {
		return tok[..eq]
	}
	if tok.starts_with('/') {
		colon := tok.index_u8(`:`)
		if colon >= 0 {
			return tok[..colon]
		}
	}
	return ''
}

// is_assignment reports `KEY=VALUE` with an env-style KEY.
fn is_assignment(tok string) bool {
	eq := tok.index_u8(`=`)
	if eq <= 0 {
		return false
	}
	for c in tok[..eq] {
		if !(c.is_alnum() || c == `_` || c == `-`) {
			return false
		}
	}
	return true
}

// hygiene strips control chars, truncates on a char boundary and trims.
fn hygiene(s string) string {
	mut b := strings.new_builder(s.len)
	for r in s.runes() {
		if r == `\t` {
			b.write_u8(` `)
		} else if !(r < 0x20 || (r >= 0x7f && r <= 0x9f)) {
			b.write_rune(r)
		}
	}
	mut out := b.str()
	if out.len > max_line_len {
		mut end := max_line_len
		for end > 0 && (out[end] & 0xC0) == 0x80 {
			end--
		}
		out = out[..end]
	}
	return out.trim_space()
}

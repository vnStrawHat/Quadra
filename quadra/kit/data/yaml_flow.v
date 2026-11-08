// Inline YAML values: flow collections ({a: 1, b: [x, y]}) and scalars.
module data

struct Flow {
	s string
mut:
	i int
}

// inline parses the value text after "key:" or "- ".
fn inline(text string) Value {
	t := text.trim_space()
	if t.starts_with('{') || t.starts_with('[') {
		mut f := Flow{
			s: t
		}
		return f.value()
	}
	return plain_scalar(t)
}

fn (mut f Flow) space() {
	for f.i < f.s.len && f.s[f.i] == ` ` {
		f.i++
	}
}

fn (mut f Flow) value() Value {
	f.space()
	if f.i >= f.s.len {
		return scalar(.null, 'null')
	}
	c := f.s[f.i]
	if c == `{` || c == `[` {
		f.i++
		mut keys := []string{}
		mut items := []Value{}
		for {
			f.space()
			if f.i >= f.s.len {
				break
			}
			if f.s[f.i] == `}` || f.s[f.i] == `]` {
				f.i++
				break
			}
			if c == `{` {
				keys << f.word(true)
				f.space()
				if f.i < f.s.len && f.s[f.i] == `:` {
					f.i++
				}
			}
			items << f.value()
			f.space()
			if f.i < f.s.len && f.s[f.i] == `,` {
				f.i++
			}
		}
		return if c == `{` {
			Value{
				kind:  .map
				keys:  keys
				items: items
			}
		} else {
			Value{
				kind:  .list
				items: items
			}
		}
	}
	return plain_scalar(f.word(false))
}

// word reads a quoted or plain scalar up to a flow indicator (and ':' for keys).
fn (mut f Flow) word(key bool) string {
	start := f.i
	if f.i < f.s.len && (f.s[f.i] == `"` || f.s[f.i] == `'`) {
		q := f.s[f.i]
		f.i++
		for f.i < f.s.len && f.s[f.i] != q {
			f.i += if f.s[f.i] == `\\` && q == `"` { 2 } else { 1 }
		}
		f.i = if f.i < f.s.len { f.i + 1 } else { f.s.len }
		w := f.s[start..f.i]
		return if key { unquote(w) } else { w }
	}
	for f.i < f.s.len {
		ch := f.s[f.i]
		if ch in [u8(`,`), `}`, `]`] || (key && ch == `:`)
			|| (ch == `:` && f.i + 1 < f.s.len && f.s[f.i + 1] == ` `) {
			break
		}
		f.i++
	}
	return f.s[start..f.i].trim_space()
}

fn unquote(w string) string {
	if w.len >= 2 && w[0] == `"` && w[w.len - 1] == `"` {
		return w[1..w.len - 1].replace('\\n', '\n').replace('\\"', '"').replace('\\\\', '\\')
	}
	if w.len >= 2 && w[0] == `'` && w[w.len - 1] == `'` {
		return w[1..w.len - 1].replace("''", "'")
	}
	return w
}

fn plain_scalar(t string) Value {
	if t.len >= 2 && (t[0] == `"` || t[0] == `'`) {
		return scalar(.str, unquote(t))
	}
	return match t {
		'', '~', 'null', 'Null', 'NULL' {
			scalar(.null, 'null')
		}
		'true', 'True', 'TRUE', 'false', 'False', 'FALSE' {
			scalar(.boolean, t.to_lower())
		}
		else {
			if is_number(t) {
				scalar(.number, t)
			} else {
				scalar(.str, t)
			}
		}
	}
}

fn is_number(w string) bool {
	if w.len > 2 && w[0] == `0` && (w[1] == `x` || w[1] == `o`) {
		return w[2..].bytes().all(it.is_hex_digit())
	}
	mut digits, mut dots, mut exps := 0, 0, 0
	for i, c in w {
		if c.is_digit() {
			digits++
		} else if c == `.` {
			dots++
		} else if c == `e` || c == `E` {
			exps++
		} else if !(c in [u8(`.`), `_`, `e`, `E`] || ((c == `-` || c == `+`)
			&& (i == 0 || w[i - 1] in [u8(`e`), `E`]))) {
			return false
		}
	}
	return digits > 0 && dots <= 1 && exps <= 1 && (w[0].is_digit() || w[0] in [u8(`-`), `+`, `.`])
}

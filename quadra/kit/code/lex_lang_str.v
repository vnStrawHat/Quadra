// Strings, comments, numbers and variables for the table-driven tokenizer (lex_lang.v).
module code

// String state flags, added to 1000 + the quote byte.
const str_raw = 256 // no escapes
const str_triple = 512 // closes at three quotes
const str_hash = 1024 // a raw string with one hash: r#"..."#

// string_open moves past a string opener at l.i and returns its state (0: none here).
fn (mut l Lex) string_open(sp &LangSpec) int {
	mut k := l.i
	mut raw := false
	for k - l.i < 2 && sp.prefixes.contains_u8(l.byte_at(k)) {
		c := l.s[k]
		raw = raw || c == `r` || c == `R` || c == `@`
		k++
	}
	q := l.byte_at(k)
	if k > l.i && q == `#` && l.byte_at(k + 1) == `"` && raw {
		l.i = k + 2
		return 1000 + int(`"`) + str_raw + str_hash
	}
	if q == 0 || !(sp.quotes.contains_u8(q) || sp.multi_quotes.contains_u8(q)) {
		if k == l.i && q == `[` && sp.long_brackets && l.at(1) == `[` {
			l.i += 2
			return 1000 + int(`[`) + str_raw
		}
		return 0
	}
	if q == `'` && sp.lifetimes && l.byte_at(k + 1) != `\\` && l.byte_at(k + 2) != `'` {
		if k == l.i && is_ident(l.at(1)) { // a lifetime or a label
			a := l.i
			l.i++
			for l.i < l.s.len && is_ident(l.s[l.i]) {
				l.i++
			}
			l.add(a, .attribute)
		}
		return 0
	}
	mut f := int(q)
	if raw || sp.raw_quotes.contains_u8(q) {
		f += str_raw
	}
	if sp.triple && l.byte_at(k + 1) == q && l.byte_at(k + 2) == q {
		l.i = k + 3
		return 1000 + f + str_triple
	}
	l.i = k + 1
	return 1000 + f
}

// close_len is the length of the string's closing quote at l.i (0: not there).
fn (l &Lex) close_len(q u8, f int) int {
	if f & str_triple != 0 {
		return if l.at(0) == q && l.at(1) == q && l.at(2) == q { 3 } else { 0 }
	}
	if f & str_hash != 0 {
		return if l.at(0) == `"` && l.at(1) == `#` { 2 } else { 0 }
	}
	if q == `[` {
		return if l.at(0) == `]` && l.at(1) == `]` { 2 } else { 0 }
	}
	return if l.at(0) == q { 1 } else { 0 }
}

// lang_string scans a string body from a; it returns the state (0 when it closed here, or
// at the line end for a one-line quote).
fn (mut l Lex) lang_string(sp &LangSpec, a int, state int) int {
	f := state - 1000
	q := u8(f & 255)
	raw := f & str_raw != 0
	interp := !raw && sp.interp_in.contains_u8(q)
	mut from := a
	for l.i < l.s.len {
		n := l.close_len(q, f)
		if n > 0 {
			l.i += n
			l.add(from, .str)
			return 0
		}
		c := l.s[l.i]
		if interp && l.interp_at(sp) {
			l.add(from, .str)
			l.interpolate(sp)
			from = l.i
		} else if interp && sp.dollar_var && c == `$` && is_ident(l.at(1)) {
			l.add(from, .str)
			l.variable(sp)
			from = l.i
		} else if !raw && c == `\\` {
			l.add(from, .str)
			e := l.i
			l.i = if l.i + 2 < l.s.len { l.i + 2 } else { l.s.len }
			l.add(e, .escape)
			from = l.i
		} else {
			l.i++
		}
	}
	l.add(from, .str)
	multi := f & (str_triple | str_hash) != 0 || q == `[` || sp.multi_quotes.contains_u8(q)
	return if multi { state } else { 0 }
}

fn (l &Lex) interp_at(sp &LangSpec) bool {
	for op in sp.interps {
		if l.starts_at(op, false) {
			return true
		}
	}
	return false
}

// interpolate marks the opener and its closer as escapes, the expression as a variable.
fn (mut l Lex) interpolate(sp &LangSpec) {
	e := l.i
	mut open := u8(`{`)
	for op in sp.interps {
		if l.starts_at(op, false) {
			open = op[op.len - 1]
			l.i += op.len
			break
		}
	}
	close := if open == `(` { u8(`)`) } else { u8(`}`) }
	l.add(e, .escape)
	inner := l.i
	mut depth := 1
	for l.i < l.s.len {
		if l.s[l.i] == open {
			depth++
		} else if l.s[l.i] == close {
			depth--
			if depth == 0 {
				break
			}
		}
		l.i++
	}
	l.add(inner, .variable)
	if l.i < l.s.len {
		l.i++
		l.add(l.i - 1, .escape)
	}
}

// variable scans $name, ${...}, $(...), $1, $@ or %name%, %1; false if none is here.
fn (mut l Lex) variable(sp &LangSpec) bool {
	a := l.i
	c := l.s[l.i]
	n := l.at(1)
	if c == `%` {
		if n == `%` || n.is_digit() || n == `~` {
			l.i += 2
		} else if is_ident(n) {
			l.to_byte(a, `%`, .variable)
			return true
		} else {
			return false
		}
		for l.i < l.s.len && is_ident(l.s[l.i]) {
			l.i++
		}
	} else if n == `{` || (n == `(` && sp.label_colon) {
		l.to_byte(a, if n == `{` { u8(`}`) } else { u8(`)`) }, .variable)
		return true
	} else if is_ident(n) {
		l.i++
		for l.i < l.s.len && is_ident(l.s[l.i]) {
			l.i++
		}
	} else if n != 0 && '@#?*!\$-_'.contains_u8(n) {
		l.i += 2
	} else {
		return false
	}
	l.add(a, .variable)
	return true
}

// at_word colors @name (and @a.b): an attribute, a variable or a keyword by the row.
fn (mut l Lex) at_word(sp &LangSpec) {
	a := l.i
	l.i++
	if l.at(0) == `@` {
		l.i++
	}
	for l.i < l.s.len && (is_ident(l.s[l.i]) || (sp.at_attr && l.s[l.i] == `.`)
		|| (sp.at_kw && l.s[l.i] == `-`)) {
		l.i++
	}
	l.add(a, if sp.at_attr {
		TokenKind.attribute
	} else if sp.at_var {
		TokenKind.variable
	} else {
		TokenKind.keyword
	})
}

// lang_number scans 0x1F, 0b101, 1_000, 1.5e-3, 10px, 2u8.
fn (mut l Lex) lang_number() {
	a := l.i
	hex := l.at(0) == `0` && (l.at(1) | 0x20) == `x`
	for l.i < l.s.len {
		c := l.s[l.i]
		if is_ident(c) || (c == `.` && l.at(1).is_digit()) {
			l.i++
		} else if (c == `+` || c == `-`) && !hex && (l.s[l.i - 1] | 0x20) == `e`
			&& l.at(1).is_digit() {
			l.i++
		} else {
			break
		}
	}
	l.add(a, .number)
}

// lang_block scans a block comment from a; it returns the depth left open.
fn (mut l Lex) lang_block(sp &LangSpec, a int, depth int) int {
	mut d := depth
	for l.i < l.s.len && d > 0 {
		if sp.nest && l.starts_at(sp.block_open, false) {
			d++
			l.i += sp.block_open.len
		} else if l.starts_at(sp.block_close, false) {
			d--
			l.i += sp.block_close.len
		} else {
			l.i++
		}
	}
	l.add(a, .comment)
	return d
}

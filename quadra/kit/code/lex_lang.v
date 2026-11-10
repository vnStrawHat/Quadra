// The table-driven tokenizer (specs/18). States: 0 code, 1..999 block comment depth,
// 1000 + quote + str_* flags an open string; markup has its own (lex_markup.v).
module code

pub fn (t LangTokenizer) tokenize(line string, state int) ([]Token, int) {
	if t.spec.mode == .diff {
		return diff_line(line), 0
	}
	mut l := Lex{
		s:   line
		out: []Token{cap: line.len / 3 + 2}
	}
	if t.spec.mode == .markup {
		st := l.markup(state)
		return l.out, st
	}
	sp := t.spec
	mut st := state
	if st >= 1000 {
		st = l.lang_string(sp, 0, st)
	} else if st > 0 {
		st = l.lang_block(sp, 0, st)
	}
	sel := sp.selectors && is_selector_line(line)
	for st == 0 && l.i < line.len {
		st = l.lang_step(sp, sel)
	}
	return l.out, st
}

// lang_step scans one token at l.i and returns the state after it.
fn (mut l Lex) lang_step(sp &LangSpec, sel bool) int {
	c := l.s[l.i]
	a := l.i
	if c == ` ` || c == `\t` {
		l.i++
		return 0
	}
	if l.starts_at(sp.block_open, false) {
		l.i += sp.block_open.len
		return l.lang_block(sp, a, 1)
	}
	if l.line_comment(sp) || (sp.colon_label && c == `:` && l.only_space_before(a)) {
		l.i = l.s.len
		l.add(a, if c == `:` && l.at(1) != `:` { TokenKind.function } else { TokenKind.comment })
		return 0
	}
	if c == `#` && sp.preproc && l.only_space_before(a) {
		l.preproc()
		return 0
	}
	if c == `#` && sp.hash_attr && (l.at(1) == `[` || (l.at(1) == `!` && l.at(2) == `[`)) {
		l.to_byte(a, `]`, .attribute)
		return 0
	}
	if c == `[` && sp.sections && l.only_space_before(a) {
		end := l.s.last_index_u8(`]`)
		l.i = if end > a { end + 1 } else { l.s.len }
		l.add(a, .tag)
		return 0
	}
	st := l.string_open(sp)
	if st > 0 {
		n := l.out.len
		end := l.lang_string(sp, a, st)
		if end == 0 && sp.key_colon && l.byte_at(l.next_nonspace(l.i)) == `:` {
			l.out.trim(n)
			l.add(a, .property)
		}
		return end
	}
	if (c == `$` && sp.dollar_var) || (c == `%` && sp.percent_var) {
		if l.variable(sp) {
			return 0
		}
	}
	if c == `@` && is_ident(l.at(1)) && (sp.at_attr || sp.at_var || sp.at_kw) {
		l.at_word(sp)
		return 0
	}
	if sp.selectors && (c == `#` || c == `:`) && (is_ident(l.at(1)) || (c == `:` && l.at(1) == `:`)) {
		l.css_mark(sel)
		return 0
	}
	if c.is_digit() || (c == `.` && l.at(1).is_digit() && !is_ident(l.byte_at(a - 1))) {
		l.lang_number()
		return 0
	}
	if l.ident_start(sp) {
		l.lang_ident(sp, sel)
		return 0
	}
	l.i++
	l.add(a, if c in [u8(`(`), `)`, `[`, `]`, `{`, `}`, `,`, `;`, `.`] {
		TokenKind.punctuation
	} else {
		TokenKind.operator
	})
	return 0
}

fn (l &Lex) line_comment(sp &LangSpec) bool {
	for m in sp.line_comments {
		word := m[0].is_letter()
		if !l.starts_at(m, word) {
			continue
		}
		if word {
			nx := l.at(m.len)
			if !l.only_space_before(l.i) || (nx != 0 && nx != ` ` && nx != `\t`) {
				continue
			}
		} else if sp.comment_at_word && l.i > 0 && l.s[l.i - 1] != ` ` && l.s[l.i - 1] != `\t` {
			continue
		}
		return true
	}
	return false
}

fn (l &Lex) ident_start(sp &LangSpec) bool {
	c := l.s[l.i]
	if is_ident(c) || (c == `$` && !sp.dollar_var) {
		return true
	}
	prev := l.byte_at(l.i - 1)
	return sp.ident_dash && c == `-` && l.at(1).is_letter()
		&& (prev == 0 || prev == ` ` || prev == `(`)
}

// lang_ident scans a word and colors it by the sets, then by what follows it.
fn (mut l Lex) lang_ident(sp &LangSpec, sel bool) {
	a := l.i
	l.i++
	for l.i < l.s.len {
		c := l.s[l.i]
		if is_ident(c) || (c == `$` && !sp.dollar_var)
			|| (c == `-` && sp.ident_dash && is_ident(l.at(1))) {
			l.i++
		} else {
			break
		}
	}
	b := l.i
	k := l.next_nonspace(b)
	nc := l.byte_at(k)
	n2 := l.byte_at(k + 1)
	call := nc == `(`
	dot := l.byte_at(a - 1) == `.`
	lead := if dot { a - 1 } else { a }
	mut kind := TokenKind.invalid // none
	if sel {
		kind = if dot { TokenKind.typ } else { TokenKind.tag }
	} else if dot && !sp.no_dot {
		kind = if call { TokenKind.function } else { TokenKind.property }
	} else if l.word_in(sp.keywords, a, b, sp.fold) {
		kind = .keyword
	} else if l.word_in(sp.control, a, b, sp.fold) {
		kind = .control
	} else if l.word_in(sp.constants, a, b, sp.fold) {
		kind = .constant
	} else if l.word_in(sp.types, a, b, sp.fold) {
		kind = .typ
	} else if l.word_in(sp.builtins, a, b, sp.fold) {
		kind = .function
	} else if sp.label_colon && nc == `:` && n2 != `=` && l.only_space_before(lead) {
		kind = .function
	} else if sp.key_colon && nc == `:` && n2 != `:` {
		kind = .property
	} else if sp.key_equals && l.only_space_before(a) && ((nc == `=` && n2 != `=`)
		|| (n2 == `=` && nc in [u8(`:`), `?`, `+`])) {
		kind = .property
	} else if sp.macro_bang && k == b && nc == `!` && n2 != `=` {
		l.i++
		kind = .function
	} else if call || (sp.ident_dash && l.s[a].is_capital() && l.has_dash(a, b)) {
		kind = .function // a call, or a cmdlet (Get-Item)
	} else if l.s[a] == `-` && sp.ident_dash {
		kind = .attribute
	} else if sp.cap_type && l.s[a].is_capital() {
		kind = if l.all_caps(a, b) { TokenKind.constant } else { TokenKind.typ }
	} else {
		return
	}
	l.add(a, kind)
}

fn (l &Lex) has_dash(a int, b int) bool {
	for k in a .. b {
		if l.s[k] == `-` {
			return true
		}
	}
	return false
}

fn (l &Lex) all_caps(a int, b int) bool {
	if b - a < 2 {
		return false
	}
	for k in a .. b {
		if l.s[k] >= `a` && l.s[k] <= `z` {
			return false
		}
	}
	return true
}

// preproc colors a # directive; an include's <path> is a string.
fn (mut l Lex) preproc() {
	a := l.i
	l.i++
	l.skip_space()
	for l.i < l.s.len && is_ident(l.s[l.i]) {
		l.i++
	}
	l.add(a, .keyword)
	l.skip_space()
	if l.at(0) == `<` {
		l.to_byte(l.i, `>`, .str)
	}
}

// to_byte colors a..the next `close` (included, or the line end) as kind.
fn (mut l Lex) to_byte(a int, close u8, kind TokenKind) {
	l.i = a + 1
	for l.i < l.s.len && l.s[l.i] != close {
		l.i++
	}
	l.i = if l.i < l.s.len { l.i + 1 } else { l.s.len }
	l.add(a, kind)
}

fn is_selector_line(line string) bool {
	t := line.trim_right(' \t')
	return line.contains_u8(`{`) || (t.ends_with(',') && !t.contains_u8(`;`))
}

// css_mark colors #id and :pseudo in selectors, and #hex colors in declarations.
fn (mut l Lex) css_mark(sel bool) {
	a := l.i
	l.i++
	if l.at(0) == `:` {
		l.i++
	}
	for l.i < l.s.len && (is_ident(l.s[l.i]) || l.s[l.i] == `-`) {
		l.i++
	}
	kind := if !sel && l.s[a] == `#` {
		TokenKind.number
	} else if l.s[a] == `#` {
		TokenKind.constant
	} else if sel {
		TokenKind.keyword
	} else {
		TokenKind.invalid
	}
	if kind == .invalid { // a value after a colon: the colon alone
		l.i = a + 1
		l.add(a, .operator)
		return
	}
	l.add(a, kind)
}

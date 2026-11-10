// The V tokenizer. States: 0 code, 1..999 block comment depth, 1000 + quote (+256 raw) an
// open string.
module code

pub struct VTokenizer {}

const v_keywords = ['fn', 'struct', 'enum', 'interface', 'type', 'union', 'pub', 'mut', 'const',
	'module', 'import', 'as', 'in', 'is', 'sizeof', 'typeof', 'isreftype', '__global', 'shared',
	'lock', 'rlock', 'atomic', 'static', 'volatile', 'unsafe', 'go', 'spawn', 'asm', 'chan', 'select',
	'assert', '__offsetof']
const v_control = ['if', 'else', 'for', 'match', 'return', 'break', 'continue', 'goto', 'defer',
	'or']
const v_constants = ['true', 'false', 'none', 'nil']
const v_types = ['int', 'i8', 'i16', 'i32', 'i64', 'u8', 'u16', 'u32', 'u64', 'f32', 'f64', 'bool',
	'string', 'rune', 'byte', 'voidptr', 'byteptr', 'charptr', 'isize', 'usize', 'char', 'any',
	'map', 'thread']

pub fn (t VTokenizer) tokenize(line string, state int) ([]Token, int) {
	mut l := Lex{
		s:   line
		out: []Token{cap: line.len / 3 + 2}
	}
	mut st := state
	if st >= 1000 {
		st = l.v_string(0, st)
	} else if st > 0 {
		st = l.block_comment(0, st)
	}
	for st == 0 && l.i < line.len {
		c := line[l.i]
		a := l.i
		if c == ` ` || c == `\t` {
			l.i++
		} else if c == `/` && l.at(1) == `/` {
			l.i = line.len
			l.add(a, .comment)
		} else if c == `/` && l.at(1) == `*` {
			l.i += 2
			st = l.block_comment(a, 1)
		} else if c == `'` || c == `"` {
			l.i++
			st = l.v_string(a, 1000 + int(c))
		} else if c == `r` && (l.at(1) == `'` || l.at(1) == `"`) {
			l.i += 2
			st = l.v_string(a, 1000 + 256 + int(line[a + 1]))
		} else if c == `\`` {
			l.v_rune()
		} else if c == `@` {
			l.v_at()
		} else if c.is_digit() {
			l.v_number()
		} else if is_ident(c) {
			l.v_ident()
		} else {
			l.i++
			l.add(a, if c in [u8(`(`), `)`, `[`, `]`, `{`, `}`, `,`, `;`, `.`] {
				TokenKind.punctuation
			} else {
				TokenKind.operator
			})
		}
	}
	return l.out, st
}

// block_comment scans a /* */ comment (nesting) from a; it returns the depth left open.
fn (mut l Lex) block_comment(a int, depth int) int {
	mut d := depth
	for l.i < l.s.len && d > 0 {
		if l.at(0) == `/` && l.at(1) == `*` {
			d++
			l.i += 2
		} else if l.at(0) == `*` && l.at(1) == `/` {
			d--
			l.i += 2
		} else {
			l.i++
		}
	}
	l.add(a, .comment)
	return d
}

// v_string scans a string body from a until its quote; it returns the state (0 if closed).
fn (mut l Lex) v_string(a int, state int) int {
	quote := u8((state - 1000) & 255)
	raw := state - 1000 >= 256
	mut from := a
	for l.i < l.s.len {
		c := l.s[l.i]
		if c == quote {
			l.i++
			l.add(from, .str)
			return 0
		}
		if !raw && c == `\\` {
			l.add(from, .str)
			e := l.i
			l.i = if l.i + 2 < l.s.len { l.i + 2 } else { l.s.len }
			l.add(e, .escape)
			from = l.i
		} else if !raw && c == `$` && l.at(1) == `{` {
			l.add(from, .str)
			l.interpolation()
			from = l.i
		} else {
			l.i++
		}
	}
	l.add(from, .str)
	return state
}

// interpolation marks ${ and } as escapes and the expression between as a variable.
fn (mut l Lex) interpolation() {
	e := l.i
	l.i += 2
	l.add(e, .escape)
	inner := l.i
	mut depth := 1
	for l.i < l.s.len {
		if l.s[l.i] == `{` {
			depth++
		} else if l.s[l.i] == `}` {
			depth--
			if depth == 0 {
				break
			}
		}
		l.i++
	}
	l.add(inner, .variable)
	if l.i < l.s.len {
		close := l.i
		l.i++
		l.add(close, .escape)
	}
}

fn (mut l Lex) v_rune() {
	a := l.i
	l.i++
	for l.i < l.s.len && l.s[l.i] != `\`` {
		l.i += if l.s[l.i] == `\\` { 2 } else { 1 }
	}
	l.i = if l.i < l.s.len { l.i + 1 } else { l.s.len }
	l.add(a, .str)
}

// v_at scans an attribute @[...] or a compile-time name @FILE.
fn (mut l Lex) v_at() {
	a := l.i
	l.i++
	if l.at(0) == `[` {
		for l.i < l.s.len && l.s[l.i] != `]` {
			l.i++
		}
		l.i = if l.i < l.s.len { l.i + 1 } else { l.s.len }
		l.add(a, .attribute)
		return
	}
	for l.i < l.s.len && is_ident(l.s[l.i]) {
		l.i++
	}
	l.add(a, .constant)
}

fn (mut l Lex) v_number() {
	a := l.i
	for l.i < l.s.len {
		c := l.s[l.i]
		if is_ident(c) || (c == `.` && l.at(1).is_digit()) {
			l.i++
		} else {
			break
		}
	}
	l.add(a, .number)
}

fn (mut l Lex) v_ident() {
	a := l.i
	for l.i < l.s.len && is_ident(l.s[l.i]) {
		l.i++
	}
	b := l.i
	mut k := l.i
	for k < l.s.len && l.s[k] == ` ` {
		k++
	}
	call := k < l.s.len && l.s[k] == `(`
	after_dot := a > 0 && l.s[a - 1] == `.`
	kind := if after_dot {
		if call { TokenKind.function } else { TokenKind.property }
	} else if l.word_in(v_keywords, a, b, false) {
		TokenKind.keyword
	} else if l.word_in(v_control, a, b, false) {
		TokenKind.control
	} else if l.word_in(v_constants, a, b, false) {
		TokenKind.constant
	} else if l.word_in(v_types, a, b, false) || l.s[a].is_capital() {
		TokenKind.typ
	} else if call {
		TokenKind.function
	} else {
		return
	}
	l.add(a, kind)
}

// The YAML tokenizer. States: 0 normal, n > 0 inside a block scalar whose parent line is
// indented n - 1 (it lasts while lines are blank or indented deeper).
module code

pub struct YamlTokenizer {}

const yaml_constants = ['true', 'false', 'True', 'False', 'TRUE', 'FALSE', 'yes', 'no', 'on', 'off',
	'null', 'Null', 'NULL', '~']

pub fn (t YamlTokenizer) tokenize(line string, state int) ([]Token, int) {
	mut l := Lex{
		s: line
	}
	l.skip_space()
	indent := l.i
	if state > 0 {
		if l.i == line.len || indent > state - 1 {
			l.i = line.len
			l.add(indent, .str)
			return l.out, state
		}
	}
	if line.starts_with('%') {
		l.i = line.len
		l.add(0, .keyword)
		return l.out, 0
	}
	if (line.starts_with('---') || line.starts_with('...')) && (line.len == 3 || line[3] == ` `) {
		l.i = 3
		l.add(0, .keyword)
		l.skip_space()
	}
	for l.at(0) == `-` && (l.at(1) == ` ` || l.i + 1 == line.len) {
		l.i++
		l.add(l.i - 1, .punctuation)
		l.skip_space()
	}
	st := l.yaml_rest(l.i) // a block scalar is relative to the key's column
	return l.out, st
}

// yaml_rest scans keys and values to the end of the line.
fn (mut l Lex) yaml_rest(indent int) int {
	mut flow := 0
	for l.i < l.s.len {
		c := l.s[l.i]
		a := l.i
		if c == ` ` || c == `\t` {
			l.i++
		} else if c == `#` && (a == 0 || l.s[a - 1] == ` ` || l.s[a - 1] == `\t`) {
			l.i = l.s.len
			l.add(a, .comment)
		} else if c == `"` || c == `'` {
			l.yaml_quoted(c)
			l.key_or(a, .str)
		} else if c == `&` || c == `*` {
			l.yaml_word()
			l.add(a, .attribute)
		} else if c == `!` {
			l.yaml_word()
			l.add(a, .tag)
		} else if (c == `|` || c == `>`) && flow == 0 && l.block_header() {
			l.add(a, .punctuation)
			return indent + 1
		} else if c in [u8(`{`), `}`, `[`, `]`, `,`] {
			flow += if c == `{` || c == `[` {
				1
			} else if c == `,` {
				0
			} else {
				-1
			}
			l.i++
			l.add(a, .punctuation)
		} else if c == `:` && (l.at(1) == ` ` || l.at(1) == 0) {
			l.i++
			l.add(a, .punctuation)
		} else {
			l.yaml_plain(flow > 0)
			mut e := l.i
			for e > a && l.s[e - 1] == ` ` {
				e--
			}
			word := l.s[a..e]
			kind := if word in yaml_constants {
				TokenKind.constant
			} else if yaml_number(word) {
				TokenKind.number
			} else {
				TokenKind.str
			}
			saved := l.i
			l.i = e
			l.key_or(a, kind)
			l.i = saved
		}
	}
	return 0
}

// key_or emits a..l.i as a property when a colon follows, else as kind.
fn (mut l Lex) key_or(a int, kind TokenKind) {
	mut k := l.i
	for k < l.s.len && l.s[k] == ` ` {
		k++
	}
	is_key := k < l.s.len && l.s[k] == `:` && (k + 1 == l.s.len || l.s[k + 1] == ` `)
	l.add(a, if is_key { TokenKind.property } else { kind })
}

fn (mut l Lex) yaml_quoted(q u8) {
	l.i++
	for l.i < l.s.len {
		c := l.s[l.i]
		if q == `"` && c == `\\` {
			l.i += 2
			continue
		}
		l.i++
		if c == q {
			if q == `'` && l.at(0) == `'` {
				l.i++ // '' is an escaped quote
				continue
			}
			break
		}
	}
	if l.i > l.s.len {
		l.i = l.s.len
	}
}

fn (mut l Lex) yaml_word() {
	l.i++
	for l.i < l.s.len && l.s[l.i] !in [u8(` `), `,`, `]`, `}`] {
		l.i++
	}
}

// yaml_plain scans a plain scalar: up to ': ', ' #', the end, or a flow indicator in flow.
fn (mut l Lex) yaml_plain(in_flow bool) {
	for l.i < l.s.len {
		c := l.s[l.i]
		if c == `:` && (l.at(1) == ` ` || l.at(1) == 0 || (in_flow && l.at(1) in [u8(`,`), `}`])) {
			return
		}
		if c == `#` && l.i > 0 && l.s[l.i - 1] == ` ` {
			return
		}
		if in_flow && c in [u8(`,`), `[`, `]`, `{`, `}`] {
			return
		}
		l.i++
	}
}

// block_header accepts | or > with optional indicators and nothing else but a comment.
fn (mut l Lex) block_header() bool {
	mut k := l.i + 1
	for k < l.s.len && (l.s[k] in [u8(`+`), `-`] || l.s[k].is_digit()) {
		k++
	}
	mut e := k
	for e < l.s.len && l.s[e] == ` ` {
		e++
	}
	if e < l.s.len && l.s[e] != `#` {
		return false
	}
	l.i = k
	return true
}

fn yaml_number(w string) bool {
	if w in ['.inf', '-.inf', '+.inf', '.nan', '.NaN', '.Inf'] {
		return true
	}
	if w.len > 2 && w[0] == `0` && (w[1] == `x` || w[1] == `o`) {
		return w[2..].bytes().all(it.is_hex_digit())
	}
	mut digits := 0
	for i, c in w {
		if c.is_digit() {
			digits++
		} else if !(c in [u8(`.`), `_`, `e`, `E`] || ((c == `-` || c == `+`)
			&& (i == 0 || w[i - 1] in [u8(`e`), `E`]))) {
			return false
		}
	}
	return digits > 0 && (w[0].is_digit() || w[0] in [u8(`-`), `+`, `.`])
}

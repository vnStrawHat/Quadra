// Markup and diff modes of the table-driven tokenizer. Markup states: 0 text, 1 inside
// <!-- -->, 2 inside a tag (attributes may run across lines).
module code

// ponytail: <script> and <style> bodies stay plain text; tokenize them with the js and css
// rows if embedded code shows up in real documents.
fn (mut l Lex) markup(state int) int {
	mut st := state
	for l.i < l.s.len {
		a := l.i
		c := l.s[l.i]
		if st == 1 {
			st = l.markup_comment(a)
		} else if st == 2 {
			st = l.tag_attrs()
		} else if l.starts_at('<!--', false) {
			l.i += 4
			st = l.markup_comment(a)
		} else if c == `<` && (l.at(1).is_letter() || l.at(1) in [u8(`/`), `!`, `?`]) {
			l.i += if l.at(1).is_letter() { 1 } else { 2 }
			l.add(a, .punctuation)
			b := l.i
			for l.i < l.s.len && (is_ident(l.s[l.i]) || l.s[l.i] in [u8(`-`), `:`, `.`]) {
				l.i++
			}
			l.add(b, if l.byte_at(a + 1) in [u8(`!`), `?`] {
				TokenKind.keyword
			} else {
				TokenKind.tag
			})
			st = 2
		} else if c == `&` && (is_ident(l.at(1)) || l.at(1) == `#`) {
			l.i++
			for l.i < l.s.len && l.i - a < 12 && (is_ident(l.s[l.i]) || l.s[l.i] == `#`) {
				l.i++
			}
			if l.at(0) == `;` {
				l.i++
				l.add(a, .escape)
			}
		} else {
			l.i++
		}
	}
	return st
}

fn (mut l Lex) markup_comment(a int) int {
	for l.i < l.s.len {
		if l.starts_at('-->', false) {
			l.i += 3
			l.add(a, .comment)
			return 0
		}
		l.i++
	}
	l.add(a, .comment)
	return 1
}

// tag_attrs scans attributes up to the tag's end: 0 when it closed, 2 when it goes on.
fn (mut l Lex) tag_attrs() int {
	for l.i < l.s.len {
		a := l.i
		c := l.s[l.i]
		if c == `>` || ((c == `/` || c == `?`) && l.at(1) == `>`) {
			l.i += if c == `>` { 1 } else { 2 }
			l.add(a, .punctuation)
			return 0
		}
		if c == `"` || c == `'` {
			l.to_byte(a, c, .str)
		} else if is_ident(c) {
			for l.i < l.s.len && (is_ident(l.s[l.i]) || l.s[l.i] in [u8(`-`), `:`, `.`]) {
				l.i++
			}
			l.add(a, .attribute)
		} else if c == `=` {
			l.i++
			l.add(a, .operator)
		} else {
			l.i++
		}
	}
	return 2
}

// diff_line colors a whole line: headers keyword, hunks escape, added str, removed variable.
fn diff_line(line string) []Token {
	kind := if line.starts_with('diff ') || line.starts_with('index ') || line.starts_with('--- ')
		|| line.starts_with('+++ ') {
		TokenKind.keyword
	} else if line.starts_with('@@') {
		TokenKind.escape
	} else if line.starts_with('+') || line.starts_with('>') {
		TokenKind.str
	} else if line.starts_with('-') || line.starts_with('<') {
		TokenKind.variable
	} else {
		return []Token{}
	}
	return [Token{0, line.len, kind}]
}

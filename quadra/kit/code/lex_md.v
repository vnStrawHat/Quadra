// The Markdown tokenizer. States: 0 normal, 1 inside a ``` fence, 2 inside a ~~~ fence,
// 3 inside $$ display math. Headings are tags, strong text numbers, emphasis keywords,
// code strings, link text functions and urls comments, math escapes, markers operators.
module code

pub struct MarkdownTokenizer {}

pub fn (t MarkdownTokenizer) tokenize(line string, state int) ([]Token, int) {
	mut l := Lex{
		s: line
	}
	l.skip_space()
	rest := line[l.i..]
	fence := if rest.starts_with('```') {
		1
	} else if rest.starts_with('~~~') {
		2
	} else {
		0
	}
	if state in [1, 2] {
		l.i = line.len
		if fence == state {
			l.add(0, .punctuation)
			return l.out, 0
		}
		l.add(0, .str)
		return l.out, state
	}
	if state == 3 || rest.starts_with('$$') {
		l.i = line.len
		l.add(0, .escape)
		closes := if state == 3 {
			rest.ends_with('$$')
		} else {
			rest.len > 2 && rest.ends_with('$$')
		}
		return l.out, if closes {
			0
		} else {
			3
		}
	}
	if fence > 0 {
		l.i += 3
		l.add(l.i - 3, .punctuation)
		start := l.i
		l.i = line.len
		l.add(start, .typ)
		return l.out, fence
	}
	if rest.starts_with('#') {
		l.i = line.len
		l.add(0, .tag)
		return l.out, 0
	}
	if rest.len >= 3 && rest.replace(' ', '').len >= 3 && rest.replace(' ', '').trim('-*_').len == 0 {
		l.i = line.len
		l.add(0, .punctuation)
		return l.out, 0
	}
	for l.at(0) == `>` {
		l.i++
		l.add(l.i - 1, .comment)
		l.skip_space()
	}
	l.list_marker()
	l.md_inline()
	return l.out, 0
}

// list_marker colors "- ", "1. " and a task box "[ ] ".
fn (mut l Lex) list_marker() {
	a := l.i
	if l.at(0) in [`-`, `*`, `+`] && l.at(1) == ` ` {
		l.i++
	} else {
		for l.at(0).is_digit() {
			l.i++
		}
		if l.i == a || l.at(0) !in [`.`, `)`] || l.at(1) != ` ` {
			l.i = a
			return
		}
		l.i++
	}
	l.add(a, .operator)
	l.skip_space()
	if l.at(0) == `[` && l.at(2) == `]` && l.at(1) in [` `, `x`, `X`] {
		l.i += 3
		l.add(l.i - 3, .operator)
	}
}

// md_inline colors the inline markup of the rest of the line.
fn (mut l Lex) md_inline() {
	s := l.s
	for l.i < s.len {
		c := s[l.i]
		a := l.i
		if c == `\\` {
			l.i += 2
			continue
		}
		if c == `\`` {
			l.span_to(a, '`', .str)
			continue
		}
		if c == `$` && l.at(1) != ` ` && l.at(1) != 0 {
			l.span_to(a, '$', .escape)
			continue
		}
		if (c == `*` || c == `_`) && l.at(1) == c {
			l.span_to(a + 1, c.ascii_str().repeat(2), .number)
			continue
		}
		if (c == `*` || (c == `_` && (a == 0 || !is_ident(s[a - 1])))) && l.at(1) != ` `
			&& l.at(1) != 0 {
			l.span_to(a, c.ascii_str(), .keyword)
			continue
		}
		if c == `[` || (c == `!` && l.at(1) == `[`) {
			close := s.index_after(']', a + 1) or { -1 }
			if close > 0 && close + 1 < s.len && s[close + 1] == `(` {
				end := s.index_after(')', close + 2) or { -1 }
				if end > 0 {
					l.i = close + 1
					l.add(a, .function)
					l.i = end + 1
					l.add(close + 1, .comment)
					continue
				}
			}
		}
		if c == `<`
			|| (c == `h` && (s[a..].starts_with('http://') || s[a..].starts_with('https://'))) {
			mut e := a + 1
			for e < s.len && s[e] != ` ` && s[e] != `>` {
				e++
			}
			if s[a..e].contains('://') {
				l.i = if e < s.len && s[e] == `>` { e + 1 } else { e }
				l.add(a, .comment)
				continue
			}
		}
		l.i++
	}
}

// span_to colors from a through the closing mark (as kind) when the line has one;
// otherwise it moves past the opening mark only. from is where the closing search starts.
fn (mut l Lex) span_to(from int, mark string, kind TokenKind) {
	a := l.i
	end := l.s.index_after(mark, from + mark.len) or { -1 }
	if end < 0 {
		l.i = a + mark.len
		return
	}
	l.i = end + mark.len
	l.add(a, kind)
}

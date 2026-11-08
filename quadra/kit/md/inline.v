// Markdown inlines: **bold**, *italic* (or _italic_ between words), ~~strikethrough~~,
// `code`, $math$, [text](url), ![alt](src), <url>, bare http(s) urls, backslash escapes.
module md

pub struct Span {
pub:
	text   string // the text; the TeX source for math; the alt text for an image
	bold   bool
	italic bool
	strike bool
	code   bool
	math   bool
	link   string
	image  string // an image's source
}

struct Inline {
mut:
	out    []Span
	buf    []u8
	bold   bool
	italic bool
	strike bool
	link   string
}

fn (mut m Inline) emit() {
	if m.buf.len > 0 {
		m.out << m.span(m.buf.bytestr())
		m.buf.clear()
	}
}

// span is text in the current style.
fn (m &Inline) span(s string) Span {
	return Span{
		text:   s
		bold:   m.bold
		italic: m.italic
		strike: m.strike
		link:   m.link
	}
}

fn word_char(c u8) bool {
	return c.is_letter() || c.is_digit() || c >= 0x80
}

// math_end finds the `$` closing inline math opened at i: the opening one is followed by a
// non-space, the closing one preceded by a non-space and not followed by a digit (so
// "$5 and $10" stays text).
fn math_end(s string, i int) int {
	if i + 1 >= s.len || s[i + 1] in [` `, `$`] {
		return -1
	}
	mut j := i + 1
	for j < s.len {
		if s[j] == `\\` {
			j += 2
			continue
		}
		if s[j] == `$` {
			if s[j - 1] != ` ` && (j + 1 >= s.len || !s[j + 1].is_digit()) {
				return j
			}
			return -1
		}
		j++
	}
	return -1
}

pub fn parse_inline(s string) []Span {
	mut m := Inline{}
	mut i := 0
	for i < s.len {
		c := s[i]
		if c == `\\` && i + 1 < s.len && !word_char(s[i + 1]) && s[i + 1] != `\n` {
			m.buf << s[i + 1]
			i += 2
			continue
		}
		if c == `\`` {
			end := s.index_after('`', i + 1) or { -1 }
			if end > i {
				m.emit()
				m.out << Span{
					...m.span(s[i + 1..end])
					code: true
				}
				i = end + 1
				continue
			}
		}
		if c == `$` {
			end := math_end(s, i)
			if end > i {
				m.emit()
				m.out << Span{
					...m.span(s[i + 1..end])
					math: true
				}
				i = end + 1
				continue
			}
		}
		if c == `~` && i + 1 < s.len && s[i + 1] == `~` {
			m.emit()
			m.strike = !m.strike
			i += 2
			continue
		}
		if (c == `*` || c == `_`) && i + 1 < s.len && s[i + 1] == c {
			m.emit()
			m.bold = !m.bold
			i += 2
			continue
		}
		if c == `*` || (c == `_` && (i == 0 || !word_char(s[i - 1]) || m.italic)) {
			closing := m.italic && (c == `*` || i + 1 >= s.len || !word_char(s[i + 1]))
			opening := !m.italic && i + 1 < s.len && s[i + 1] != ` `
			if closing || opening {
				m.emit()
				m.italic = !m.italic
				i++
				continue
			}
		}
		if c == `!` && i + 1 < s.len && s[i + 1] == `[` {
			if alt, src, end := bracket_link(s, i + 1) {
				m.emit()
				m.out << Span{
					text:  alt
					image: src
					link:  m.link
				}
				i = end
				continue
			}
		}
		if c == `[` && m.link == '' {
			if label, url, end := bracket_link(s, i) {
				m.emit()
				m.link = url
				for sp in parse_inline(label) {
					m.out << Span{
						...sp
						bold:   sp.bold || m.bold
						italic: sp.italic || m.italic
						strike: sp.strike || m.strike
						link:   url
					}
				}
				m.link = ''
				i = end
				continue
			}
		}
		if c == `<` {
			end := s.index_after('>', i + 1) or { -1 }
			if end > 0 && is_url(s[i + 1..end]) {
				m.emit()
				url := s[i + 1..end]
				m.out << Span{
					...m.span(url)
					link: url
				}
				i = end + 1
				continue
			}
		}
		if c == `h` && m.link == '' && (i == 0 || !word_char(s[i - 1])) && is_url(s[i..]) {
			mut end := i
			for end < s.len && s[end] !in [` `, `\n`, `<`, `>`, `"`] {
				end++
			}
			for end > i && s[end - 1] in [`.`, `,`, `;`, `:`, `!`, `?`, `)`] {
				end-- // trailing punctuation ends the sentence, not the url
			}
			m.emit()
			url := s[i..end]
			m.out << Span{
				...m.span(url)
				link: url
			}
			i = end
			continue
		}
		m.buf << c
		i++
	}
	m.emit()
	return m.out
}

fn is_url(s string) bool {
	return s.starts_with('http://') || s.starts_with('https://')
}

// bracket_link reads `[label](target)` at i; it returns the label, the target (without a
// title) and the index after it.
fn bracket_link(s string, i int) ?(string, string, int) {
	mut depth := 0
	mut close := -1
	for j in i .. s.len {
		if s[j] == `[` {
			depth++
		} else if s[j] == `]` {
			depth--
			if depth == 0 {
				close = j
				break
			}
		}
	}
	if close < 0 || close + 1 >= s.len || s[close + 1] != `(` {
		return none
	}
	end := s.index_after(')', close + 2) or { return none }
	mut target := s[close + 2..end].trim_space()
	if sp := target.index(' ') {
		target = target[..sp] // `(url "title")`
	}
	return s[i + 1..close], target.trim('<>'), end + 1
}

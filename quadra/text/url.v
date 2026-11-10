// URLs in plain text: the one detector for the terminal, the code editor and Markdown
// (specs/04, URLs). It works on runes; find_urls maps its ranges to the bytes of a line.
module text

const url_prefixes = ['https://', 'http://', 'www.']

// UrlSpan is a URL found in text, start..end, and what it opens.
pub struct UrlSpan {
pub:
	start int
	end   int
	url   string // `www.` gains `https://`
}

// is_white is char::is_whitespace of the terminal's original.
pub fn is_white(r rune) bool {
	c := u32(r)
	return c == 32 || (c >= 9 && c <= 13) || c == 0x85 || c == 0xa0 || c == 0x1680
		|| (c >= 0x2000 && c <= 0x200a) || c == 0x2028 || c == 0x2029 || c == 0x202f
		|| c == 0x205f || c == 0x3000
}

// url_trailing is punctuation that ends the sentence around a URL, not the URL.
pub fn url_trailing(r rune) bool {
	return r in [`)`, `]`, `}`, `>`, `.`, `,`, `;`, `:`, `!`, `?`, `'`, `"`, `\``]
}

// url_stop ends a URL: whitespace, `<`, `>` and `"`.
pub fn url_stop(r rune) bool {
	return is_white(r) || r == `<` || r == `>` || r == `"`
}

fn word_rune(r rune) bool {
	return (r >= `0` && r <= `9`) || (r >= `a` && r <= `z`) || (r >= `A` && r <= `Z`) || r >= 0x80
}

fn starts_with_at(chars []rune, at int, p string) bool {
	if at + p.len > chars.len {
		return false
	}
	for i in 0 .. p.len {
		if chars[at + i] != rune(p[i]) {
			return false
		}
	}
	return true
}

// url_run matches a URL prefix at `at` (after a word boundary): its length (0 when none
// matches) and the end of the run up to whitespace, `<`, `>` or `"`, before trailing
// punctuation is stripped.
pub fn url_run(chars []rune, at int) (int, int) {
	if at > 0 && word_rune(chars[at - 1]) {
		return 0, at
	}
	for p in url_prefixes {
		if !starts_with_at(chars, at, p) {
			continue
		}
		mut end := at + p.len
		for end < chars.len && !url_stop(chars[end]) {
			end++
		}
		return p.len, end
	}
	return 0, at
}

// url_trim strips trailing punctuation from a run ending at `end`, down to `floor` (the end
// of the prefix). A `)` that closes a `(` inside the URL stays (`.../Foo_(bar)`).
pub fn url_trim(chars []rune, floor int, end int) int {
	mut e := end
	for e > floor && url_trailing(chars[e - 1]) {
		if chars[e - 1] == `)` && paren_depth(chars, floor, e - 1) > 0 {
			break
		}
		e--
	}
	return e
}

// paren_depth is how many more parentheses chars[a..b] opens than it closes.
pub fn paren_depth(chars []rune, a int, b int) int {
	mut depth := 0
	for k in a .. b {
		if chars[k] == `(` {
			depth++
		} else if chars[k] == `)` {
			depth--
		}
	}
	return depth
}

// url_text is what a URL opens.
pub fn url_text(chars []rune) string {
	s := chars.string()
	return if s.starts_with('www.') { 'https://${s}' } else { s }
}

// url_spans lists the URLs in chars as rune ranges.
pub fn url_spans(chars []rune) []UrlSpan {
	mut out := []UrlSpan{}
	mut i := 0
	for i < chars.len {
		plen, end := url_run(chars, i)
		if plen == 0 {
			i++
			continue
		}
		stripped := url_trim(chars, i + plen, end)
		if stripped > i + plen {
			out << UrlSpan{i, stripped, url_text(chars[i..stripped])}
		}
		i = end
	}
	return out
}

// may_have_url is the cheap test before a scan: every URL holds `://` or `www.`.
pub fn may_have_url(s string) bool {
	return s.contains('://') || s.contains('www.')
}

// find_urls lists the URLs of a line that may be opened as byte ranges.
// A line with no `://` or `www.` costs one substring search and allocates nothing.
pub fn find_urls(s string) []UrlSpan {
	if !may_have_url(s) {
		return []UrlSpan{}
	}
	chars := s.runes()
	spans := url_spans(chars)
	if spans.len == 0 {
		return spans
	}
	// Each rune's bytes, as runes() decoded them: an invalid sequence is one U+FFFD per byte
	// (utf8_char_len of its lead byte overcounted, and the ranges ran past the line).
	mut offs := []int{len: chars.len + 1}
	mut b := 0
	for k, r in chars {
		offs[k] = b
		b += if r == `�` && !is_fffd_at(s, b) { 1 } else { r.length_in_bytes() }
	}
	offs[chars.len] = s.len
	mut out := []UrlSpan{cap: spans.len}
	for sp in spans {
		if url_openable(sp.url) {
			out << UrlSpan{offs[sp.start], offs[sp.end], sp.url}
		}
	}
	return out
}

// is_fffd_at says whether s holds an encoded U+FFFD at b (not one runes() made of a bad byte).
fn is_fffd_at(s string, b int) bool {
	return b + 2 < s.len && s[b] == 0xef && s[b + 1] == 0xbf && s[b + 2] == 0xbd
}

// url_openable is the gate for links in user content: http, https and mailto only, and no
// control, bidi or space characters. No other scheme (file:, ms-*:, a custom protocol
// handler) is ever handed to the shell.
pub fn url_openable(url string) bool {
	lower := url.to_lower()
	if !lower.starts_with('https://') && !lower.starts_with('http://')
		&& !lower.starts_with('mailto:') {
		return false
	}
	for r in url.runes() {
		c := u32(r)
		if c < 0x21 || c == 0x7f || (c >= 0x80 && c <= 0x9f) || is_white(r)
			|| (c >= 0x202a && c <= 0x202e) || (c >= 0x2066 && c <= 0x2069)
			|| c == 0x200e || c == 0x200f {
			return false
		}
	}
	return true
}

// Tokens and the lazy highlighter (specs/18). A tokenizer works one line at a time and
// returns the state the next line starts in; the highlighter keeps those end states.
module code

// TokenKind mirrors the fields of kit.Syntax; the view maps one to the other.
pub enum TokenKind {
	keyword
	control
	str
	number
	constant
	typ
	function
	variable
	property
	operator
	punctuation
	comment
	tag
	attribute
	regexp
	escape
	invalid
}

// Token covers line bytes start..end. Bytes no token covers are plain text.
pub struct Token {
pub:
	start int
	end   int
	kind  TokenKind
}

pub interface Tokenizer {
	tokenize(line string, state int) ([]Token, int)
}

pub struct PlainTokenizer {}

pub fn (t PlainTokenizer) tokenize(line string, state int) ([]Token, int) {
	return []Token{}, 0
}

// tokenizer_for picks a tokenizer by file name ('src/a.rs', 'Dockerfile') or by a fence
// info string ('python', 'js {.class}', 'rust title=main.rs'), ignoring case. It tries the
// whole name, then the first word, then the extension.
pub fn tokenizer_for(path string) Tokenizer {
	mut p := path.to_lower()
	if p.contains_u8(`{`) {
		p = p.all_before('{')
	}
	p = p.trim_space()
	p = p.all_after_last('/').all_after_last('\\')
	word := p.all_before(' ')
	for name in [p, word, p.all_after_last('.'), word.all_after_last('.')] {
		if t := tokenizer_named(name) {
			return t
		}
	}
	return PlainTokenizer{}
}

fn tokenizer_named(name string) ?Tokenizer {
	if name in ['v', 'vsh', 'vlang'] {
		return VTokenizer{}
	}
	if name in ['yaml', 'yml'] {
		return YamlTokenizer{}
	}
	if name in ['md', 'markdown'] {
		return MarkdownTokenizer{}
	}
	if sp := lang_named(name) {
		return LangTokenizer{
			spec: sp
		}
	}
	return none
}

// Lex is the scanning state shared by the tokenizers.
struct Lex {
	s string
mut:
	i   int
	out []Token
}

fn (l &Lex) at(k int) u8 {
	return if l.i + k < l.s.len { l.s[l.i + k] } else { 0 }
}

// add emits a..l.i as kind, merging with an adjacent token of the same kind.
fn (mut l Lex) add(a int, kind TokenKind) {
	if l.i <= a {
		return
	}
	if l.out.len > 0 && l.out.last().kind == kind && l.out.last().end == a {
		l.out[l.out.len - 1] = Token{l.out.last().start, l.i, kind}
		return
	}
	l.out << Token{a, l.i, kind}
}

fn (mut l Lex) skip_space() {
	for l.i < l.s.len && (l.s[l.i] == ` ` || l.s[l.i] == `\t`) {
		l.i++
	}
}

fn is_ident(c u8) bool {
	return c.is_letter() || c.is_digit() || c == `_` || c >= 0x80
}

// Highlighter keeps the end state of every line above `states.len`; lines below are
// stale. An edit (Doc.dirty) trims the list, and asking for a line tokenizes forward from
// the last known state only as far as that line, so a keystroke costs the visible lines.
@[heap]
pub struct Highlighter {
pub:
	tok Tokenizer
mut:
	states []int
}

pub fn new_highlighter(tok Tokenizer) &Highlighter {
	return &Highlighter{
		tok: tok
	}
}

fn (mut h Highlighter) sync(mut d Doc) {
	if d.dirty < h.states.len {
		h.states.trim(d.dirty)
	}
	d.dirty = max_int
}

// state_before is the state line n starts in.
pub fn (mut h Highlighter) state_before(mut d Doc, n int) int {
	h.sync(mut d)
	// ponytail: a far jump after an edit near the top retokenizes everything in between;
	// stop early when a recomputed state matches the old one if that shows in profiles.
	for h.states.len < n {
		h.states << h.end_state(d, h.states.len)
	}
	return if n == 0 { 0 } else { h.states[n - 1] }
}

fn (h &Highlighter) end_state(d &Doc, k int) int {
	prev := if k == 0 { 0 } else { h.states[k - 1] }
	_, st := h.tok.tokenize(d.buf.line(k), prev)
	return st
}

// tokens returns the tokens of line n.
pub fn (mut h Highlighter) tokens(mut d Doc, n int) []Token {
	st := h.state_before(mut d, n)
	toks, end := h.tok.tokenize(d.buf.line(n), st)
	if h.states.len == n {
		h.states << end
	}
	return toks
}

// known is how many line end states are cached (for tests and benchmarks).
pub fn (h &Highlighter) known() int {
	return h.states.len
}

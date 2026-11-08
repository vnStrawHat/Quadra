// Parser: tokens to the math tree. Errors are collected (the first one is reported) and
// parsing goes on, so a formula with a typo still shows what it can.
module tex

struct Parser {
	toks []Tok
mut:
	pos  int
	font Font
	err  string
}

// parse turns TeX math into a node list; the error is the first problem found.
pub fn parse(src string) ([]Node, string) {
	mut p := Parser{
		toks: lex(src)
	}
	nodes := p.list(.eof)
	if p.err == '' && p.peek().kind != .eof {
		p.fail('unexpected ${p.peek().text}')
	}
	return nodes, p.err
}

fn (mut p Parser) fail(msg string) {
	if p.err == '' {
		p.err = msg
	}
}

fn (mut p Parser) skip_spaces() {
	for p.toks[p.pos].kind == .space {
		p.pos++
	}
}

fn (mut p Parser) peek() Tok {
	p.skip_spaces()
	return p.toks[p.pos]
}

fn (mut p Parser) next() Tok {
	t := p.peek()
	if t.kind != .eof {
		p.pos++
	}
	return t
}

// stops reports whether t ends the list being parsed.
fn stops(t Tok, until TokKind) bool {
	if t.kind == .eof || (t.kind == until && until != .char) || t.kind == .amp {
		return true
	}
	return (t.kind == .cmd && t.text in ['\\', 'right', 'end', 'middle'])
		|| (until == .char && t.kind == .char && t.text == ']')
}

// list parses atoms with their scripts until a stop token (left in place).
fn (mut p Parser) list(until TokKind) []Node {
	mut out := []Node{}
	for {
		t := p.peek()
		if stops(t, until) {
			return out
		}
		if t.kind in [.sup, .sub] || (t.kind == .char && t.text == "'") {
			p.scripts(mut out)
			continue
		}
		if n := p.atom() {
			out << n
		}
	}
	return out
}

// scripts attaches ^, _ or primes to the last node.
fn (mut p Parser) scripts(mut out []Node) {
	t := p.next()
	mut base := if out.len > 0 {
		out.pop()
	} else {
		Node{
			kind: .group
		}
	}
	mut s := if base.kind == .scripts {
		base
	} else {
		Node{
			kind: .scripts
			body: [base]
		}
	}
	if t.kind == .char { // a prime: ′ in the superscript
		mut primes := "'"
		for p.toks[p.pos].kind == .char && p.toks[p.pos].text == "'" {
			p.pos++
			primes += "'"
		}
		glyph := ['′', '″', '‴'][primes.len - 1] or { '‴' }
		s = Node{
			...s
			a:     [Node{
				kind: .atom
				text: glyph
			}]
			has_a: true
		}
	} else if t.kind == .sup {
		prime := s.has_a && s.a.len == 1 && s.a[0].text in ['′', '″', '‴']
		if s.has_a && !prime {
			p.fail('double superscript')
		}
		mut a := if prime { s.a.clone() } else { []Node{} }
		a << p.arg()
		s = Node{
			...s
			a:     a
			has_a: true
		}
	} else {
		if s.has_b {
			p.fail('double subscript')
		}
		s = Node{
			...s
			b:     p.arg()
			has_b: true
		}
	}
	out << s
}

// arg is a braced group or a single atom.
fn (mut p Parser) arg() []Node {
	t := p.peek()
	if t.kind == .lbrace {
		p.pos++
		body := p.list(.rbrace)
		if p.peek().kind == .rbrace {
			p.pos++
		} else {
			p.fail('missing }')
		}
		return body
	}
	if stops(t, .eof) || t.kind in [.sup, .sub] {
		p.fail('missing argument')
		return []
	}
	if n := p.atom() {
		return [n]
	}
	return []
}

// opt_arg is an optional [..] argument (\sqrt[3]{x}).
fn (mut p Parser) opt_arg() ?[]Node {
	t := p.peek()
	if t.kind != .char || t.text != '[' {
		return none
	}
	p.pos++
	body := p.list(.char)
	if p.peek().text == ']' {
		p.pos++
	} else {
		p.fail('missing ]')
	}
	return body
}

// raw is the text of a braced argument, spaces kept (\text, \operatorname, \begin).
fn (mut p Parser) raw() string {
	if p.peek().kind != .lbrace {
		t := p.next()
		return t.text
	}
	p.pos++
	mut depth := 1
	mut s := ''
	for p.toks[p.pos].kind != .eof {
		t := p.toks[p.pos]
		p.pos++
		if t.kind == .lbrace {
			depth++
		} else if t.kind == .rbrace {
			depth--
			if depth == 0 {
				return s
			}
		}
		s += if t.kind == .cmd {
			if t.text.len == 1 { t.text } else { '\\' + t.text }
		} else {
			t.text
		}
	}
	p.fail('missing }')
	return s
}

fn (mut p Parser) atom() ?Node {
	t := p.next()
	match t.kind {
		.lbrace {
			body := p.list(.rbrace)
			if p.peek().kind == .rbrace {
				p.pos++
			} else {
				p.fail('missing }')
			}
			return Node{
				kind: .group
				body: body
			}
		}
		.char {
			return p.char_atom(t.text)
		}
		.cmd {
			return p.command(t.text)
		}
		else {
			p.fail('unexpected ${t.text}')
			return none
		}
	}
}

fn (mut p Parser) char_atom(c string) Node {
	r := c.runes()[0]
	if is_alpha(r) || (r >= `α` && r <= `ω`) || (r >= `0` && r <= `9`) {
		return Node{
			kind: .atom
			text: styled(r,
				if r >= `0` && r <= `9` && p.font == .normal { Font.rm } else { p.font })
		}
	}
	text, cls := match c {
		'+' { '+', Cls.bin }
		'-' { '−', Cls.bin }
		'*' { '∗', Cls.bin }
		'=', '<', '>', ':' { c, Cls.rel }
		'(', '[' { c, Cls.open }
		')', ']', '!', '?' { c, Cls.close }
		',', ';' { c, Cls.punct }
		'~' { ' ', Cls.ord }
		else { c, Cls.ord }
	}

	return Node{
		kind: .atom
		text: text
		cls:  cls
	}
}

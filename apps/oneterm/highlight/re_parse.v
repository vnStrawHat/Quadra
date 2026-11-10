// The parser of the pattern matcher (re.v): one pass, emitting relative-offset instructions.
module highlight

struct ReParser {
	src []rune
mut:
	pos     int
	classes []ReClass
}

fn (p &ReParser) peek() rune {
	return if p.pos < p.src.len { p.src[p.pos] } else { rune(0) }
}

fn re_compile(pattern string) Re {
	mut p := ReParser{
		src: pattern.runes()
	}
	mut prog := p.alt()
	if p.pos != p.src.len {
		panic('re: unexpected `${p.peek()}` in ${pattern}')
	}
	prog << ReInst{
		op: .accept
	}
	mut re := Re{
		prog:    prog
		classes: p.classes
	}
	re.build_first_set()
	return re
}

fn (mut p ReParser) alt() []ReInst {
	mut left := p.seq()
	for p.peek() == `|` {
		p.pos++
		right := p.seq()
		mut code := [ReInst{
			op: .split
			x:  1
			y:  left.len + 2
		}]
		code << left
		code << ReInst{
			op: .jmp
			x:  right.len + 1
		}
		code << right
		left = code.clone()
	}
	return left
}

fn (mut p ReParser) seq() []ReInst {
	mut code := []ReInst{}
	for p.pos < p.src.len && p.peek() != `|` && p.peek() != `)` {
		atom := p.atom()
		code << p.quantified(atom)
	}
	return code
}

fn (mut p ReParser) atom() []ReInst {
	c := p.src[p.pos]
	p.pos++
	match c {
		`(` {
			if p.peek() == `?` {
				p.pos += 2 // the only group form the patterns use is `(?:`
			}
			inner := p.alt()
			p.pos++ // `)`
			return inner
		}
		`[` {
			return p.class()
		}
		`^` {
			return [ReInst{
				op: .bol
			}]
		}
		`$` {
			return [ReInst{
				op: .eol
			}]
		}
		`\\` {
			e := p.src[p.pos]
			p.pos++
			if e == `b` {
				return [ReInst{
					op: .wb
				}]
			}
			if e == `d` || e == `s` {
				mut cls := ReClass{}
				class_add_escape(mut cls, e)
				p.classes << cls
				return [ReInst{
					op: .cls
					r:  rune(p.classes.len - 1)
				}]
			}
			return [ReInst{
				op: .ch
				r:  escape_literal(e)
			}]
		}
		else {
			return [ReInst{
				op: .ch
				r:  c
			}]
		}
	}
}

fn escape_literal(e rune) rune {
	return match e {
		`r` { `\r` }
		`n` { `\n` }
		else { e }
	}
}

fn class_add_escape(mut cls ReClass, e rune) {
	if e == `s` {
		cls.space = true
		for r in [` `, `\t`, `\n`, `\v`, `\f`, `\r`] {
			cls.ascii[r] = true
		}
	} else {
		cls.digit = true
		for r in `0` .. `9` + 1 {
			cls.ascii[r] = true
		}
	}
}

fn (mut p ReParser) class() []ReInst {
	mut cls := ReClass{}
	if p.peek() == `^` {
		cls.negated = true
		p.pos++
	}
	mut first := true
	for first || p.peek() != `]` {
		first = false
		mut c := p.src[p.pos]
		p.pos++
		if c == `\\` {
			e := p.src[p.pos]
			p.pos++
			if e == `s` || e == `d` {
				class_add_escape(mut cls, e)
				continue
			}
			c = escape_literal(e)
		}
		mut hi := c
		if p.peek() == `-` && p.pos + 1 < p.src.len && p.src[p.pos + 1] != `]` {
			p.pos++
			hi = p.src[p.pos]
			p.pos++
		}
		for r in c .. hi + 1 {
			if r < 128 {
				cls.ascii[r] = true
			} else {
				cls.other << r
			}
		}
	}
	p.pos++ // `]`
	p.classes << cls
	return [ReInst{
		op: .cls
		r:  rune(p.classes.len - 1)
	}]
}

fn (mut p ReParser) quantified(atom []ReInst) []ReInst {
	mut min := 1
	mut max := 1
	match p.peek() {
		`?` {
			min = 0
			p.pos++
		}
		`*` {
			min = 0
			max = -1
			p.pos++
		}
		`+` {
			max = -1
			p.pos++
		}
		`{` {
			p.pos++
			min = p.number()
			max = min
			if p.peek() == `,` {
				p.pos++
				max = p.number()
			}
			p.pos++ // `}`
		}
		else {
			return atom
		}
	}

	mut code := []ReInst{}
	for _ in 0 .. min {
		code << atom
	}
	if max < 0 {
		code << ReInst{
			op: .split
			x:  1
			y:  atom.len + 2
		}
		code << atom
		code << ReInst{
			op: .jmp
			x:  -(atom.len + 1)
		}
		return code
	}
	n := max - min
	for k in 0 .. n {
		code << ReInst{
			op: .split
			x:  1
			y:  (n - k - 1) * (atom.len + 1) + atom.len + 1
		}
		code << atom
	}
	return code
}

fn (mut p ReParser) number() int {
	mut n := 0
	for p.peek() >= `0` && p.peek() <= `9` {
		n = n * 10 + int(p.peek() - `0`)
		p.pos++
	}
	return n
}

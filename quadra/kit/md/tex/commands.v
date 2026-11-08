// The commands of the supported subset (specs/21, Math).
module tex

const styles = {
	'displaystyle':      0
	'textstyle':         1
	'scriptstyle':       2
	'scriptscriptstyle': 3
}

const fixed_delims = {
	'big':  1
	'Big':  2
	'bigg': 3
	'Bigg': 4
}

const environments = ['matrix', 'pmatrix', 'bmatrix', 'Bmatrix', 'vmatrix', 'Vmatrix', 'smallmatrix',
	'cases', 'aligned', 'align', 'align*', 'gathered', 'array', 'split']

fn (mut p Parser) command(name string) ?Node {
	if s := symbols[name] {
		r := s.ch.runes()
		text := if r.len == 1 && r[0] >= `α` && r[0] <= `ω` && p.font == .normal {
			italic(r[0])
		} else {
			s.ch
		}
		return Node{
			kind: .atom
			text: text
			cls:  s.cls
		}
	}
	if op := big_ops[name] {
		return Node{
			kind:   .bigop
			text:   op.ch
			cls:    .op
			limits: p.limits(if op.limits { 0 } else { -1 })
		}
	}
	if lim := functions[name] {
		return Node{
			kind:   .fn_name
			text:   name
			cls:    .op
			limits: p.limits(if lim { 0 } else { -1 })
		}
	}
	if acc := accents[name] {
		return Node{
			kind:  .accent
			text:  acc
			body:  p.arg()
			under: name == 'underline'
		}
	}
	if f := font_commands[name] {
		saved := p.font
		p.font = f
		body := p.arg()
		p.font = saved
		return Node{
			kind: .group
			body: body
		}
	}
	if w := spaces[name] {
		return Node{
			kind:   .space
			amount: w
		}
	}
	if st := styles[name] {
		return Node{
			kind:  .style
			level: st
		}
	}
	if lvl := fixed_delims[name.trim_right('lrm')] {
		return Node{
			kind:  .big
			text:  p.delimiter()
			level: lvl
			cls:   if name.ends_with('l') {
				Cls.open
			} else if name.ends_with('r') {
				Cls.close
			} else {
				Cls.ord
			}
		}
	}
	match name {
		'frac', 'dfrac', 'tfrac', 'cfrac' {
			return Node{
				kind:  .frac
				a:     p.arg()
				b:     p.arg()
				level: if name == 'dfrac' || name == 'cfrac' {
					0
				} else if name == 'tfrac' {
					1
				} else {
					-1
				}
			}
		}
		'binom', 'dbinom', 'tbinom' {
			return Node{
				kind:  .delim
				text:  '('
				text2: ')'
				body:  [
					Node{
						kind: .frac
						a:    p.arg()
						b:    p.arg()
						rule: false
					},
				]
			}
		}
		'sqrt' {
			index := p.opt_arg() or { []Node{} }
			return Node{
				kind:  .sqrt
				a:     index
				has_a: index.len > 0
				body:  p.arg()
			}
		}
		'left' {
			return p.left_right()
		}
		'text', 'textrm', 'textit', 'textbf', 'mbox', 'textnormal', 'hbox' {
			return Node{
				kind: .text
				text: p.raw()
			}
		}
		'operatorname', 'operatorname*' {
			return Node{
				kind:   .fn_name
				text:   p.raw()
				cls:    .op
				limits: p.limits(if name.ends_with('*') { 0 } else { -1 })
			}
		}
		'overset', 'underset', 'stackrel' {
			mark := p.arg()
			base := p.arg()
			return Node{
				kind:  .stack
				body:  base
				a:     if name == 'underset' { []Node{} } else { mark }
				b:     if name == 'underset' { mark } else { []Node{} }
				has_a: name != 'underset'
				has_b: name == 'underset'
				cls:   if name == 'stackrel' { Cls.rel } else { Cls.ord }
			}
		}
		'pmod' {
			mut body := [Node{
				kind:   .space
				amount: 6
			}, Node{
				kind: .atom
				text: '('
				cls:  .open
			}, Node{
				kind: .fn_name
				text: 'mod'
			}, Node{
				kind:   .space
				amount: 6
			}]
			body << p.arg()
			body << Node{
				kind: .atom
				text: ')'
				cls:  .close
			}
			return Node{
				kind: .group
				body: body
			}
		}
		'not' {
			n := p.atom() or { return none }
			text := match n.text {
				'=' { '≠' }
				'∈' { '∉' }
				'⊂' { '⊄' }
				'≡' { '≢' }
				else { n.text + '̸' }
			}

			return Node{
				...n
				text: text
			}
		}
		'begin' {
			return p.environment()
		}
		'color', 'textcolor' { // colors are not supported: the content shows in the text color
			p.raw()
			return Node{
				kind: .group
				body: if name == 'textcolor' { p.arg() } else { []Node{} }
			}
		}
		'limits', 'nolimits', 'mathstrut', 'strut', 'displaylimits' {
			return none
		}
		else {
			p.fail('unknown command \\${name}')
			return Node{
				kind: .text
				text: '\\' + name
				cls:  .ord
			}
		}
	}
}

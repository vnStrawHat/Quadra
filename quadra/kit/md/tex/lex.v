// Tokens of TeX math and the tree the parser builds from them.
module tex

enum TokKind {
	eof
	cmd  // \name, or \ and one non-letter (\, \{ \\)
	char // one character
	lbrace
	rbrace
	sup
	sub
	amp
	space
}

struct Tok {
	kind TokKind
	text string // the command name without the backslash, or the character
}

fn lex(src string) []Tok {
	mut out := []Tok{cap: src.len}
	rs := src.runes()
	mut i := 0
	for i < rs.len {
		r := rs[i]
		i++
		match r {
			`\\` {
				if i >= rs.len {
					out << Tok{.char, '\\'}
					continue
				}
				start := i
				if is_alpha(rs[i]) {
					for i < rs.len && is_alpha(rs[i]) {
						i++
					}
					// \operatorname* and friends: the star is part of the name.
					if i < rs.len && rs[i] == `*` {
						i++
					}
				} else {
					i++
				}
				out << Tok{.cmd, rs[start..i].string()}
			}
			`{` {
				out << Tok{.lbrace, '{'}
			}
			`}` {
				out << Tok{.rbrace, '}'}
			}
			`^` {
				out << Tok{.sup, '^'}
			}
			`_` {
				out << Tok{.sub, '_'}
			}
			`&` {
				out << Tok{.amp, '&'}
			}
			`%` { // a comment runs to the end of the line
				for i < rs.len && rs[i] != `\n` {
					i++
				}
			}
			` `, `\t`, `\n`, `\r` {
				out << Tok{.space, ' '}
			}
			else {
				out << Tok{.char, r.str()}
			}
		}
	}
	out << Tok{.eof, ''}
	return out
}

// Kind is what a node of the math tree is.
pub enum Kind {
	atom    // glyphs of one class
	group   // a braced list, laid out as one ordinary atom
	frac    // a over b (no rule for \binom)
	sqrt    // body under the radical, a the index
	scripts // body[0] with superscript a and subscript b
	bigop   // a big operator (its scripts become limits in display)
	fn_name // an upright operator name (sin, lim)
	delim   // body between grown delimiters text and text2
	big     // a delimiter of a fixed size (\big, \Big, ...)
	accent  // a mark (text) over body; underline when under
	text    // \text content, upright
	space   // amount mu wide
	matrix  // rows of cells
	stack   // a over body, b under body (\overset, \underset)
	style   // switches the style of what follows in its list
}

pub struct Node {
pub:
	kind   Kind
	cls    Cls
	text   string
	text2  string
	body   []Node
	a      []Node
	b      []Node
	has_a  bool
	has_b  bool
	rows   [][][]Node
	env    string
	limits int // -1 \nolimits, 0 default, 1 \limits
	amount f32
	rule   bool = true
	level  int
	under  bool
}

fn is_alpha(r rune) bool {
	return (r >= `a` && r <= `z`) || (r >= `A` && r <= `Z`)
}

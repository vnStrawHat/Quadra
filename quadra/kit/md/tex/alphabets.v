// Accents, spaces, delimiters and math alphabets (\mathbb, \mathcal, ...).
module tex

// Accents: the mark drawn over (or under) the argument.
const accents = {
	'hat':       'ˆ'
	'widehat':   'ˆ'
	'check':     'ˇ'
	'tilde':     '˜'
	'widetilde': '˜'
	'acute':     '´'
	'grave':     '`'
	'dot':       '˙'
	'ddot':      '¨'
	'breve':     '˘'
	'bar':       '¯'
	'vec':       '→'
	'overline':  ''
	'underline': ''
}

// Spaces in mu (1/18 em).
const spaces = {
	',':         f32(3)
	':':         4
	'>':         4
	';':         5
	'!':         -3
	' ':         6
	'quad':      18
	'qquad':     36
	'enspace':   9
	'thinspace': 3
}

// Delimiters that \left, \right and \big take, by the token after them.
const delimiters = {
	'(':        '('
	')':        ')'
	'[':        '['
	']':        ']'
	'|':        '|'
	'/':        '/'
	'.':        ''
	'\\{':      '{'
	'\\}':      '}'
	'\\|':      '‖'
	'\\lbrace': '{'
	'\\rbrace': '}'
	'\\langle': '⟨'
	'\\rangle': '⟩'
	'\\lvert':  '|'
	'\\rvert':  '|'
	'\\vert':   '|'
	'\\lVert':  '‖'
	'\\rVert':  '‖'
	'\\Vert':   '‖'
	'\\lfloor': '⌊'
	'\\rfloor': '⌋'
	'\\lceil':  '⌈'
	'\\rceil':  '⌉'
}

// Font is the math alphabet a command (\mathbf, ...) selects.
pub enum Font {
	normal // letters italic, as TeX sets variables
	rm
	bf
	bfit
	sf
	tt
	bb
	cal
	frak
}

const font_commands = {
	'mathrm':     Font.rm
	'mathup':     Font.rm
	'mathbf':     Font.bf
	'mathit':     Font.normal
	'boldsymbol': Font.bfit
	'bm':         Font.bfit
	'mathsf':     Font.sf
	'mathtt':     Font.tt
	'mathbb':     Font.bb
	'mathcal':    Font.cal
	'mathscr':    Font.cal
	'mathfrak':   Font.frak
}

// Letters missing from the math alphabet blocks: they live in Letterlike Symbols.
const holes = {
	'bbC':   'ℂ'
	'bbH':   'ℍ'
	'bbN':   'ℕ'
	'bbP':   'ℙ'
	'bbQ':   'ℚ'
	'bbR':   'ℝ'
	'bbZ':   'ℤ'
	'calB':  'ℬ'
	'calE':  'ℰ'
	'calF':  'ℱ'
	'calH':  'ℋ'
	'calI':  'ℐ'
	'calL':  'ℒ'
	'calM':  'ℳ'
	'calR':  'ℛ'
	'cale':  'ℯ'
	'calg':  'ℊ'
	'calo':  'ℴ'
	'frakC': 'ℭ'
	'frakH': 'ℌ'
	'frakI': 'ℑ'
	'frakR': 'ℜ'
	'frakZ': 'ℨ'
	'itih':  'ℎ'
}

// Bases of the alphabets: capital A, small a, digit 0 (0 = not in the block).
const alphabet_bases = {
	Font.bf:   [u32(0x1D400), 0x1D41A, 0x1D7CE]
	Font.bfit: [u32(0x1D468), 0x1D482, 0x1D7CE]
	Font.sf:   [u32(0x1D5A0), 0x1D5BA, 0x1D7E2]
	Font.tt:   [u32(0x1D670), 0x1D68A, 0x1D7F6]
	Font.bb:   [u32(0x1D538), 0x1D552, 0x1D7D8]
	Font.cal:  [u32(0x1D49C), 0x1D4B6, 0]
	Font.frak: [u32(0x1D504), 0x1D51E, 0]
}

const font_names = {
	Font.bb:   'bb'
	Font.cal:  'cal'
	Font.frak: 'frak'
}

// styled maps a character to its form in font f (a math alphanumeric where one exists).
fn styled(r rune, f Font) string {
	if f == .rm {
		return r.str()
	}
	if f == .normal {
		return italic(r)
	}
	bases := alphabet_bases[f] or { return r.str() }
	letter := (r >= `A` && r <= `Z`) || (r >= `a` && r <= `z`)
	if letter {
		if name := font_names[f] {
			if h := holes['${name}${r.str()}'] {
				return h
			}
		}
		base := if r <= `Z` { bases[0] + u32(r - `A`) } else { bases[1] + u32(r - `a`) }
		return rune(base).str()
	}
	if r >= `0` && r <= `9` && bases[2] != 0 {
		return rune(bases[2] + u32(r - `0`)).str()
	}
	return r.str()
}

// italic is the math italic form of a Latin or lowercase Greek letter.
fn italic(r rune) string {
	if r == `h` {
		return 'ℎ'
	}
	if r >= `a` && r <= `z` {
		return rune(0x1D44E + u32(r - `a`)).str()
	}
	if r >= `A` && r <= `Z` {
		return rune(0x1D434 + u32(r - `A`)).str()
	}
	if r >= `α` && r <= `ω` {
		return rune(0x1D6FC + u32(r - `α`)).str()
	}
	return match r {
		`ϵ` { '𝜖' }
		`ϑ` { '𝜗' }
		`ϕ` { '𝜙' }
		`ϱ` { '𝜚' }
		`ϖ` { '𝜛' }
		else { r.str() }
	}
}

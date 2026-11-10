// Language rows for the table-driven tokenizer (specs/18): one LangSpec per language
// (langs_*.v), one scanner (lex_lang.v). Word sets are matched in place, without substrings.
module code

pub enum LangMode {
	code
	markup // tags, attributes, entities, <!-- --> (HTML, XML, SVG)
	diff   // whole lines by their first characters
}

pub struct LangSpec {
pub:
	names           []string // fence names, aliases and file extensions, lowercase
	mode            LangMode
	keywords        []string
	control         []string
	types           []string
	constants       []string
	builtins        []string // colored as functions
	fold            bool     // words match case-insensitively (the sets are lowercase)
	line_comments   []string // a letter marker (rem) counts only first on a line
	comment_at_word bool     // a comment marker counts only at a word start (shell #, scss //)
	block_open      string
	block_close     string
	nest            bool     // block comments nest
	quotes          string   // string quotes; they end at the end of the line
	multi_quotes    string   // quotes whose strings run across lines
	raw_quotes      string   // quotes without escapes
	triple          bool     // ''' and """ strings across lines
	prefixes        string   // letters before a quote: r b f u (r and @ make it raw)
	interps         []string // interpolation openers: ${ #{ \( $(
	interp_in       string   // the quotes that interpolate
	long_brackets   bool     // Lua [[ ]] strings
	dollar_var      bool     // $name, ${name}, $(name) are variables
	percent_var     bool     // %name% and %1 are variables
	at_attr         bool     // @name is an attribute (decorators, annotations)
	at_var          bool     // @name is a variable
	at_kw           bool     // @name is a keyword (CSS at-rules)
	preproc         bool     // # directives at line start (C family)
	hash_attr       bool     // #[...] attributes
	lifetimes       bool     // 'a is a lifetime unless it is a char literal
	macro_bang      bool     // name! is a macro call
	cap_type        bool     // Capitalized words are types, ALL_CAPS constants
	no_dot          bool     // a word after a dot is not a property (shell paths)
	ident_dash      bool     // - joins words (cmdlets, CSS); a leading -word is an attribute
	key_colon       bool     // a word or string before : is a property (JSON, CSS)
	key_equals      bool     // a first word before = is a property (TOML, INI, make)
	sections        bool     // [section] lines are tags
	label_colon     bool     // a first word before : is a function (make targets)
	colon_label     bool     // a line starting with :name is a function (batch labels)
	selectors       bool     // CSS: words on a line with { are selectors (tags)
}

// LangTokenizer tokenizes one LangSpec row.
pub struct LangTokenizer {
pub:
	spec &LangSpec // a row of all_langs
}

// lang_named finds the row for a lowercase fence name, alias or extension.
fn lang_named(name string) ?&LangSpec {
	for i, sp in all_langs {
		if name in sp.names {
			return unsafe { &all_langs[i] }
		}
	}
	return none
}

// word_in tells whether s[a..b] is in set, comparing in place (folded to lowercase).
fn (l &Lex) word_in(set []string, a int, b int, fold bool) bool {
	n := b - a
	for w in set {
		if w.len != n {
			continue
		}
		mut k := 0
		for k < n {
			mut c := l.s[a + k]
			if fold && c >= `A` && c <= `Z` {
				c += 32
			}
			if c != w[k] {
				break
			}
			k++
		}
		if k == n {
			return true
		}
	}
	return false
}

// starts_at tells whether m starts at l.i (letters folded when fold).
fn (l &Lex) starts_at(m string, fold bool) bool {
	if m.len == 0 || l.i + m.len > l.s.len {
		return false
	}
	for k in 0 .. m.len {
		mut c := l.s[l.i + k]
		if fold && c >= `A` && c <= `Z` {
			c += 32
		}
		if c != m[k] {
			return false
		}
	}
	return true
}

fn (l &Lex) only_space_before(a int) bool {
	for k in 0 .. a {
		if l.s[k] != ` ` && l.s[k] != `\t` {
			return false
		}
	}
	return true
}

fn (l &Lex) next_nonspace(from int) int {
	mut k := from
	for k < l.s.len && (l.s[k] == ` ` || l.s[k] == `\t`) {
		k++
	}
	return k
}

fn (l &Lex) byte_at(k int) u8 {
	return if k >= 0 && k < l.s.len { l.s[k] } else { 0 }
}

// words splits a space-separated word list (keeps the tables short).
fn words(s string) []string {
	return s.split(' ')
}

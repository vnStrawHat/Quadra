// The Class enum: one u8 per char, indexing a flat style array (specs/24).
module highlight

import encoding.utf8

// Class is the semantic class of a char. 25 to 31 are reserved.
pub enum Class as u8 {
	plain        = 0 // no semantic class: ANSI colors only
	prompt_sign  = 1
	command      = 2
	option       = 3
	error        = 4
	success      = 5
	warn         = 6
	info         = 7
	debug        = 8
	path         = 9
	ip           = 10
	mac          = 11
	date_time    = 12
	number       = 13
	quoted       = 14
	operator     = 15
	bracket      = 16
	url          = 17 // from the view's URL detection, never from the scanner
	permission   = 18 // the whole block; the scanner uses the per-char classes below
	perm_type    = 19
	perm_read    = 20
	perm_write   = 21
	perm_exec    = 22
	perm_special = 23
	perm_none    = 24
}

pub const class_count = 32

// class_from_u8 converts a raw class byte; unknown values are plain.
pub fn class_from_u8(v u8) Class {
	return if v <= 24 { unsafe { Class(v) } } else { Class.plain }
}

// is_word_char: alphanumeric or underscore (the keyword and option boundaries).
fn is_word_char(c rune) bool {
	return c == `_` || is_alnum(c)
}

fn is_alnum(c rune) bool {
	if c < 128 {
		return (c >= `a` && c <= `z`) || (c >= `A` && c <= `Z`) || (c >= `0` && c <= `9`)
	}
	return utf8.is_letter(c) || utf8.is_number(c)
}

// is_re_word is `\w` of the original's regex engine: word chars plus marks and joiners.
fn is_re_word(c rune) bool {
	return is_word_char(c) || (c >= 0x300 && c <= 0x36F) || c == 0x200C || c == 0x200D
}

fn is_ws_rune(r rune) bool {
	return match r {
		9...13, 0x20, 0x85, 0xA0, 0x1680, 0x2000...0x200A, 0x2028, 0x2029, 0x202F, 0x205F, 0x3000 { true }
		else { false }
	}
}

fn is_digit(c rune) bool {
	return c >= `0` && c <= `9`
}

fn is_ascii_alpha(c rune) bool {
	return (c >= `a` && c <= `z`) || (c >= `A` && c <= `Z`)
}

fn is_hexdigit(c rune) bool {
	return is_digit(c) || (c >= `a` && c <= `f`) || (c >= `A` && c <= `F`)
}

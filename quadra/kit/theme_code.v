// Code, editor and terminal colors as theme tokens (specs/15), so editors, log viewers and
// terminals built on the kit share one palette with the chrome. The values live in the
// theme files (themes/).
module kit

import quadra.prim

pub struct Syntax {
pub mut:
	keyword     prim.Color
	control     prim.Color
	str         prim.Color
	number      prim.Color
	constant    prim.Color
	typ         prim.Color
	function    prim.Color
	variable    prim.Color
	property    prim.Color
	operator    prim.Color
	punctuation prim.Color
	comment     prim.Color
	tag         prim.Color
	attribute   prim.Color
	regexp      prim.Color
	escape      prim.Color
	invalid     prim.Color
}

pub struct EditorColors {
pub mut:
	foreground         prim.Color // plain text in editors and single-line inputs (finish: text)
	cursor             prim.Color
	line_highlight     prim.Color
	gutter             prim.Color
	gutter_active      prim.Color
	selection          prim.Color
	selection_inactive prim.Color
	find_match         prim.Color
	find_current       prim.Color
	bracket_match      prim.Color
	indent_guide       prim.Color
	whitespace         prim.Color
	diff_add           prim.Color // line tints; gutter marks use the palette's ok/error/warn
	diff_remove        prim.Color
	diff_change        prim.Color
}

pub struct TerminalColors {
pub mut:
	ansi       [16]prim.Color // black red green yellow blue magenta cyan white, then bright
	foreground prim.Color
	background prim.Color
	cursor     prim.Color
	selection  prim.Color
}

pub struct CodeColors {
pub mut:
	syntax   Syntax
	editor   EditorColors
	terminal TerminalColors
}

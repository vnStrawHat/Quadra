// Cells, colors and character widths of the terminal grid (specs/17).
module term

import quadra.text

// Cell is 16 bytes. ch 0 is a blank. Colors: 0 = default, 1 + index for the 256-color
// palette, true_color | rgb for 24-bit.
pub struct Cell {
pub mut:
	ch    u32
	fg    u32
	bg    u32
	flags u16
	pad   u16 // 1 + an index into Screen.extras (link, underline color, overline), 0 for none
}

pub const true_color = u32(0x1000000)

pub const f_bold = u16(1)
pub const f_faint = u16(2)
pub const f_italic = u16(4)
pub const f_underline = u16(8)
pub const f_double = u16(16) // double underline (with f_underline)
pub const f_curly = u16(32) // curly underline (with f_underline)
pub const f_blink = u16(64)
pub const f_inverse = u16(128)
pub const f_hidden = u16(256)
pub const f_strike = u16(512)
pub const f_wide = u16(1024) // the first cell of a two-cell character
pub const f_spacer = u16(2048) // the second cell of a two-cell character
pub const f_dotted = u16(4096) // dotted underline (with f_underline)
pub const f_dashed = u16(8192) // dashed underline (with f_underline)

const underline_styles = f_underline | f_double | f_curly | f_dotted | f_dashed

// f_zone is the OSC 133 zone of a cell: the two top flag bits, see Zone.
pub const f_zone = u16(0xc000)

// Zone says what a cell is part of, from the shell's OSC 133 marks.
pub enum Zone {
	none
	prompt // after A, the shell's prompt
	input  // after B, the command being typed
	output // after C, what the command printed
}

// zone is the OSC 133 zone this cell was printed in.
@[inline]
pub fn (c Cell) zone() Zone {
	return unsafe { Zone(int(c.flags >> 14)) }
}

@[inline]
fn zone_bits(z Zone) u16 {
	return u16(z) << 14
}

// style_of keeps what a run of text shares (colors and the flags that change painting).
@[inline]
pub fn (c Cell) same_style(o Cell) bool {
	return c.fg == o.fg && c.bg == o.bg && c.pad == o.pad
		&& (c.flags & ~(f_wide | f_spacer)) == (o.flags & ~(f_wide | f_spacer))
}

pub fn palette(i int) u32 {
	return u32(1 + i)
}

pub fn rgb_color(r int, g int, b int) u32 {
	return true_color | (u32(r & 255) << 16) | (u32(g & 255) << 8) | u32(b & 255)
}

// width is the number of cells a code point takes: 0 for combining marks, 2 for East Asian
// wide characters and emoji, 1 otherwise.
pub fn width(r u32) int {
	return text.rune_width(r)
}

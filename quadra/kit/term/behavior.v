// Behavior: what an app may change about a terminal (specs/22). The defaults are the plain
// embedded terminal the kit has always shown.
module term

import quadra.prim
import quadra.kit
import quadra.text

pub enum CursorChoice {
	program // the program decides (DECSCUSR)
	block
	bar
	underline
}

// Stats are running totals, for an app that wants to show throughput.
pub struct Stats {
pub mut:
	bytes    u64 // received from the program
	chunks   int // deliveries
	parse_us i64
	frames   int // paints of the grid
	paint_us i64
}

pub struct Behavior {
pub mut:
	family           string // '' = the theme's mono face
	size             f32    // 0 = font_size
	weight           int = 400
	line_height      f32 = 1.25
	padding          prim.Edges
	cursor           CursorChoice
	cursor_blink     bool = true
	copy_on_select   bool
	middle_paste     bool
	wheel            f32  = 1 // lines per wheel line
	alternate_scroll bool = true
	bell             bool // BEL raises the badge
	clipboard_keys   bool = true // ctrl-shift-c and -v (off when the app binds its own)
	ligatures        bool = true // false: no contextual alternates for the terminal font
	fallbacks        []string   // families tried for a glyph the font lacks, before the system's
	features         []string   // OpenType features turned on (`calt` here beats `ligatures`)
	cursor_color     prim.Color // the cursor over the theme's; alpha 0 = unset
	cell_width       f32        // advance override in px; 0 = from the font
	min_contrast     f32        // minimum WCAG contrast of text; 0 and <= 1 = off
}

fn (t &Terminal) text_style(th &kit.Theme) text.TextStyle {
	base := if t.b.size > 0 { t.b.size } else { font_size }
	size := if base + t.zoom < 6 {
		f32(6)
	} else if base + t.zoom > 100 {
		f32(100)
	} else {
		base + t.zoom
	}
	return text.TextStyle{
		family:      if t.b.family != '' { t.b.family } else { th.mono_family }
		size:        th.scaled(size)
		weight:      t.b.weight
		line_height: t.b.line_height
	}
}

// shape_now is the cursor shape to draw: the setting, or the program's.
fn (t &Terminal) shape_now() CursorShape {
	return match t.b.cursor {
		.program { t.screen.shape }
		.block { CursorShape.block }
		.bar { CursorShape.bar }
		.underline { CursorShape.underline }
	}
}

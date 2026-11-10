// Dynamic colors: OSC 4, 10, 11, 12, 17, 19, 104, 110-112 (specs/17).
module term

import quadra.prim
import quadra.kit

// Screen.over slots after the 256 palette entries. 0 = no override.
const key_fg = 256
const key_bg = 257
const key_cursor = 258
const key_sel_bg = 259
const key_sel_fg = 260
const color_slots = 261

// parse_color_spec reads rgb:r/g/b (1 to 4 hex digits a channel, scaled) or #rgb forms. It
// returns the true-color cell value, or 0 for a bad spec.
fn parse_color_spec(spec string) u32 {
	if spec.starts_with('rgb:') {
		f := spec[4..].split('/')
		if f.len != 3 {
			return 0
		}
		mut rgb := [3]int{}
		for i, part in f {
			if part == '' || part.len > 4 || !is_hex(part) {
				return 0
			}
			rgb[i] = 255 * int(part.parse_uint(16, 32) or { return 0 }) / ((1 << (part.len * 4)) - 1)
		}
		return rgb_color(rgb[0], rgb[1], rgb[2])
	}
	if spec.starts_with('#') {
		// #rgb, #rrggbb and longer: truncated or filled to one byte a channel.
		n := (spec.len - 1) / 3
		if n == 0 || !is_hex(spec[1..]) {
			return 0
		}
		mut rgb := [3]int{}
		for i in 0 .. 3 {
			part := spec[1 + i * n..1 + (i + 1) * n]
			v := int(part.parse_uint(16, 32) or { return 0 }) << 4
			rgb[i] = v >> (4 * (part.len - 1))
		}
		return rgb_color(rgb[0], rgb[1], rgb[2])
	}
	return 0
}

fn is_hex(s string) bool {
	for c in s {
		if hex_val(c) < 0 {
			return false
		}
	}
	return s.len > 0
}

fn (mut s Screen) set_color(key int, v u32) {
	s.over[key], s.has_over = v, true
	s.gen++
}

fn (mut s Screen) reset_color(key int) {
	s.over[key] = 0
	s.gen++
}

fn (mut s Screen) osc_set_or_query(key int, spec string, bel bool) {
	v := parse_color_spec(spec)
	if v != 0 {
		s.set_color(key, v)
	} else if spec == '?' && s.color_queries.len < 512 {
		s.color_queries << ColorQuery{key, bel}
	}
}

// osc_palette is OSC 4: every complete index;spec pair is applied.
fn (mut p Parser) osc_palette(mut s Screen, args []string) {
	mut i := 1
	for i + 1 < args.len {
		idx := args[i].int()
		if args[i].len > 0 && idx >= 0 && idx <= 255 {
			s.osc_set_or_query(idx, args[i + 1], p.bel)
		}
		i += 2
	}
}

// osc_dynamic is OSC 10, 11, 12: more parameters advance to the next color, up to cursor.
fn (mut p Parser) osc_dynamic(mut s Screen, parts []string) {
	mut key := key_fg + parts[0].int() - 10
	for param in parts[1..] {
		if key > key_cursor {
			break
		}
		s.osc_set_or_query(key, param, p.bel)
		key++
	}
}

// osc_selection is OSC 17 and 19: one parameter each.
fn (mut p Parser) osc_selection(mut s Screen, parts []string) {
	if parts.len >= 2 && parts[1] != '' {
		key := if parts[0] == '17' { key_sel_bg } else { key_sel_fg }
		s.osc_set_or_query(key, parts[1], p.bel)
	}
}

// osc_reset_palette is OSC 104: bare resets 0..255, otherwise the listed indices.
fn (mut s Screen) osc_reset_palette(args []string) {
	if args.len == 1 || args[1] == '' {
		for i in 0 .. 256 {
			s.over[i] = 0
		}
		s.gen++
		return
	}
	for a in args[1..] {
		i := a.int()
		if a.len > 0 && i >= 0 && i <= 255 {
			s.reset_color(i)
		}
	}
}

// answer_queries replies to the color questions of the last chunk, overrides first and
// then the theme, in the terminator the question used.
fn (mut s Screen) answer_queries(pal kit.TerminalColors) {
	for q in s.color_queries {
		c := if q.key < 256 && s.over[q.key] != 0 {
			prim.rgb(s.over[q.key] & 0xffffff)
		} else {
			s.color_default(q.key, pal)
		}
		// One byte a channel, doubled: rgb:RRRR/GGGG/BBBB.
		r, g, b := byte_of(c.r), byte_of(c.g), byte_of(c.b)
		end := if q.bel { '\x07' } else { '\x1b\\' }
		mut prefix := match q.key {
			key_fg { '10' }
			key_bg { '11' }
			key_cursor { '12' }
			key_sel_bg { '17' }
			key_sel_fg { '19' }
			else { '4;${q.key}' }
		}

		s.replies << '\x1b]${prefix};rgb:${r}${r}/${g}${g}/${b}${b}${end}'.bytes()
	}
	s.color_queries.clear()
}

fn (s &Screen) color_default(key int, pal kit.TerminalColors) prim.Color {
	if key >= 256 && s.over[key] != 0 {
		return prim.rgb(s.over[key] & 0xffffff)
	}
	return match key {
		key_fg, key_sel_fg { pal.foreground }
		key_bg { pal.background }
		key_cursor { pal.cursor }
		key_sel_bg { pal.selection }
		else { resolve(palette(key), pal.foreground, pal) }
	}
}

fn byte_of(v f32) string {
	return '${u8(clamp(int(v * 255 + 0.5), 0, 255)):02x}'
}

// paint_palette is the theme with the program's dynamic colors and reverse video applied.
pub fn (s &Screen) paint_palette(base kit.TerminalColors) kit.TerminalColors {
	if !s.has_over && !s.reverse_video {
		return base
	}
	mut pal := base
	if s.has_over {
		for i in 0 .. 16 {
			if s.over[i] != 0 {
				pal.ansi[i] = prim.rgb(s.over[i] & 0xffffff)
			}
		}
		if s.over[key_fg] != 0 {
			pal.foreground = prim.rgb(s.over[key_fg] & 0xffffff)
		}
		if s.over[key_bg] != 0 {
			pal.background = prim.rgb(s.over[key_bg] & 0xffffff)
		}
		if s.over[key_cursor] != 0 {
			pal.cursor = prim.rgb(s.over[key_cursor] & 0xffffff)
		}
		if s.over[key_sel_bg] != 0 {
			pal.selection = prim.rgb(s.over[key_sel_bg] & 0xffffff)
		}
	}
	if s.reverse_video {
		pal.foreground, pal.background = pal.background, pal.foreground
	}
	return pal
}

// color_of resolves a cell color with palette overrides 16..255 (0..15 are in pal).
@[inline]
fn (s &Screen) color_of(c u32, def prim.Color, pal &kit.TerminalColors) prim.Color {
	if c == 0 {
		return def
	}
	if c & true_color != 0 {
		return prim.rgb(c & 0xffffff)
	}
	if s.has_over && s.over[c - 1] != 0 {
		return prim.rgb(s.over[c - 1] & 0xffffff)
	}
	return if int(c - 1) < 16 { pal.ansi[c - 1] } else { cube_color(int(c - 1)) }
}

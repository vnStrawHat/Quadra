// Minimum contrast of terminal text against its background (WCAG 2.x), specs/17.
module term

import math
import quadra.prim

struct ContrastSlot {
mut:
	fg  prim.Color
	bg  prim.Color
	min f32
	out prim.Color
	set bool
}

fn linear(ch f32) f32 {
	return if ch <= 0.03928 { ch / 12.92 } else { f32(math.pow((ch + 0.055) / 1.055, 2.4)) }
}

fn luminance(c prim.Color) f32 {
	return 0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
}

// contrast_ratio is the WCAG ratio of two colors, at least 1.
pub fn contrast_ratio(a prim.Color, b prim.Color) f32 {
	la, lb := luminance(a), luminance(b)
	hi, lo := if la >= lb { la, lb } else { lb, la }
	return (hi + 0.05) / (lo + 0.05)
}

fn to_hsl(c prim.Color) (f32, f32, f32) {
	r, g, b := c.r, c.g, c.b
	mx := f32_max3(r, g, b)
	mn := f32_min3(r, g, b)
	l := (mx + mn) / 2
	if mx == mn {
		return 0, 0, l
	}
	d := mx - mn
	s := if l > 0.5 { d / (2 - mx - mn) } else { d / (mx + mn) }
	mut h := if mx == r {
		(g - b) / d + (if g < b { f32(6) } else { 0 })
	} else if mx == g {
		(b - r) / d + 2
	} else {
		(r - g) / d + 4
	}
	h *= 60 // degrees, as prim.hsla takes
	return h, s, l
}

fn f32_max3(a f32, b f32, c f32) f32 {
	return if a > b && a > c {
		a
	} else if b > c {
		b
	} else {
		c
	}
}

fn f32_min3(a f32, b f32, c f32) f32 {
	return if a < b && a < c {
		a
	} else if b < c {
		b
	} else {
		c
	}
}

// ensure_contrast pushes `fg` away from `bg` until `min` is met, keeping its hue and alpha.
pub fn ensure_contrast(fg prim.Color, bg prim.Color, min f32) prim.Color {
	if min <= 1 || contrast_ratio(fg, bg) >= min {
		return fg
	}
	h, s, l := to_hsl(fg)
	mut up, mut down := l, l
	mut up_ok, mut down_ok := false, false
	for _ in 0 .. 40 {
		if !up_ok {
			up = fclamp(up + 0.03, 0, 1)
			up_ok = contrast_ratio(prim.hsla(h, s, up, fg.a), bg) >= min
		}
		if !down_ok {
			down = fclamp(down - 0.03, 0, 1)
			down_ok = contrast_ratio(prim.hsla(h, s, down, fg.a), bg) >= min
		}
		if up_ok && down_ok {
			break
		}
	}
	mut chosen := up
	if up_ok && down_ok {
		chosen = if up - l <= l - down { up } else { down }
	} else if down_ok {
		chosen = down
	} else if !up_ok
		&& contrast_ratio(prim.hsla(h, s, up, fg.a), bg) < contrast_ratio(prim.hsla(h, s, down, fg.a), bg) {
		chosen = down
	}
	return prim.hsla(h, s, chosen, fg.a)
}

// is_decorative_char: glyphs drawn for shape, which keep their color.
fn is_decorative_char(c u32) bool {
	return (c >= 0x2500 && c <= 0x25ff) || (c >= 0xe0b0 && c <= 0xe0ca)
		|| (c >= 0xe0cc && c <= 0xe0d7)
}

// text_contrast applies the terminal's minimum contrast to one run's text color, except for a
// color the program chose exactly (24-bit or palette 16+) and decorative characters.
fn (mut t Terminal) text_contrast(c Cell, fg prim.Color, bg prim.Color) prim.Color {
	min := t.b.min_contrast
	if min <= 1 || is_decorative_char(c.ch) {
		return fg
	}
	inverse := c.flags & f_inverse != 0
	src := if inverse { c.bg } else { c.fg }
	if src & true_color != 0 || (src != 0 && src - 1 >= 16) {
		return fg
	}
	slot := u32(fg.r * 251) * 31 + u32(fg.g * 241) * 17 + u32(fg.b * 239) * 7 +
		u32(bg.r * 233) * 3 + u32(bg.g * 229) + u32(bg.b * 227)
	mut s := &t.contrast[slot % 64]
	if s.set && s.min == min && s.fg == fg && s.bg == bg {
		return s.out
	}
	s.fg, s.bg, s.min, s.set = fg, bg, min, true
	s.out = ensure_contrast(fg, bg, min)
	return s.out
}

module prim

// Color is straight (non-premultiplied) sRGB with alpha, components in [0, 1].
pub struct Color {
pub mut:
	r f32
	g f32
	b f32
	a f32
}

pub const transparent = Color{0, 0, 0, 0}
pub const black = Color{0, 0, 0, 1}
pub const white = Color{1, 1, 1, 1}

// rgb builds an opaque color from 0xRRGGBB.
pub fn rgb(hex u32) Color {
	return Color{f32((hex >> 16) & 0xFF) / 255, f32((hex >> 8) & 0xFF) / 255, f32(hex & 0xFF) / 255, 1}
}

// rgba builds a color from 0xRRGGBBAA.
pub fn rgba(hex u32) Color {
	return Color{f32((hex >> 24) & 0xFF) / 255, f32((hex >> 16) & 0xFF) / 255, f32((hex >> 8) & 0xFF) / 255, f32(hex & 0xFF) / 255}
}

// hsla builds a color from hue in degrees and saturation, lightness, alpha in [0, 1].
pub fn hsla(h f32, s f32, l f32, a f32) Color {
	c := (1 - abs(2 * l - 1)) * s
	hh := fmod(h, 360) / 60
	x := c * (1 - abs(fmod(hh, 2) - 1))
	mut r, mut g, mut b := f32(0), f32(0), f32(0)
	if hh < 1 {
		r, g = c, x
	} else if hh < 2 {
		r, g = x, c
	} else if hh < 3 {
		g, b = c, x
	} else if hh < 4 {
		g, b = x, c
	} else if hh < 5 {
		r, b = x, c
	} else {
		r, b = c, x
	}
	m := l - c / 2
	return Color{r + m, g + m, b + m, a}
}

pub fn (c Color) alpha(a f32) Color {
	return Color{c.r, c.g, c.b, a}
}

pub fn (c Color) is_transparent() bool {
	return c.a <= 0
}

pub fn (a Color) lerp(b Color, t f32) Color {
	return Color{a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, a.a +
		(b.a - a.a) * t}
}

// to_array returns [r, g, b, a] for GPU instance data.
pub fn (c Color) to_array() [4]f32 {
	return [c.r, c.g, c.b, c.a]!
}

pub fn (c Color) hex() string {
	b := fn (v f32) string {
		return u8(clamp(v, 0, 1) * 255 + 0.5).hex()
	}
	return '#${b(c.r)}${b(c.g)}${b(c.b)}${b(c.a)}'
}

@[inline]
fn abs(v f32) f32 {
	return if v < 0 { -v } else { v }
}

fn fmod(v f32, m f32) f32 {
	mut r := v - f32(int(v / m)) * m
	if r < 0 {
		r += m
	}
	return r
}

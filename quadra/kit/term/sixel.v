// Sixel decoder (specs/17): pixels accumulate in a growable buffer; finish makes RGBA.
module term

pub const sixel_max_dim = 4096

enum SixelState {
	ground
	repeat
	color
	raster
}

@[heap]
pub struct Sixel {
mut:
	palette  [256]u32
	color    int
	x        int
	band     int
	width    int
	height   int
	raster_w int
	raster_h int
	state    SixelState
	args     []int
	pixels   []u32
	stride   int
pub mut:
	rgba    []u8 // after flush
	img_w   int
	img_h   int
	band_px int
}

fn new_sixel() &Sixel {
	mut s := &Sixel{}
	vt340 := [[0, 0, 0], [20, 20, 80], [80, 13, 13], [20, 80, 20],
		[80, 20, 80], [20, 80, 80], [80, 80, 20], [53, 53, 53],
		[26, 26, 26], [33, 33, 60], [60, 26, 26], [33, 60, 33],
		[60, 33, 60], [33, 60, 60], [60, 60, 33], [80, 80, 80]]
	for i in 0 .. 256 {
		s.palette[i] = if i < 16 {
			rgba_u32(pct(vt340[i][0]), pct(vt340[i][1]), pct(vt340[i][2]))
		} else {
			rgba_u32(0, 0, 0)
		}
	}
	return s
}

@[inline]
fn rgba_u32(r int, g int, b int) u32 {
	return u32(r) | u32(g) << 8 | u32(b) << 16 | u32(255) << 24
}

@[inline]
fn pct(v int) int {
	return clamp(v, 0, 100) * 255 / 100
}

fn (mut s Sixel) put(b u8) {
	if s.state == .ground {
		s.ground(b)
		return
	}
	s.argument(b)
}

fn (mut s Sixel) argument(b u8) {
	if b >= `0` && b <= `9` {
		last := s.args.len - 1
		if s.args[last] < 0x7fffff {
			s.args[last] = s.args[last] * 10 + int(b - `0`)
		}
		return
	}
	if b == `;` {
		s.args << 0
		return
	}
	state := s.state
	s.state = .ground
	match state {
		.repeat {
			mut count := if s.args.len > 0 { s.args[0] } else { 0 }
			if count < 1 {
				count = 1
			}
			if b >= 0x3f && b <= 0x7e {
				s.sixel(b - 0x3f, count)
			} else {
				s.ground(b)
			}
		}
		.color {
			s.color_command()
			s.ground(b)
		}
		.raster {
			s.raster_command()
			s.ground(b)
		}
		else {}
	}
}

fn (mut s Sixel) ground(b u8) {
	match true {
		b >= 0x3f && b <= 0x7e {
			s.sixel(b - 0x3f, 1)
		}
		b == `!` {
			s.begin(.repeat)
		}
		b == `#` {
			s.begin(.color)
		}
		b == `"` {
			s.begin(.raster)
		}
		b == `$` {
			s.x = 0
		}
		b == `-` {
			s.x = 0
			s.band++
		}
		else {}
	}
}

fn (mut s Sixel) begin(st SixelState) {
	s.state = st
	s.args.clear()
	s.args << 0
}

fn (mut s Sixel) color_command() {
	reg := s.args[0] & 0xff
	if s.args.len >= 5 {
		a, b, c := s.args[2], s.args[3], s.args[4]
		if s.args[1] == 2 {
			s.palette[reg] = rgba_u32(pct(a), pct(b), pct(c))
		} else if s.args[1] == 1 {
			s.palette[reg] = hls_u32(a, b, c)
		}
	}
	s.color = reg
}

fn hls_u32(hue int, lightness int, saturation int) u32 {
	h := f32((hue + 240) % 360)
	l := f32(clamp(lightness, 0, 100)) / 100
	sat := f32(clamp(saturation, 0, 100)) / 100
	x0 := 2 * l - 1
	c := (1 - (if x0 < 0 { -x0 } else { x0 })) * sat
	m6 := h / 60
	mut h2 := m6 - 2 * f32(int(m6 / 2))
	h2 -= 1
	x := c * (1 - (if h2 < 0 { -h2 } else { h2 }))
	m := l - c / 2
	r, g, b := match int(m6) {
		0 { c, x, f32(0) }
		1 { x, c, f32(0) }
		2 { f32(0), c, x }
		3 { f32(0), x, c }
		4 { x, f32(0), c }
		else { c, f32(0), x }
	}

	return rgba_u32(clamp(int((r + m) * 255 + 0.5), 0, 255),
		clamp(int((g + m) * 255 + 0.5), 0, 255), clamp(int((b + m) * 255 + 0.5), 0, 255))
}

fn (mut s Sixel) raster_command() {
	if s.args.len >= 4 {
		w, h := if s.args[2] > sixel_max_dim { sixel_max_dim } else { s.args[2] }, if s.args[3] > sixel_max_dim {
			sixel_max_dim
		} else {
			s.args[3]
		}
		if w > 0 && h > 0 {
			s.raster_w, s.raster_h = w, h
			s.ensure_stride(w)
		}
	}
}

fn (mut s Sixel) sixel(bits u8, count int) {
	y0 := s.band * 6
	if y0 >= sixel_max_dim {
		return
	}
	x_end := if s.x + count > sixel_max_dim { sixel_max_dim } else { s.x + count }
	y_end := if y0 + 6 > sixel_max_dim { sixel_max_dim } else { y0 + 6 }
	if bits != 0 && x_end > s.x {
		s.ensure_size(x_end, y_end)
		color := s.palette[s.color]
		for bit in 0 .. 6 {
			y := y0 + bit
			if bits & (u8(1) << bit) == 0 || y >= y_end {
				continue
			}
			row := y * s.stride
			for x in s.x .. x_end {
				s.pixels[row + x] = color
			}
		}
	}
	s.x = x_end
	if x_end > s.width {
		s.width = x_end
	}
	if y_end > s.height {
		s.height = y_end
	}
}

fn (mut s Sixel) ensure_size(w int, h int) {
	s.ensure_stride(w)
	need := h * s.stride
	if s.pixels.len < need {
		s.pixels << []u32{len: need - s.pixels.len}
	}
}

fn (mut s Sixel) ensure_stride(w int) {
	if w <= s.stride {
		return
	}
	mut nw := if s.stride * 2 > w { s.stride * 2 } else { w }
	nw = clamp(nw, 64, sixel_max_dim)
	if s.stride == 0 {
		s.stride = nw
		return
	}
	rows := s.pixels.len / s.stride
	mut px := []u32{len: rows * nw}
	for y in 0 .. rows {
		src := y * s.stride
		dst := y * nw
		for x in 0 .. s.stride {
			px[dst + x] = s.pixels[src + x]
		}
	}
	s.pixels, s.stride = px, nw
}

// flush ends the string: the pending argument runs and the image becomes RGBA.
fn (mut s Sixel) flush() {
	if s.state != .ground {
		s.put(` `)
	}
	mut w := s.width
	mut h := s.height
	if s.raster_w > 0 {
		w, h = s.raster_w, s.raster_h
	}
	s.img_w, s.img_h = w, h
	s.band_px = s.band * 6
	if w == 0 || h == 0 {
		return
	}
	s.rgba = []u8{len: w * h * 4}
	rows := if s.stride == 0 { 0 } else { s.pixels.len / s.stride }
	copy_w := if w < s.stride { w } else { s.stride }
	y_end := if h < rows { h } else { rows }
	for y in 0 .. y_end {
		src := y * s.stride
		dst := y * w * 4
		for x in 0 .. copy_w {
			px := s.pixels[src + x]
			if px != 0 {
				s.rgba[dst + x * 4] = u8(px)
				s.rgba[dst + x * 4 + 1] = u8(px >> 8)
				s.rgba[dst + x * 4 + 2] = u8(px >> 16)
				s.rgba[dst + x * 4 + 3] = u8(px >> 24)
			}
		}
	}
}

// SVG path data ("M10 10 L20 20 a5 5 0 0 1 5 5 Z") parsing.
module vector

import quadra.prim

// numbers extracts every number from a string ("1,2 -3.5e2" -> [1, 2, -350]).
pub fn numbers(s string) []f32 {
	mut out := []f32{}
	mut i := 0
	for i < s.len {
		c := s[i]
		if c.is_digit() || c == `-` || c == `+` || c == `.` {
			mut j := i + 1
			mut seen_dot := c == `.`
			for j < s.len {
				d := s[j]
				if d.is_digit() {
					j++
				} else if d == `.` && !seen_dot {
					seen_dot = true
					j++
				} else if (d == `e` || d == `E`) && j + 1 < s.len {
					j += if s[j + 1] == `-` || s[j + 1] == `+` { 2 } else { 1 }
				} else {
					break
				}
			}
			out << s[i..j].f32()
			i = j
		} else {
			i++
		}
	}
	return out
}

// tokens splits path data into command letters and numbers. Arc flags may be packed
// without separators ("a1 1 0 011 1"), so flags are read one digit at a time.
fn parse_path_data(mut p Path, d string) {
	mut i := 0
	mut cmd := u8(` `)
	mut prev_ctrl := prim.Point{}
	mut prev_cmd := u8(` `)
	mut nums := []f32{}
	for {
		for i < d.len && !d[i].is_letter() && !(d[i].is_digit() || d[i] == `-` || d[i] == `.`) {
			i++
		}
		if i >= d.len {
			break
		}
		if d[i].is_letter() {
			cmd = d[i]
			i++
			if cmd == `Z` || cmd == `z` {
				p.close()
				prev_cmd = cmd
				continue
			}
		}
		arity := match cmd.ascii_str().to_upper() {
			'M', 'L', 'T' { 2 }
			'H', 'V' { 1 }
			'C' { 6 }
			'S', 'Q' { 4 }
			'A' { 7 }
			else { 0 }
		}

		if arity == 0 {
			break
		}
		nums.clear()
		for k in 0 .. arity {
			v := read_num(d, mut &i, (cmd == `a` || cmd == `A`) && (k == 3 || k == 4)) or { break }
			nums << v
		}
		if nums.len < arity {
			break
		}
		rel := cmd >= `a`
		c := p.current()
		ox, oy := if rel { c.x } else { f32(0) }, if rel { c.y } else { f32(0) }
		match cmd.ascii_str().to_upper() {
			'M' {
				p.move_to(nums[0] + ox, nums[1] + oy)
				cmd = if rel { `l` } else { `L` } // further pairs are line-tos
			}
			'L' {
				p.line_to(nums[0] + ox, nums[1] + oy)
			}
			'H' {
				p.line_to(nums[0] + ox, c.y)
			}
			'V' {
				p.line_to(c.x, nums[0] + oy)
			}
			'C' {
				p.cubic_to(nums[0] + ox, nums[1] + oy, nums[2] + ox, nums[3] + oy, nums[4] + ox,

					nums[5] + oy)
				prev_ctrl = prim.pt(nums[2] + ox, nums[3] + oy)
			}
			'S' {
				r := reflect(prev_cmd, `C`, `S`, prev_ctrl, c)
				p.cubic_to(r.x, r.y, nums[0] + ox, nums[1] + oy, nums[2] + ox, nums[3] + oy)
				prev_ctrl = prim.pt(nums[0] + ox, nums[1] + oy)
			}
			'Q' {
				p.quad_to(nums[0] + ox, nums[1] + oy, nums[2] + ox, nums[3] + oy)
				prev_ctrl = prim.pt(nums[0] + ox, nums[1] + oy)
			}
			'T' {
				r := reflect(prev_cmd, `Q`, `T`, prev_ctrl, c)
				p.quad_to(r.x, r.y, nums[0] + ox, nums[1] + oy)
				prev_ctrl = r
			}
			'A' {
				p.arc_to(nums[0], nums[1], nums[2], nums[3] != 0, nums[4] != 0, nums[5] + ox,

					nums[6] + oy)
			}
			else {}
		}

		prev_cmd = cmd
	}
}

fn reflect(prev u8, a u8, b u8, ctrl prim.Point, cur prim.Point) prim.Point {
	up := prev.ascii_str().to_upper()[0]
	if up == a || up == b {
		return prim.pt(2 * cur.x - ctrl.x, 2 * cur.y - ctrl.y)
	}
	return cur
}

// render rasterizes the SVG at size px (square fit). Returns coverage for single-color
// (currentColor) art, which the caller tints.
pub fn (s &Svg) render_mask(width int, height int) []u8 {
	mut cov := new_coverage(width, height)
	scale := prim.min(f32(width) / s.view_w, f32(height) / s.view_h)
	ox := -s.view_x * scale + (f32(width) - s.view_w * scale) / 2
	oy := -s.view_y * scale + (f32(height) - s.view_h * scale) / 2
	tol := f32(0.25)
	for sh in s.shapes {
		lines := sh.path.flatten(scale, ox, oy, tol)
		if _ := sh.fill {
			cov.fill(lines)
		}
		if _ := sh.stroke {
			cov.stroke(lines, sh.stroke_width * scale)
		}
	}
	return cov.to_u8()
}

// render_rgba rasterizes multi-color SVGs into premultiplied RGBA; currentColor parts
// use `current`.
pub fn (s &Svg) render_rgba(width int, height int, current prim.Color) []u8 {
	mut out := []f32{len: width * height * 4}
	scale := prim.min(f32(width) / s.view_w, f32(height) / s.view_h)
	ox := -s.view_x * scale + (f32(width) - s.view_w * scale) / 2
	oy := -s.view_y * scale + (f32(height) - s.view_h * scale) / 2
	for sh in s.shapes {
		lines := sh.path.flatten(scale, ox, oy, 0.25)
		for pass in 0 .. 2 {
			paint := if pass == 0 { sh.fill } else { sh.stroke }
			c0 := paint or { continue }
			c := if sh.current { current } else { c0 }
			mut cov := new_coverage(width, height)
			if pass == 0 {
				cov.fill(lines)
			} else {
				cov.stroke(lines, sh.stroke_width * scale)
			}
			for y in 0 .. height {
				for x in 0 .. width {
					a := cov.data[y * (width + 1) + x] * c.a
					if a <= 0 {
						continue
					}
					i := (y * width + x) * 4
					out[i] = c.r * a + out[i] * (1 - a)
					out[i + 1] = c.g * a + out[i + 1] * (1 - a)
					out[i + 2] = c.b * a + out[i + 2] * (1 - a)
					out[i + 3] = a + out[i + 3] * (1 - a)
				}
			}
		}
	}
	return out.map(u8(prim.clamp(it, 0, 1) * 255 + 0.5))
}

// is_single_color reports whether the art only uses currentColor (tintable mask).
pub fn (s &Svg) is_single_color() bool {
	return s.shapes.all(it.current)
}

// hash identifies a path's geometry, for caching rasterized results.
pub fn (p &Path) hash() u64 {
	mut h := u64(1469598103934665603)
	for c in p.cmds {
		h = (h ^ u64(int(c.verb))) * 1099511628211
		for q in c.pts {
			h = (h ^ u64(u32(q.x * 64))) * 1099511628211
			h = (h ^ u64(u32(q.y * 64))) * 1099511628211
		}
	}
	return h
}

// read_num reads the number at i (a single digit for arc flags) and moves i past it.
fn read_num(d string, mut i &int, flag bool) ?f32 {
	for i < d.len
		&& (d[*i] == ` ` || d[*i] == `,` || d[*i] == `\n` || d[*i] == `\t` || d[*i] == `\r`) {
		unsafe {
			*i += 1
		}
	}
	if *i >= d.len || (d[*i].is_letter() && d[*i] != `e` && d[*i] != `E`) {
		return none
	}
	if flag {
		v := f32(d[*i] - `0`)
		unsafe {
			*i += 1
		}
		return v
	}
	mut j := *i + 1
	mut dot := d[*i] == `.`
	for j < d.len {
		c := d[j]
		if c.is_digit() {
			j++
		} else if c == `.` && !dot {
			dot = true
			j++
		} else if (c == `e` || c == `E`) && j + 1 < d.len {
			j += if d[j + 1] == `-` || d[j + 1] == `+` { 2 } else { 1 }
		} else {
			break
		}
	}
	v := d[*i..j].f32()
	unsafe {
		*i = j
	}
	return v
}

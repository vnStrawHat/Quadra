// A small SVG reader: path, circle, ellipse, rect, line, polyline, polygon and <g>,
// with fill, stroke and stroke-width (inherited). Enough for icon sets and simple art.
module vector

import quadra.prim

pub struct Shape {
pub:
	path         Path
	fill         ?prim.Color // none = no fill
	stroke       ?prim.Color
	stroke_width f32
	current      bool // painted with the element's color (currentColor)
}

pub struct Svg {
pub:
	view_x f32
	view_y f32
	view_w f32
	view_h f32
pub mut:
	shapes []Shape
}

struct PaintAttrs {
	fill         ?prim.Color = prim.black
	fill_current bool
	stroke       ?prim.Color
	stroke_cur   bool
	width        f32 = 1
}

pub fn parse_svg(src string) !Svg {
	mut svg := Svg{
		view_w: 24
		view_h: 24
	}
	mut stack := [PaintAttrs{}]
	mut i := 0
	for {
		lt := src.index_after('<', i) or { break }
		gt := src.index_after('>', lt) or { break }
		tag := src[lt + 1..gt]
		i = gt + 1
		if tag.starts_with('/') {
			if stack.len > 1 && tag.trim_space() in ['/g', '/svg'] {
				stack.pop()
			}
			continue
		}
		if tag.starts_with('?') || tag.starts_with('!') {
			continue
		}
		name := tag.all_before(' ').all_before('/').trim_space()
		attrs := parse_attrs(tag)
		self_closing := tag.ends_with('/')
		paint := inherit(stack.last(), attrs)
		match name {
			'svg' {
				if vb := attrs['viewBox'] {
					n := numbers(vb)
					if n.len == 4 && n[2] > 0 && n[3] > 0 && finite(n[0]) && finite(n[1])
						&& finite(n[2]) && finite(n[3]) {
						svg = Svg{n[0], n[1], n[2], n[3], svg.shapes}
					}
				}
				stack << paint
			}
			'g' {
				if !self_closing {
					stack << paint
				}
			}
			else {
				if p := shape_path(name, attrs) {
					svg.shapes << Shape{
						path:         p
						fill:         paint.fill
						stroke:       paint.stroke
						stroke_width: paint.width
						current:      paint.fill_current || paint.stroke_cur
					}
				}
			}
		}
	}
	return svg
}

fn parse_attrs(tag string) map[string]string {
	mut out := map[string]string{}
	mut i := 0
	for {
		eq := tag.index_after('=', i) or { break }
		q := tag[eq + 1..].trim_left(' ')
		if q.len == 0 {
			break
		}
		quote := q[0]
		start := tag.index_after(quote.ascii_str(), eq) or { break } + 1
		end := tag.index_after(quote.ascii_str(), start) or { break }
		names := tag[i..eq].fields()
		if names.len > 0 { // `="x"` names nothing
			out[names.last()] = tag[start..end]
		}
		i = end + 1
	}
	return out
}

fn color_attr(v string) (?prim.Color, bool) {
	s := v.trim_space()
	return match true {
		s == 'none' {
			?prim.Color(none), false
		}
		s == 'currentColor' {
			?prim.Color(prim.black), true
		}
		s.starts_with('#') && s.len == 7 {
			?prim.Color(prim.rgb(u32(s[1..].parse_uint(16, 32) or { 0 }))), false
		}
		s.starts_with('#') && s.len == 4 {
			?prim.Color(prim.rgb(u32(('${s[1..2]}${s[1..2]}${s[2..3]}${s[2..3]}${s[3..4]}${s[3..4]}').parse_uint(16,
				32) or { 0 }))), false
		}
		s == 'white' {
			?prim.Color(prim.white), false
		}
		else {
			?prim.Color(prim.black), false
		}
	}
}

fn inherit(parent PaintAttrs, a map[string]string) PaintAttrs {
	mut p := parent
	if f := a['fill'] {
		c, cur := color_attr(f)
		p = PaintAttrs{
			...p
			fill:         c
			fill_current: cur
		}
	}
	if s := a['stroke'] {
		c, cur := color_attr(s)
		p = PaintAttrs{
			...p
			stroke:     c
			stroke_cur: cur
		}
	}
	if w := a['stroke-width'] {
		p = PaintAttrs{
			...p
			width: w.f32()
		}
	}
	return p
}

fn shape_path(name string, a map[string]string) ?Path {
	mut p := new_path()
	match name {
		'path' {
			parse_path_data(mut p, a['d'] or { return none })
		}
		'circle' {
			p.ellipse(attr_f32(a, 'cx'), attr_f32(a, 'cy'), attr_f32(a, 'r'), attr_f32(a, 'r'))
		}
		'ellipse' {
			p.ellipse(attr_f32(a, 'cx'), attr_f32(a, 'cy'), attr_f32(a, 'rx'), attr_f32(a, 'ry'))
		}
		'rect' {
			rx := if 'rx' in a { attr_f32(a, 'rx') } else { attr_f32(a, 'ry') }
			p.rounded_rect(attr_f32(a, 'x'), attr_f32(a, 'y'), attr_f32(a, 'width'), attr_f32(a,
				'height'), rx)
		}
		'line' {
			p.move_to(attr_f32(a, 'x1'), attr_f32(a, 'y1'))
			p.line_to(attr_f32(a, 'x2'), attr_f32(a, 'y2'))
		}
		'polyline', 'polygon' {
			n := numbers(a['points'] or { return none })
			for k := 0; k + 1 < n.len; k += 2 {
				if k == 0 {
					p.move_to(n[0], n[1])
				} else {
					p.line_to(n[k], n[k + 1])
				}
			}
			if name == 'polygon' {
				p.close()
			}
		}
		else {
			return none
		}
	}

	return p
}

fn attr_f32(a map[string]string, k string) f32 {
	return (a[k] or { '0' }).f32()
}

module vector

fn sum(px []u8) int {
	mut s := 0
	for v in px {
		s += v
	}
	return s
}

fn test_fill_square_has_exact_area() {
	mut c := new_coverage(10, 10)
	mut p := new_path()
	p.rect(2, 2, 4, 4)
	c.fill(p.flatten(1, 0, 0, 0.25))
	px := c.to_u8()
	assert px[3 * 10 + 3] == 255
	assert px[0] == 0
	assert sum(px) == 16 * 255
}

fn test_fill_half_pixel_edges_antialias() {
	mut c := new_coverage(4, 4)
	mut p := new_path()
	p.rect(0.5, 0, 2, 4) // covers half of column 0 and half of column 2
	c.fill(p.flatten(1, 0, 0, 0.25))
	px := c.to_u8()
	assert px[0] == 128 && px[1] == 255 && px[2] == 128 && px[3] == 0
}

fn test_circle_area_is_close_to_pi_r2() {
	mut c := new_coverage(40, 40)
	mut p := new_path()
	p.ellipse(20, 20, 15, 15)
	c.fill(p.flatten(1, 0, 0, 0.1))
	area := f32(sum(c.to_u8())) / 255
	assert area > 702 && area < 712 // pi * 15^2 = 706.9
}

fn test_stroke_line_width() {
	mut c := new_coverage(20, 10)
	mut p := new_path()
	p.move_to(2, 5)
	p.line_to(18, 5)
	c.stroke(p.flatten(1, 0, 0, 0.25), 2)
	px := c.to_u8()
	assert px[5 * 20 + 10] == 255 && px[4 * 20 + 10] == 255
	assert px[2 * 20 + 10] == 0
}

fn test_svg_icon_parses_and_renders() {
	// A Lucide-style icon: stroked paths, circle and arcs with packed flags.
	icon := '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/><path d="M3 12a9 9 0 0118 0"/></svg>'
	svg := parse_svg(icon)!
	assert svg.shapes.len == 3
	assert svg.shapes[0].current
	if _ := svg.shapes[0].fill {
		assert false, 'fill="none" is inherited'
	}
	mask := svg.render_mask(48, 48)
	assert mask.len == 48 * 48
	assert sum(mask) > 255 * 200 // visibly drawn
	assert mask[22 * 48 + 22] == 0 // the lens is hollow
}

fn test_numbers_and_path_data() {
	assert numbers('1,2 -3.5e1 .5-1') == [f32(1), 2, -35, 0.5, -1]
	mut p := new_path()
	parse_path_data(mut p, 'M0 0h10v10H0z')
	lines := p.flatten(1, 0, 0, 0.25)
	assert lines.len == 1
	assert lines[0].closed
	assert lines[0].points.len == 4, '${lines[0].points.len}'
}

// Div builder: layout methods. Every method mutates and returns the Div, so chains never copy.
module ui

import quadra.layout

pub fn (self_ &Div) flex_row() &Div {
	mut d := unsafe { self_ }
	d.style.display = .flex
	d.style.direction = .row
	return d
}

pub fn (self_ &Div) flex_col() &Div {
	mut d := unsafe { self_ }
	d.style.display = .flex
	d.style.direction = .column
	return d
}

pub fn (self_ &Div) reverse() &Div {
	mut d := unsafe { self_ }
	d.style.direction = if d.style.is_row() { .row_reverse } else { .column_reverse }
	return d
}

pub fn (self_ &Div) wrap() &Div {
	mut d := unsafe { self_ }
	d.style.wrap = true
	return d
}

pub fn (self_ &Div) gap(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.gap_row, d.style.gap_col = v, v
	return d
}

pub fn (self_ &Div) gap_x(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.gap_col = v
	return d
}

pub fn (self_ &Div) gap_y(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.gap_row = v
	return d
}

pub fn (self_ &Div) grow() &Div {
	mut d := unsafe { self_ }
	d.style.grow = 1
	return d
}

pub fn (self_ &Div) flex(grow f32, shrink f32, basis layout.Length) &Div {
	mut d := unsafe { self_ }
	d.style.grow, d.style.shrink, d.style.basis = grow, shrink, basis
	return d
}

pub fn (self_ &Div) shrink_0() &Div {
	mut d := unsafe { self_ }
	d.style.shrink = 0
	return d
}

pub fn (self_ &Div) justify(j layout.Justify) &Div {
	mut d := unsafe { self_ }
	d.style.justify = j
	return d
}

pub fn (self_ &Div) justify_center() &Div {
	mut d := unsafe { self_ }
	return d.justify(.center)
}

pub fn (self_ &Div) justify_between() &Div {
	mut d := unsafe { self_ }
	return d.justify(.between)
}

pub fn (self_ &Div) justify_end() &Div {
	mut d := unsafe { self_ }
	return d.justify(.end)
}

pub fn (self_ &Div) items(a layout.Align) &Div {
	mut d := unsafe { self_ }
	d.style.align_items = a
	return d
}

pub fn (self_ &Div) items_center() &Div {
	mut d := unsafe { self_ }
	return d.items(.center)
}

pub fn (self_ &Div) items_start() &Div {
	mut d := unsafe { self_ }
	return d.items(.start)
}

pub fn (self_ &Div) items_end() &Div {
	mut d := unsafe { self_ }
	return d.items(.end)
}

pub fn (self_ &Div) self_align(a layout.AlignSelf) &Div {
	mut d := unsafe { self_ }
	d.style.align_self = a
	return d
}

pub fn (self_ &Div) center() &Div {
	mut d := unsafe { self_ }
	d.style.justify = .center
	d.style.align_items = .center
	return d
}

// --- size ---

pub fn (self_ &Div) w(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.width = layout.px(v)
	return d
}

pub fn (self_ &Div) h(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.height = layout.px(v)
	return d
}

pub fn (self_ &Div) size(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.width, d.style.height = layout.px(v), layout.px(v)
	return d
}

pub fn (self_ &Div) w_len(l layout.Length) &Div {
	mut d := unsafe { self_ }
	d.style.width = l
	return d
}

pub fn (self_ &Div) h_len(l layout.Length) &Div {
	mut d := unsafe { self_ }
	d.style.height = l
	return d
}

pub fn (self_ &Div) w_full() &Div {
	mut d := unsafe { self_ }
	d.style.width = layout.pct(100)
	return d
}

pub fn (self_ &Div) h_full() &Div {
	mut d := unsafe { self_ }
	d.style.height = layout.pct(100)
	return d
}

pub fn (self_ &Div) size_full() &Div {
	mut d := unsafe { self_ }
	d.style.width, d.style.height = layout.pct(100), layout.pct(100)
	return d
}

pub fn (self_ &Div) min_w(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.min_width = layout.px(v)
	return d
}

pub fn (self_ &Div) max_w(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.max_width = layout.px(v)
	return d
}

pub fn (self_ &Div) min_h(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.min_height = layout.px(v)
	return d
}

pub fn (self_ &Div) max_h(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.max_height = layout.px(v)
	return d
}

pub fn (self_ &Div) aspect(ratio f32) &Div {
	mut d := unsafe { self_ }
	d.style.aspect_ratio = ratio
	return d
}

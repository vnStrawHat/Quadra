// Div builder: spacing, position and overflow.
module ui

import quadra.layout

// --- spacing ---

pub fn (self_ &Div) p(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.padding.top, d.style.padding.right, d.style.padding.bottom, d.style.padding.left = v, v, v, v
	return d
}

pub fn (self_ &Div) px(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.padding.left, d.style.padding.right = v, v
	return d
}

pub fn (self_ &Div) py(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.padding.top, d.style.padding.bottom = v, v
	return d
}

pub fn (self_ &Div) pt(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.padding.top = v
	return d
}

pub fn (self_ &Div) pb(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.padding.bottom = v
	return d
}

pub fn (self_ &Div) pl(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.padding.left = v
	return d
}

pub fn (self_ &Div) pr(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.padding.right = v
	return d
}

pub fn (self_ &Div) m(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.margin.top, d.style.margin.right, d.style.margin.bottom, d.style.margin.left = v, v, v, v
	return d
}

pub fn (self_ &Div) mx(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.margin.left, d.style.margin.right = v, v
	return d
}

pub fn (self_ &Div) my(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.margin.top, d.style.margin.bottom = v, v
	return d
}

pub fn (self_ &Div) mt(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.margin.top = v
	return d
}

pub fn (self_ &Div) mb(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.margin.bottom = v
	return d
}

pub fn (self_ &Div) ml_auto() &Div {
	mut d := unsafe { self_ }
	d.style.margin_auto.set(.left)
	return d
}

pub fn (self_ &Div) mx_auto() &Div {
	mut d := unsafe { self_ }
	d.style.margin_auto.set(.left)
	d.style.margin_auto.set(.right)
	return d
}

// --- position and overflow ---

pub fn (self_ &Div) absolute() &Div {
	mut d := unsafe { self_ }
	d.style.position = .absolute
	return d
}

pub fn (self_ &Div) inset(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.inset_top, d.style.inset_right = layout.px(v), layout.px(v)
	d.style.inset_bottom, d.style.inset_left = layout.px(v), layout.px(v)
	return d
}

pub fn (self_ &Div) top(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.inset_top = layout.px(v)
	return d
}

pub fn (self_ &Div) left(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.inset_left = layout.px(v)
	return d
}

pub fn (self_ &Div) right(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.inset_right = layout.px(v)
	return d
}

pub fn (self_ &Div) bottom(v f32) &Div {
	mut d := unsafe { self_ }
	d.style.inset_bottom = layout.px(v)
	return d
}

pub fn (self_ &Div) overflow_hidden() &Div {
	mut d := unsafe { self_ }
	d.style.overflow_x, d.style.overflow_y = .hidden, .hidden
	return d
}

pub fn (self_ &Div) hidden(on bool) &Div {
	mut d := unsafe { self_ }
	if on {
		d.style.display = .none
	}
	return d
}

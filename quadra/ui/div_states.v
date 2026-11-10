// Div state styles: the hover, press, focus and group patches, and the focus ring band.
module ui

import quadra.core
import quadra.gpu
import quadra.prim

// group makes this Div a hover group: descendants restyle with group_hover / group_active
// while it is hovered or pressed (specs/08).
pub fn (self_ &Div) group() &Div {
	mut d := unsafe { self_ }
	d.states_mut().group = true
	return d
}

// group_hover applies p while the nearest enclosing group() is hovered.
pub fn (self_ &Div) group_hover(p Patch) &Div {
	mut d := unsafe { self_ }
	d.states_mut().group_hover = p
	return d
}

// group_active applies p while the nearest enclosing group() is pressed (after group_hover).
pub fn (self_ &Div) group_active(p Patch) &Div {
	mut d := unsafe { self_ }
	d.states_mut().group_active = p
	return d
}

// ring paints a band w wide just outside the border box, in c (a focus ring). Only the band
// is painted, so a translucent fill is left as it is.
pub fn (self_ &Div) ring(w f32, c prim.Color) &Div {
	mut d := unsafe { self_ }
	mut s := d.states_mut()
	s.ring_w, s.ring_color = w, c
	return d
}

fn (mut d Div) states_mut() &States {
	if d.states == unsafe { nil } {
		if d.spare != unsafe { nil } {
			d.states, d.spare = d.spare, unsafe { nil } // an arena div's, reset (arena.v)
		} else {
			d.states = &States{}
		}
	}
	return d.states
}

@[inline]
fn (d &Div) restyles() bool {
	return d.states != unsafe { nil }
		&& (d.states.hover != none || d.states.active != none || d.in_group())
}

@[inline]
fn (d &Div) in_group() bool {
	return d.states != unsafe { nil }
		&& (d.states.group || d.states.group_hover != none || d.states.group_active != none)
}

// enter_group records the group a group-patched Div follows and, for a group, makes this
// Div the one its descendants follow. It returns the group to restore after the children.
fn (mut d Div) enter_group(mut cx core.PrepaintCx) int {
	saved := cx.group
	if d.states == unsafe { nil } {
		return saved
	}
	d.states.group_hb = cx.group
	if d.states.group {
		cx.group = d.hitbox
	}
	return saved
}

fn (d &Div) paint_ring(bounds prim.Rect, r prim.Corners, mut cx core.PaintCx) {
	w := d.states.ring_w
	cx.quad(gpu.Quad{
		bounds:        prim.rect(bounds.x - w, bounds.y - w, bounds.width + w * 2, bounds.height +
			w * 2)
		border_widths: prim.edges(w)
		border_color:  d.states.ring_color
		radii:         prim.Corners{r.top_left + w, r.top_right + w, r.bottom_right + w,
			r.bottom_left + w}
	})
}

fn (d &Div) resolved(cx &core.PaintCx) (Visual, TextPatch) {
	mut v := d.visual
	mut t := d.text
	if d.states == unsafe { nil } {
		return v, t
	}
	if hp := d.states.hover {
		if cx.is_hovered(d.hitbox) {
			apply_patch(mut v, mut t, hp)
		}
	}
	if d.states.group_hb >= 0 {
		if gp := d.states.group_hover {
			if cx.is_hovered(d.states.group_hb) {
				apply_patch(mut v, mut t, gp)
			}
		}
		if gp := d.states.group_active {
			if cx.is_active(d.states.group_hb) {
				apply_patch(mut v, mut t, gp)
			}
		}
	}
	if ap := d.states.active {
		if cx.is_active(d.hitbox) {
			apply_patch(mut v, mut t, ap)
		}
	}
	if fp := d.states.focus {
		if cx.is_focused(d.focus_id) && cx.focus_visible() {
			apply_patch(mut v, mut t, fp)
		}
	}
	if dp := d.states.drop {
		if cx.is_drop_target(d.hitbox) {
			apply_patch(mut v, mut t, dp)
		}
	}
	return v, t
}

fn apply_patch(mut v Visual, mut t TextPatch, p Patch) {
	if c := p.background {
		v.background = c
	}
	if c := p.border_color {
		v.border_color = c
	}
	if o := p.opacity {
		v.opacity = o
	}
	if s := p.shadows {
		v.shadows = s
	}
	if m := p.translate {
		v.translate = m
	}
	if k := p.scale {
		v.scale = k
	}
	if r := p.rotate {
		v.rotate = r
	}
	if c := p.text_color {
		t.set_color(c)
	}
}

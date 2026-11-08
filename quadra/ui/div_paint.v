// Div builder: grid, paint, text, interaction and tree methods.
module ui

import quadra.core
import quadra.prim
import quadra.platform

// --- paint ---

pub fn (self_ &Div) bg(c prim.Color) &Div {
	mut d := unsafe { self_ }
	d.visual.background = c
	return d
}

pub fn (self_ &Div) gradient(from prim.Color, to prim.Color, angle f32) &Div {
	mut d := unsafe { self_ }
	d.visual.background = from
	d.visual.gradient_to = to
	d.visual.gradient_angle = angle
	return d
}

pub fn (self_ &Div) border(w f32) &Div {
	mut d := unsafe { self_ }
	d.style.border = prim.edges(w)
	return d
}

pub fn (self_ &Div) border_b(w f32) &Div {
	mut d := unsafe { self_ }
	d.style.border.bottom = w
	return d
}

pub fn (self_ &Div) border_t(w f32) &Div {
	mut d := unsafe { self_ }
	d.style.border.top = w
	return d
}

pub fn (self_ &Div) border_r(w f32) &Div {
	mut d := unsafe { self_ }
	d.style.border.right = w
	return d
}

pub fn (self_ &Div) border_l(w f32) &Div {
	mut d := unsafe { self_ }
	d.style.border.left = w
	return d
}

pub fn (self_ &Div) border_color(c prim.Color) &Div {
	mut d := unsafe { self_ }
	d.visual.border_color = c
	return d
}

pub fn (self_ &Div) rounded(r f32) &Div {
	mut d := unsafe { self_ }
	d.visual.radii = prim.corners(r)
	return d
}

pub fn (self_ &Div) rounded_full() &Div {
	mut d := unsafe { self_ }
	d.visual.radii = prim.corners(9999)
	return d
}

pub fn (self_ &Div) shadow(s BoxShadow) &Div {
	mut d := unsafe { self_ }
	d.visual.shadows << s
	return d
}

// shadow_level adds the layered elevation shadows of specs/15: 1 for menus, tooltips and
// popovers, 2 (and above) for pickers, dialogs and toasts. 0 adds none.
pub fn (self_ &Div) shadow_level(level int) &Div {
	mut d := unsafe { self_ }
	if level <= 0 {
		return d
	}
	layers := if level == 1 { elevated_layers } else { modal_layers }
	for l in layers {
		d.visual.shadows << BoxShadow{
			offset_y: l[0]
			blur:     l[1]
			color:    prim.Color{0, 0, 0, l[2]}
		}
	}
	return d
}

// [offset y, blur, black alpha] per shadow layer.
const elevated_layers = [[f32(2), 3, 0.12], [f32(1), 0, 0.06]]
const modal_layers = [[f32(2), 3, 0.12], [f32(3), 6, 0.08], [f32(6), 12, 0.04],
	[f32(1), 0, 0.12]]

pub fn (self_ &Div) opacity(o f32) &Div {
	mut d := unsafe { self_ }
	d.visual.opacity = o
	return d
}

// --- inherited text style ---

pub fn (self_ &Div) text_size(v f32) &Div {
	mut d := unsafe { self_ }
	d.text.set_size(v)
	return d
}

pub fn (self_ &Div) text_color(c prim.Color) &Div {
	mut d := unsafe { self_ }
	d.text.set_color(c)
	return d
}

pub fn (self_ &Div) font_weight(w int) &Div {
	mut d := unsafe { self_ }
	d.text.set_weight(w)
	return d
}

pub fn (self_ &Div) font(family string) &Div {
	mut d := unsafe { self_ }
	d.text.set_family(family)
	return d
}

pub fn (self_ &Div) line_height(v f32) &Div {
	mut d := unsafe { self_ }
	d.text.set_line_height(v)
	return d
}

// --- interaction ---

pub fn (self_ &Div) id(key string) &Div {
	mut d := unsafe { self_ }
	d.key = key
	return d
}

pub fn (self_ &Div) test_id(id string) &Div {
	mut d := unsafe { self_ }
	d.test_id = id
	return d
}

pub fn (self_ &Div) cursor(c platform.Cursor) &Div {
	mut d := unsafe { self_ }
	d.cursor = c
	return d
}

pub fn (self_ &Div) hover(p Patch) &Div {
	mut d := unsafe { self_ }
	d.states_mut().hover = p
	return d
}

pub fn (self_ &Div) active(p Patch) &Div {
	mut d := unsafe { self_ }
	d.states_mut().active = p
	return d
}

pub fn (self_ &Div) on_click(l core.Listener[core.ClickEvent]) &Div {
	mut d := unsafe { self_ }
	d.handlers << Handler{.click, false, l.any}
	return d
}

pub fn (self_ &Div) on_mouse_down(l core.Listener[core.MouseDownEvent]) &Div {
	mut d := unsafe { self_ }
	d.handlers << Handler{.mouse_down, false, l.any}
	return d
}

pub fn (self_ &Div) on_mouse_up(l core.Listener[core.MouseUpEvent]) &Div {
	mut d := unsafe { self_ }
	d.handlers << Handler{.mouse_up, false, l.any}
	return d
}

pub fn (self_ &Div) on_mouse_move(l core.Listener[core.MouseMoveEvent]) &Div {
	mut d := unsafe { self_ }
	d.handlers << Handler{.mouse_move, false, l.any}
	return d
}

pub fn (self_ &Div) on_scroll(l core.Listener[core.ScrollEvent]) &Div {
	mut d := unsafe { self_ }
	d.handlers << Handler{.scroll, false, l.any}
	return d
}

// --- tree ---

pub fn (self_ &Div) child(el core.Element) &Div {
	mut d := unsafe { self_ }
	d.children << el
	return d
}

pub fn (self_ &Div) children(els []core.Element) &Div {
	mut d := unsafe { self_ }
	d.children << els
	return d
}

// when applies f only if cond holds, keeping chains readable.
pub fn (self_ &Div) when(cond bool, f fn (mut d Div)) &Div {
	mut d := unsafe { self_ }
	if cond {
		f(mut d)
	}
	return d
}

// apply copies another Div's style (use a childless div as a reusable style value).
pub fn (self_ &Div) apply(s &Div) &Div {
	mut d := unsafe { self_ }
	d.style = s.style
	d.visual = s.visual
	if s.states != unsafe { nil } {
		d.states = &States{
			...*s.states
		}
	}
	d.text = s.text
	if c := s.cursor {
		d.cursor = c
	}
	return d
}

// style returns an empty Div used as a reusable style value with apply().
pub fn style() &Div {
	return div()
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

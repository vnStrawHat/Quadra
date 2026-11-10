// Boolean controls: checkbox and switch. The parent owns the value.
module kit

import quadra.core
import quadra.prim
import quadra.layout
import quadra.ui

@[heap]
pub struct Toggle {
	Lazy
pub mut:
	id            string
	is_switch     bool
	on            bool
	indeterminate bool
	text          string
	disabled      bool
	change        ?core.AnyListener
mut:
	div &ui.Div = unsafe { nil }
}

pub fn checkbox(id string, checked bool) &Toggle {
	return &Toggle{
		id: id
		on: checked
	}
}

pub fn switch(id string, on bool) &Toggle {
	return &Toggle{
		id:        id
		on:        on
		is_switch: true
	}
}

pub fn (self_ &Toggle) label(s string) &Toggle {
	mut t := unsafe { self_ }
	t.text = s
	return t
}

pub fn (self_ &Toggle) indeterminate(on bool) &Toggle {
	mut t := unsafe { self_ }
	t.indeterminate = on
	return t
}

pub fn (self_ &Toggle) disabled(on bool) &Toggle {
	mut t := unsafe { self_ }
	t.disabled = on
	return t
}

pub fn (self_ &Toggle) on_change(l core.Listener[bool]) &Toggle {
	mut t := unsafe { self_ }
	t.change = l.any
	return t
}

// control draws the box or the track (specs/15). Checkbox: a 16 px outlined box with an
// accent check. Switch: a 32 x 20 track inside a 2 px ring 1 px off it (38 x 26 in all),
// a 12 px knob in the text color, the accent at 40 % when on.
fn (tg &Toggle) control(t &Theme) &ui.Div {
	p := t.p
	if t.ctl.plain_toggles {
		return tg.plain_control(t)
	}
	if tg.is_switch {
		bg := if tg.on { p.accent.alpha(0.4) } else { p.raised }
		mut track := ui.div().w(32).h(20).shrink_0().rounded_full().px(2).flex_row().items_center().bg(bg).border(1).border_color(if tg.on {
			p.accent_text.alpha(0.2)
		} else {
			p.line_strong
		})
		if !tg.disabled {
			track.hover(ui.Patch{
				background: bg.lerp(p.text, if t.mode == .light { f32(0.08) } else { f32(0.05) })
			})
		}
		if tg.on {
			track.justify_end()
		}
		knob := if tg.disabled {
			f32(0.2)
		} else if tg.on {
			f32(1)
		} else {
			f32(0.5)
		}
		track.child(ui.div().size(12).rounded_full().bg(p.text.alpha(knob)))
		return ui.div().p(1).shrink_0().rounded_full().border(2).border_color(p.line_strong).child(track)
	}
	mark := if tg.disabled { p.text_disabled } else { p.accent_text }
	mut box := ui.div().size(16).shrink_0().center().rounded(t.radius_sm).border(1).border_color(if tg.disabled {
		p.line
	} else {
		p.line_strong
	}).text_color(mark)
	if tg.disabled {
		box.bg(p.raised.alpha(0.6))
	} else {
		box.hover(ui.Patch{
			border_color: p.line_strong.alpha(0.7)
		})
	}
	if tg.indeterminate {
		box.child(icon(icon_minus).size(14))
	} else if tg.on {
		box.child(icon(icon_check).size(14))
	}
	return box
}

// plain_control is the plain shape (Controls.plain_toggles): a 36 x 20 track with a 16
// thumb inset 2, or a 16 box with radius 4 filled with the accent when checked.
fn (tg &Toggle) plain_control(t &Theme) &ui.Div {
	p := t.p
	mut d := ui.div().shrink_0()
	if tg.is_switch {
		d.w(36).h(20).p(2).rounded_full().flex_row().items_center().bg(if tg.on {
			p.accent
		} else {
			p.line
		}).child(ui.div().size(16).rounded_full().bg(p.text))
		if tg.on {
			d.justify_end()
		}
	} else {
		on := tg.on || tg.indeterminate

		d.size(16).center().rounded(t.radius_md).border(1).border_color(if on {
			p.accent
		} else {
			p.line
		}).bg(if on { p.accent } else { p.input_bg }).text_color(p.text_on_accent)
		if on {
			d.child(icon(if tg.indeterminate { icon_minus } else { icon_check }).size(12))
		}
	}
	if tg.disabled {
		d.opacity(0.5)
	}
	return d
}

pub fn (mut tg Toggle) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut tg, mut cx)
	return tg.layout_inner(el, mut cx)
}

pub fn (mut tg Toggle) build_tree(mut cx core.LayoutCx) core.Element {
	t := theme_of(cx.app)
	mut d := ui.div().id(tg.id).test_id(tg.id).flex_row().items_center().gap(6).role(if tg.is_switch {
		core.Role.switch
	} else {
		core.Role.checkbox
	}).label(tg.text).checked(tg.on).disabled(tg.disabled).child(tg.control(t))
	if t.ctl.plain_toggles {
		d.gap(8)
	}
	if tg.text != '' {
		d.child(ui.text(tg.text).color(if tg.disabled { t.p.text_disabled } else { t.p.text }))
	}
	if !tg.disabled {
		d.cursor(.pointer).on_click(fire_click(tg.change, !tg.on)).on_key_down(core.handler_with(Fire[bool]{tg.change, !tg.on}, fn (f Fire[bool], e &core.KeyDownEvent, mut cx core.Cx) {
			if e.key == .space {
				fire(f.change, f.value, mut cx)
				cx.stop_propagation()
			}
		}))
	}
	tg.div = d
	return d
}

pub fn (mut tg Toggle) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	if !tg.disabled {
		tg.div.focus_id = stable_focus(mut cx, tg.id)
	}
	tg.Lazy.prepaint(bounds, mut cx)
}

// Single choice from a few options: radio_group and segmented control.
module kit

import quadra.core
import quadra.prim
import quadra.layout
import quadra.ui

@[heap]
pub struct Choice {
	Lazy
pub mut:
	id         string
	options    []string
	selected   int
	horizontal bool
	segmented  bool
	size       Size = .sm // segmented cells: .sm 22 px with 12 px labels, .md/.lg 14 px
	disabled   bool
	change     ?core.AnyListener
mut:
	div &ui.Div = unsafe { nil }
}

pub fn radio_group(id string, options []string, selected int) &Choice {
	return &Choice{
		id:       id
		options:  options
		selected: selected
	}
}

pub fn segmented(id string, options []string, selected int) &Choice {
	return &Choice{
		id:         id
		options:    options
		selected:   selected
		horizontal: true
		segmented:  true
	}
}

pub fn (self_ &Choice) horizontal() &Choice {
	mut c := unsafe { self_ }
	c.horizontal = true
	return c
}

pub fn (self_ &Choice) size(s Size) &Choice {
	mut c := unsafe { self_ }
	c.size = s
	return c
}

pub fn (self_ &Choice) disabled(on bool) &Choice {
	mut c := unsafe { self_ }
	c.disabled = on
	return c
}

pub fn (self_ &Choice) on_change(l core.Listener[int]) &Choice {
	mut c := unsafe { self_ }
	c.change = l.any
	return c
}

// option draws one choice (specs/15). Segmented: a cell of an outlined group, the selected
// one tinted with an accent label. Radio: a 12 px ring with a 6 px accent dot.
fn (ch &Choice) option(i int, label string, t &Theme) &ui.Div {
	p := t.p
	on := i == ch.selected
	mut d :=
		ui.div().test_id('${ch.id}-${i}').flex_row().items_center().gap(6).role(.radio).label(label).checked(on)
	if !ch.disabled {
		d.cursor(.pointer).on_click(fire_click(ch.change, i))
	}
	if ch.segmented {
		is_small := ch.size == .xs || ch.size == .sm
		d.px(if is_small { 8 } else { 12 }).shrink_0().h(t.height(ch.size)).justify_center().text_size(t.ui_text(if is_small {
			Size.sm
		} else {
			Size.md
		}))
		if i > 0 {
			d.border_l(1).border_color(p.line_strong.alpha(0.6))
		}
		if ch.disabled {
			d.text_color(p.text_disabled)
		} else if on {
			d.bg(p.accent_tint).text_color(p.accent_text)
		} else {
			d.text_color(p.text).hover(ui.Patch{
				background: p.panel_hover
			})
		}
		return d.child(ui.text(label))
	}
	mut ring :=
		ui.div().size(12).shrink_0().rounded_full().center().border(1).border_color(p.line_strong).bg(p.canvas)
	if on {
		ring.child(ui.div().size(6).rounded_full().bg(if ch.disabled {
			p.text_disabled
		} else {
			p.accent_text
		}))
	}
	return d.child(ring).child(ui.text(label).color(if ch.disabled {
		p.text_disabled
	} else {
		p.text
	}))
}

pub fn (mut ch Choice) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut ch, mut cx)
	return ch.layout_inner(el, mut cx)
}

pub fn (mut ch Choice) build_tree(mut cx core.LayoutCx) core.Element {
	t := theme_of(cx.app)
	mut d := ui.div().id(ch.id).test_id(ch.id).role(.group).gap(if ch.segmented {
		0
	} else if ch.horizontal {
		16
	} else {
		8
	})
	if ch.horizontal {
		d.flex_row()
	} else {
		d.flex_col()
	}
	if ch.segmented {
		d.rounded(t.radius_lg).border(1).border_color(t.p.line_strong.alpha(0.6)).overflow_hidden().items_center().self_align(.start).shrink_0()
	}
	for i, o in ch.options {
		d.child(ch.option(i, o, t))
	}
	change := ch.change
	sel := ch.selected
	n := ch.options.len
	horizontal := ch.horizontal
	d.on_key_down(core.handler_with(ChoiceKeys{change, sel, n, horizontal}, fn (k ChoiceKeys, e &core.KeyDownEvent, mut cx core.Cx) {
		change, sel, n, horizontal := k.change, k.sel, k.n, k.horizontal
		step := match e.key {
			.right {
				1
			}
			.down {
				if horizontal { 0 } else { 1 }
			}
			.left {
				-1
			}
			.up {
				if horizontal { 0 } else { -1 }
			}
			else {
				0
			}
		}

		if step != 0 && n > 0 {
			fire(change, (sel + step + n) % n, mut cx)
			cx.stop_propagation()
		}
	}))
	ch.div = d
	return d
}

pub fn (mut ch Choice) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	if !ch.disabled {
		ch.div.focus_id = stable_focus(mut cx, ch.id)
	}
	ch.Lazy.prepaint(bounds, mut cx)
}

struct ChoiceKeys {
	change     ?core.AnyListener
	sel        int
	n          int
	horizontal bool
}

// Select: a button showing the current option that opens a list of options.
module kit

import quadra.core
import quadra.prim
import quadra.layout
import quadra.ui

@[heap]
pub struct Select {
	Lazy
pub mut:
	id          string
	options     []string
	selected    int = -1
	placeholder string // '' = the installed Strings.select_placeholder
	width       f32 = 220
	disabled    bool
	change      ?core.AnyListener
mut:
	div &ui.Div = unsafe { nil }
}

pub fn select_box(id string, options []string, selected int) &Select {
	return &Select{
		id:       id
		options:  options
		selected: selected
	}
}

pub fn (self_ &Select) placeholder(s string) &Select {
	mut s_ := unsafe { self_ }
	s_.placeholder = s
	return s_
}

pub fn (self_ &Select) width(w f32) &Select {
	mut s := unsafe { self_ }
	s.width = w
	return s
}

pub fn (self_ &Select) on_change(l core.Listener[int]) &Select {
	mut s := unsafe { self_ }
	s.change = l.any
	return s
}

struct SelectOpen {
	options  []string
	selected int
	change   ?core.AnyListener
	width    f32
}

fn options_menu(s SelectOpen, _ &Theme) core.Element {
	mut items := []MenuItem{cap: s.options.len}
	for i, o in s.options {
		items << MenuItem{
			label:  o
			glyph:  if i == s.selected { icon_check } else { '' }
			action: PickIndex{s.change, i}
		}
	}
	mut m := menu(items)
	m.width = s.width
	return m
}

fn open_options(mut cx core.Cx, s SelectOpen) {
	open_popup_with(mut cx, cx.target_bounds(), .bottom_start, s, options_menu)
}

pub fn (mut s Select) request_layout(mut cx core.LayoutCx) layout.NodeId {
	el := core.tracked(mut s, mut cx)
	return s.layout_inner(el, mut cx)
}

pub fn (mut s Select) build_tree(mut cx core.LayoutCx) core.Element {
	t := theme_of(cx.app)
	shown := if s.selected >= 0 && s.selected < s.options.len {
		s.options[s.selected]
	} else if s.placeholder != '' {
		s.placeholder
	} else {
		strings_of(cx.app).select_placeholder
	}
	// A filled dropdown button (specs/15): the value, then an up-down chevron.
	mut d := ui.div().id(s.id).test_id(s.id).flex_row().items_center().gap(4).w(s.width).h(t.height(.md)).px(8).rounded(t.radius_md).bg(t.p.raised).border(1).border_color(t.p.line).role(.listbox).label(shown).disabled(s.disabled).focus_ring(ui.Patch{
		border_color: t.p.border_focused
	})

	d.child(ui.div().grow().overflow_hidden().child(ui.text(shown).color(if s.selected < 0 {
		t.p.text_3
	} else if s.disabled {
		t.p.text_disabled
	} else {
		t.p.text
	}))).child(icon(icon_chevrons_up_down).size(12).color(t.p.text_2))
	if !s.disabled {
		open := SelectOpen{s.options, s.selected, s.change, s.width}

		d.cursor(.pointer).hover(ui.Patch{
			background: t.p.raised_hover
		}).on_click(core.handler_with(open, fn (s SelectOpen, _ &core.ClickEvent, mut cx core.Cx) {
			open_options(mut cx, s)
		})).on_key_down(core.handler_with(open, fn (s SelectOpen, e &core.KeyDownEvent, mut cx core.Cx) {
			match e.key {
				.enter, .space, .down {
					open_options(mut cx, s)
				}
				else {
					return
				}
			}

			cx.stop_propagation()
		}))
	}
	s.div = d
	return d
}

pub fn (mut s Select) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	if !s.disabled {
		s.div.focus_id = stable_focus(mut cx, s.id)
	}
	s.Lazy.prepaint(bounds, mut cx)
}

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
	compact     bool // as wide as its value, the menu right-aligned (specs/15)
	size        Size = .md
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

// compact makes the select as wide as its value, its menu opening below with the right
// edges aligned.
pub fn (self_ &Select) compact() &Select {
	mut s := unsafe { self_ }
	s.compact = true
	return s
}

pub fn (self_ &Select) size(sz Size) &Select {
	mut s := unsafe { self_ }
	s.size = sz
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
	end      bool // the menu's right edge on the button's
}

// options_menu lists the options: One's menu with a check icon on the chosen row, or with
// Controls.select_input a list whose chosen row is filled and checked at its end.
fn options_menu(s SelectOpen, t &Theme) core.Element {
	list := t.ctl.select_input
	mut items := []MenuItem{cap: s.options.len}
	for i, o in s.options {
		items << MenuItem{
			label:   o
			glyph:   if i == s.selected && !list { icon_check } else { '' }
			checked: i == s.selected && list
			action:  PickIndex{s.change, i}
		}
	}
	mut m := menu(items)
	m.list = list
	if s.width > 0 {
		m.width = s.width
	}
	return m
}

fn open_options(mut cx core.Cx, s SelectOpen) {
	open_popup_with(mut cx, cx.target_bounds(), if s.end { .bottom_end } else { .bottom_start }, s,
		options_menu)
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
	// The trigger (specs/15): One's filled button with an up-down chevron, or with
	// Controls.select_input the input fill and a chevron-down.
	fg := if s.selected < 0 {
		t.p.text_3
	} else if s.disabled {
		t.p.text_disabled
	} else {
		t.p.text
	}
	mut d := ui.div().id(s.id).test_id(s.id).flex_row().items_center().h(t.height(s.size)).rounded(t.radius_md).border(1).border_color(t.p.line).text_size(t.ui_text(.md)).role(.listbox).label(shown).disabled(s.disabled).focus_ring(ui.Patch{
		border_color: t.p.border_focused
	})
	value := ui.div().grow().overflow_hidden().child(ui.text(shown).color(fg))
	if s.compact {
		d.shrink_0()
	} else {
		d.w(s.width)
	}
	if t.ctl.select_input {
		d.gap(8).px(10).bg(t.p.input_bg).child(value).child(icon(icon_chevron_down).size(16).color(if s.compact {
			fg.alpha(0.75)
		} else {
			t.p.text_2
		}))
	} else {
		d.gap(4).px(8).bg(t.p.raised).child(value).child(icon(icon_chevrons_up_down).size(12).color(t.p.text_2))
	}
	if !s.disabled {
		open := SelectOpen{s.options, s.selected, s.change, if s.compact {
			f32(0)
		} else {
			s.width
		}, s.compact}

		d.cursor(.pointer).hover(ui.Patch{
			background: if t.ctl.select_input { t.p.line.alpha(0.5) } else { t.p.raised_hover }
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

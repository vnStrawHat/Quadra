// SearchSelect: the searchable picker used for the group, the jump host and the forward kind.
// Its popup is drawn with ui.anchored inside the dialog, above it (specs/26 decision 3).
module sessionui

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.prim
import quadra.ui

@[params]
pub struct SelectOptions {
pub:
	id          string
	options     []string
	value       string
	placeholder string
	search_hint string
	searchable  bool = true
	creatable   bool // an option may be typed that the list does not have
	clearable   bool = true
	list_h      f32  = 240 // the list scrolls past this height
}

@[heap]
pub struct SearchSelect {
pub mut:
	id          string
	options     []string
	value       string
	placeholder string
	search_hint string
	searchable  bool
	creatable   bool
	clearable   bool
	is_open     bool
	query       string
	highlight   int
	matches     []int
	width       f32 = 260
	list_h      f32 = 240
	focus       core.FocusHandle
	field       core.View[input.TextInput]
	changed     ?core.AnyListener // string: the new value
	me          core.View[SearchSelect]
}

pub fn new_select(mut cx core.Cx, opts SelectOptions) core.View[SearchSelect] {
	v := cx.new_view(SearchSelect{
		id:          opts.id
		options:     opts.options
		value:       opts.value
		placeholder: opts.placeholder
		search_hint: opts.search_hint
		searchable:  opts.searchable
		creatable:   opts.creatable
		clearable:   opts.clearable
		list_h:      opts.list_h
		focus:       cx.focus_handle()
	})
	cx.update_view(v, fn (mut s SearchSelect, mut cx core.ViewCx[SearchSelect]) {
		s.me = cx.handle()
		if s.searchable {
			s.field = input.new_input(mut cx.Cx,
				id:          '${s.id}-search'
				placeholder: s.search_hint
				bare:        true
			)
			input.on_change(mut cx.Cx, s.field, cx.listener(fn (mut s SearchSelect, v &string, mut cx core.ViewCx[SearchSelect]) {
				s.set_query(*v, mut cx)
			}))
		}
	})
	return v
}

fn (mut s SearchSelect) set_query(q string, mut cx core.ViewCx[SearchSelect]) {
	s.query = q
	q_ := q.trim_space().to_lower()
	s.matches = []int{}
	for i, o in s.options {
		if q_ == '' || o.to_lower().contains(q_) {
			s.matches << i
		}
	}
	s.highlight = 0
	cx.notify()
}

fn (mut s SearchSelect) open(width f32, mut cx core.ViewCx[SearchSelect]) {
	s.is_open = true
	s.width = width
	s.set_query('', mut cx)
	if s.searchable {
		input.set_value(mut cx.Cx, s.field, '')
		cx.peek(s.field.model()).focus.focus(mut cx.Cx)
	} else {
		s.focus.focus(mut cx.Cx)
	}
}

fn (mut s SearchSelect) close(mut cx core.ViewCx[SearchSelect]) {
	s.is_open = false
	s.focus.focus(mut cx.Cx)
	cx.notify()
}

// empty_text is what the list says when no option matches (specs/26).
fn (s &SearchSelect) empty_text() string {
	q := s.query.trim_space()
	if !s.creatable {
		return 'No results.'
	}
	return if q == '' || s.options.len == 0 {
		'No groups yet. Type a name to create one.'
	} else {
		'No group matches "${q}". Press Enter to create it.'
	}
}

// commit sets the value and tells the owner.
fn (mut s SearchSelect) commit(value string, mut cx core.ViewCx[SearchSelect]) {
	s.value = value
	s.close(mut cx)
	kit.fire(s.changed, value, mut cx.Cx)
}

// clear empties the value without opening the list.
fn (mut s SearchSelect) clear(mut cx core.ViewCx[SearchSelect]) {
	s.value = ''
	cx.notify()
	kit.fire(s.changed, '', mut cx.Cx)
}

// enter picks the highlighted match; with no match, a creatable field takes the typed text.
fn (mut s SearchSelect) enter(mut cx core.ViewCx[SearchSelect]) {
	q := s.query.trim_space()
	if s.highlight < s.matches.len {
		s.commit(s.options[s.matches[s.highlight]], mut cx)
	} else if s.creatable && q != '' {
		s.commit(q, mut cx)
	}
}

fn (mut s SearchSelect) popup_key(e &core.KeyDownEvent, mut cx core.ViewCx[SearchSelect]) {
	match e.key {
		.down {
			if s.matches.len > 0 {
				s.highlight = (s.highlight + 1) % s.matches.len
			}
		}
		.up {
			if s.matches.len > 0 {
				s.highlight = (s.highlight + s.matches.len - 1) % s.matches.len
			}
		}
		.enter {
			s.enter(mut cx)
		}
		else {
			return
		}
	}

	cx.stop_propagation()
	cx.notify()
}

@[markused]
pub fn (mut s SearchSelect) render(mut cx core.ViewCx[SearchSelect]) core.Element {
	t := kit.theme(cx)
	shown := if s.value != '' { s.value } else { s.placeholder }
	mut trigger := ui.div().id(s.id).test_id(s.id).flex_row().items_center().gap(4).w_full().min_h(32).px(8).py(6).rounded(t.radius_lg).bg(t.p.canvas).border(1).border_color(t.p.line).cursor(.pointer).role(.listbox).label(shown).track_focus(s.focus).on_click(cx.listener(fn (mut s SearchSelect, _ &core.ClickEvent, mut cx core.ViewCx[SearchSelect]) {
		if s.is_open {
			s.close(mut cx)
		} else {
			s.open(cx.target_bounds().width, mut cx)
		}
	})).on_key_down(cx.listener(fn (mut s SearchSelect, e &core.KeyDownEvent, mut cx core.ViewCx[SearchSelect]) {
		if !s.is_open && (e.key == .enter || e.key == .space || e.key == .down) {
			s.open(cx.target_bounds().width, mut cx)
			cx.stop_propagation()
		}
	}))
	trigger.child(ui.div().grow().min_w(0).overflow_hidden().child(ui.text(shown).truncate().color(if s.value != '' {
		t.p.text
	} else {
		t.p.text_3
	})))
	if s.clearable && s.value != '' && !s.is_open {
		trigger.child(ui.div().test_id('${s.id}-clear').size(16).center().rounded_full().cursor(.pointer).on_mouse_down(cx.listener(fn (mut s SearchSelect, _ &core.MouseDownEvent, mut cx core.ViewCx[SearchSelect]) {
			s.clear(mut cx)
			cx.stop_propagation()
		})).child(kit.icon(kit.icon_circle_x).size(12).color(t.p.text_2)))
	}
	trigger.child(kit.icon(kit.icon_chevron_down).size(12).color(t.p.text_2))
	mut root := ui.div().flex_col().child(trigger)
	if s.is_open {
		root.child(ui.anchored(ui.div().size_full().on_mouse_down(cx.listener(fn (mut s SearchSelect, _ &core.MouseDownEvent, mut cx core.ViewCx[SearchSelect]) {
			s.close(mut cx)
		}))).fill_window().layer(5998))
		root.child(ui.anchored(s.popup(t, mut cx)).placement(.bottom_start).layer(6000))
	}
	return root
}

fn (mut s SearchSelect) popup(t &kit.Theme, mut cx core.ViewCx[SearchSelect]) core.Element {
	mut box := ui.div().test_id('${s.id}-popup').flex_col().w(s.width).py(4).bg(t.p.overlay_bg).rounded(t.radius_xl).border(1).border_color(t.p.line).shadow_level(1).text_size(t.font_size(.md)).text_color(t.p.text).on_key_down(cx.listener(fn (mut s SearchSelect, e &core.KeyDownEvent, mut cx core.ViewCx[SearchSelect]) {
		s.popup_key(e, mut cx)
	})).on_action(cx.listener(fn (mut s SearchSelect, _ &kit.Dismiss, mut cx core.ViewCx[SearchSelect]) {
		s.close(mut cx)
	}))
	if s.searchable {
		box.child(ui.div().flex_col().px(8).pb(4).mb(4).border_b(1).border_color(t.p.line).child(cx.child_view(s.field)))
	}
	if s.matches.len > 0 {
		mut list :=
			ui.div().flex_col().max_h(prim.min(cx.win.size().height * 0.4, s.list_h)).overflow_y_scroll()
		for k, i in s.matches {
			list.child(s.option_row(i, k, t, mut cx))
		}
		box.child(list)
	} else {
		box.child(ui.div().px(12).py(12).child(ui.text(s.empty_text()).size(t.font_size(.sm)).color(t.p.text_2)))
	}
	if s.creatable {
		box.child(s.create_row(t, mut cx))
	}
	return box
}

fn (mut s SearchSelect) option_row(i int, k int, t &kit.Theme, mut cx core.ViewCx[SearchSelect]) core.Element {
	opt := s.options[i]
	on := k == s.highlight
	mut row := ui.div().test_id('${s.id}-opt-${i}').flex_row().items_center().gap(6).mx(4).px(6).h(23).rounded(t.radius_md).cursor(.pointer).role(.listitem).label(opt).selected(on).on_click(core.listener_with[SearchSelect, int, core.ClickEvent](cx.entity,
		i, fn (mut s SearchSelect, i int, _ &core.ClickEvent, mut cx core.ViewCx[SearchSelect]) {
		s.commit(s.options[i], mut cx)
	}))
	if on {
		row.bg(t.p.press)
	} else {
		row.hover(ui.Patch{
			background: t.p.panel_hover
		})
	}
	if opt == s.value {
		row.child(kit.icon(kit.icon_check).size(14).color(t.p.accent_text))
	}
	return row.child(ui.text(opt))
}

fn (mut s SearchSelect) create_row(t &kit.Theme, mut cx core.ViewCx[SearchSelect]) core.Element {
	q := s.query.trim_space()
	enabled := q != ''
	label := if enabled { 'Create "${q}"' } else { 'Type to create new group' }
	mut b :=
		kit.button('${s.id}-create').label(label).icon(kit.icon_plus).ghost().disabled(!enabled)
	if enabled {
		b = b.on_click(core.listener_with[SearchSelect, string, core.ClickEvent](cx.entity, q, fn (mut s SearchSelect, q string, _ &core.ClickEvent, mut cx core.ViewCx[SearchSelect]) {
			s.commit(q, mut cx)
		}))
	}
	return ui.div().px(4).pt(4).mt(4).border_t(1).border_color(t.p.line).child(b)
}

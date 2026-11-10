// ValueView: a collapsible tree of a JSON or YAML value (specs/19). Keys use the syntax
// colors, containers show their size when collapsed, ctrl-c copies the selected path.
module data

import quadra.core
import quadra.prim
import quadra.kit
import quadra.ui

struct VRow {
	path  string
	key   string // '' for the root, '[i]' for list items
	depth int
	value Value
}

@[heap]
pub struct ValueView {
pub mut:
	id       string
	value    Value
	opened   map[string]bool // explicit toggles by path; the default opens depth < 2
	selected string
	cursor   int
	focus    core.FocusHandle
	row_h    f32 = 24
}

pub fn new_value_view(mut cx core.Cx, id string, v Value) core.View[ValueView] {
	return cx.new_view(ValueView{
		id:    id
		value: v
		focus: cx.focus_handle()
	})
}

// set_value shows another value; the open state of paths that still exist is kept.
pub fn set_value(mut cx core.Cx, v core.View[ValueView], value Value) {
	mut vv := cx.peek(v.model())
	vv.value = value
	cx.touch(v.model())
}

fn (vv &ValueView) is_open(path string, depth int) bool {
	return vv.opened[path] or { depth < 2 }
}

// rows lists the visible nodes in order (the root's children at depth 0).
fn (vv &ValueView) rows() []VRow {
	mut out := []VRow{}
	vv.walk(vv.value, '', 0, mut out)
	return out
}

fn (vv &ValueView) walk(v Value, path string, depth int, mut out []VRow) {
	for i, child in v.items {
		key := if v.kind == .map { v.keys[i] } else { '[${i}]' }
		p := if v.kind == .map { '${path}.${key}' } else { '${path}[${i}]' }
		out << VRow{p, key, depth, child}
		if child.kind in [.map, .list] && vv.is_open(p, depth) {
			vv.walk(child, p, depth + 1, mut out)
		}
	}
}

fn (mut vv ValueView) toggle(r VRow) {
	if r.value.kind in [.map, .list] {
		vv.opened[r.path] = !vv.is_open(r.path, r.depth)
	}
}

fn (mut vv ValueView) key(e &core.KeyDownEvent, mut cx core.ViewCx[ValueView]) {
	rows := vv.rows()
	if rows.len == 0 {
		return
	}
	mut i := clamp_int(vv.cursor, 0, rows.len - 1)
	match e.key {
		.down {
			i = clamp_int(i + 1, 0, rows.len - 1)
		}
		.up {
			i = clamp_int(i - 1, 0, rows.len - 1)
		}
		.right, .left {
			r := rows[i]
			open := vv.is_open(r.path, r.depth)
			if r.value.kind in [.map, .list] && open == (e.key == .left) {
				vv.toggle(r)
			}
		}
		.enter, .space {
			vv.toggle(rows[i])
		}
		.c {
			if !e.mods.has(.ctrl) {
				return
			}
			cx.clipboard_write(vv.selected)
		}
		else {
			return
		}
	}

	vv.cursor = i
	vv.selected = rows[i].path
	cx.stop_propagation()
	cx.notify()
}

fn clamp_int(v int, lo int, hi int) int {
	return if v < lo {
		lo
	} else if v > hi {
		hi
	} else {
		v
	}
}

fn value_color(th &kit.Theme, v Value) prim.Color {
	s := th.code.syntax
	return match v.kind {
		.str { s.str }
		.number { s.number }
		.boolean, .null { s.constant }
		else { th.p.text_3 }
	}
}

@[markused]
pub fn (mut vv ValueView) render(mut cx core.ViewCx[ValueView]) core.Element {
	th := kit.theme(cx)
	rows := vv.rows()
	selected := vv.selected
	focused := vv.focus.is_focused(cx.Cx)
	opened := vv.opened.clone()
	ent := cx.entity
	id := vv.id
	list := ui.uniform_list_with('${vv.id}-rows', rows.len, vv.row_h, ValueRows{rows, selected, focused, opened, ent, id, unsafe { th }},
		value_row)
	path := if vv.selected == '' { 'Select a value' } else { vv.selected }
	bar := ui.div().flex_row().items_center().gap(6).px(8).h(th.panel_header_h).shrink_0().border_b(1).border_color(th.p.line).child(ui.text(path).family(th.mono_family).size(th.ui_text(.sm)).color(th.p.text_2).selectable()).child(ui.div().grow()).child(kit.button('${vv.id}-copy').label('Copy path').size(.sm).disabled(vv.selected == '').on_click(cx.listener(fn (mut vv ValueView, _ &core.ClickEvent, mut cx core.ViewCx[ValueView]) {
		cx.clipboard_write(vv.selected)
	})))
	return ui.div().id(vv.id).test_id(vv.id).flex_col().size_full().role(.tree).track_focus(vv.focus).on_key_down(cx.listener(fn (mut vv ValueView, e &core.KeyDownEvent, mut cx core.ViewCx[ValueView]) {
		vv.key(e, mut cx)
	})).child(bar).child(ui.div().grow().child(list))
}

// ValueRows is what the visible rows are built from.
struct ValueRows {
	rows     []VRow
	selected string
	focused  bool
	opened   map[string]bool
	ent      u32
	id       string
	th       &kit.Theme = unsafe { nil }
}

fn value_row(d ValueRows, i int) core.Element {
	rows, selected, focused, ent, id, th := d.rows, d.selected, d.focused, d.ent, d.id, d.th
	r := rows[i]
	container := r.value.kind in [.map, .list]
	open := d.opened[r.path] or { r.depth < 2 }
	mut row := ui.div().test_id('${id}-row${i}').flex_row().items_center().gap(6).h_full().pl(8 +
		f32(r.depth) * 16).text_size(th.ui_text(.md)).font(th.mono_family).role(.treeitem).label(r.path).selected(r.path == selected)
	if r.path == selected {
		row.bg(th.p.press)
		if focused {
			row.border(1).border_color(th.p.border_focused)
		}
	} else {
		row.hover(ui.Patch{
			background: th.p.raised_hover
		})
	}
	row.child(if container {
		core.Element(kit.icon(if open { kit.icon_chevron_down } else { kit.icon_chevron_right }).size(14).color(th.p.text_2))
	} else {
		core.Element(ui.div().w(14).shrink_0())
	})
	row.child(ui.text(if r.key.starts_with('[') { r.key } else { '${r.key}:' }).color(if r.key.starts_with('[') {
		th.p.text_3
	} else {
		th.code.syntax.property
	}))
	if !container || !open {
		row.child(ui.text(r.value.summary()).color(value_color(th, r.value)))
	}
	row.on_click(core.listener_with[ValueView, ValueClick, core.ClickEvent](ent, ValueClick{r, i}, fn (mut vv ValueView, k ValueClick, _ &core.ClickEvent, mut cx core.ViewCx[ValueView]) {
		r, i := k.r, k.i
		if vv.selected == r.path {
			vv.toggle(r)
		}
		vv.selected, vv.cursor = r.path, i
		cx.notify()
	}))
	return row
}

struct ValueClick {
	r VRow
	i int
}

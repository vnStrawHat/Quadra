// Incremental layout (specs/05, specs/07): a build for notified views lays out only what their
// new trees change, and the result is the layout a full build makes.
module main

import quadra.core
import quadra.gpu
import quadra.kit
import quadra.prim
import quadra.testing
import quadra.ui

const blue = prim.rgb(0x2563EB)
const red = prim.rgb(0xDC2626)
const long_label = 'a label long enough to wrap onto more than one line'

struct Row {
mut:
	id    string
	label string
	color prim.Color = blue
}

@[markused]
pub fn (mut r Row) render(mut cx core.ViewCx[Row]) core.Element {
	return ui.div().test_id(r.id).w(120).px(4).bg(r.color).hover(ui.Patch{
		background: red
	}).child(ui.text(r.label))
}

struct List {
mut:
	rows []core.View[Row]
}

@[markused]
pub fn (mut l List) render(mut cx core.ViewCx[List]) core.Element {
	mut d := ui.div().test_id('list').flex_col().size_full().overflow_y_scroll()
	for v in l.rows {
		d.child(cx.child_view(v))
	}
	return d
}

fn open_list(n int) !(&testing.Harness, core.View[List], []core.View[Row]) {
	mut h := testing.new_harness()!
	mut cx := core.Cx{
		app: h.app
	}
	mut rows := []core.View[Row]{}
	for i in 0 .. n {
		rows << cx.new_view(Row{ id: 'row${i}', label: 'Row ${i}' })
	}
	v := h.open(List{ rows: rows })!
	return h, v, rows
}

fn set_color(mut r Row, c prim.Color, mut _ core.ViewCx[Row]) {
	r.color = c
}

fn set_label(mut r Row, s string, mut _ core.ViewCx[Row]) {
	r.label = s
}

fn set_color_if(mut r Row, c prim.Color, mut _ core.ViewCx[Row]) bool {
	return core.assign(mut r.color, c)
}

fn update[D](mut h testing.Harness, v core.View[Row], d D, f fn (mut r Row, d D, mut cx core.ViewCx[Row])) {
	mut cx := core.Cx{
		app: h.app
	}
	cx.update_with(v.model(), d, f)
	h.frame()
}

fn quad_color(h &testing.Harness, r prim.Rect) [4]f32 {
	for inst in h.scene().instances {
		if inst.kind == u32(gpu.Kind.quad) && inst.bounds[0] == r.x && inst.bounds[1] == r.y
			&& inst.bounds[2] == r.width && inst.bounds[3] == r.height {
			return inst.color
		}
	}
	return [f32(-1), -1, -1, -1]!
}

fn same_as_full(h &testing.Harness) {
	if d := h.window().verify_layout() {
		assert false, d
	}
}

fn test_a_paint_only_change_runs_no_layout() {
	mut h, _, rows := open_list(50)!
	w := h.window()
	before, nodes := w.stats.relayouts, w.stats.layout_nodes
	r := h.find('row5') or { panic('') }
	c0 := quad_color(h, r)
	update(mut h, rows[5], red, set_color)
	assert w.stats.relayouts == before + 1
	assert w.tree.measure_calls == 0 && w.stats.relaid == 0 // the row's layout was taken over
	assert w.stats.layout_nodes <= nodes + 1 // its nodes were filled again: no garbage but its root
	assert h.find('row5') or { panic('') } == r
	c1 := quad_color(h, r)
	assert c1 != c0 && c1[0] > c1[2] // red now
	mut fw := h.window()
	fw.invalidate()
	h.frame()
	assert quad_color(h, r) == c1 // a full build paints the same
}

fn test_a_taller_row_moves_the_rows_below() {
	mut h, _, rows := open_list(20)!
	r5 := h.find('row5') or { panic('') }
	r6 := h.find('row6') or { panic('') }
	update(mut h, rows[5], long_label, set_label)
	n5 := h.find('row5') or { panic('') }
	n6 := h.find('row6') or { panic('') }
	assert n5.height > r5.height
	assert n6.y == r6.y + n5.height - r5.height
	assert h.window().stats.relaid > 0 // the list was laid out again above the row
	same_as_full(h)
	update(mut h, rows[5], 'Row 5', set_label) // and back
	assert h.find('row6') or { panic('') } == r6
	same_as_full(h)
}

fn test_many_taller_rows_fall_back_to_one_full_layout() {
	mut h, _, rows := open_list(30)!
	w := h.window()
	n := w.stats.relayouts
	mut cx := core.Cx{
		app: h.app
	}
	for v in rows {
		cx.update_with(v.model(), long_label, set_label)
	}
	h.frame()
	assert w.stats.relayouts == n // each reached the list: one full layout instead
	r0 := h.find('row0') or { panic('') }
	r1 := h.find('row1') or { panic('') }
	assert r0.height > 30 && r1.y == r0.y + r0.height
}

struct Boxed {
mut:
	inner core.View[Row]
}

@[markused]
pub fn (mut b Boxed) render(mut cx core.ViewCx[Boxed]) core.Element {
	mut d := ui.div().flex_col()
	for i in 0 .. 30 {
		d.child(ui.text('Static ${i}'))
	}
	return d.child(ui.div().test_id('box').w(300).h(100).items_start().child(cx.child_view(b.inner)))
}

fn test_a_change_inside_a_fixed_box_stops_at_the_box() {
	mut h := testing.new_harness()!
	mut cx := core.Cx{
		app: h.app
	}
	inner := cx.new_view(Row{ id: 'inner', label: 'short' })
	h.open(Boxed{ inner: inner })!
	b := h.find('box') or { panic('') }
	update(mut h, inner, long_label, set_label)
	w := h.window()
	assert w.stats.relaid == 1 // the box answers its parent as before: nothing above it ran
	assert w.tree.measure_calls < 30 // the 30 texts beside it were not measured again
	assert h.find('box') or { panic('') } == b
	assert (h.find('inner') or { panic('') }).height > 20
	same_as_full(h)
}

fn test_hover_and_scroll_survive_incremental_builds() {
	mut h, _, rows := open_list(100)!
	h.scroll_at(50, 50, -300)
	h.frame()
	r20 := h.find('row20') or { panic('') }
	assert r20.y < 200 // scrolled
	update(mut h, rows[20], blue, set_color) // a notify that changes nothing on screen
	assert h.find('row20') or { panic('') } == r20 // still scrolled
	update(mut h, rows[19], long_label, set_label)
	n19 := h.find('row19') or { panic('') }
	n20 := h.find('row20') or { panic('') }
	assert n19.height > r20.height && n20.y == n19.y + n19.height
	same_as_full(h)
	before := quad_color(h, n20)
	h.move_to(n20.x + 2, n20.y + 2)
	h.frame()
	hovered := quad_color(h, n20)
	assert hovered != before && hovered[0] > hovered[2] // the hover patch still applies
}

struct VList {
mut:
	rows []core.View[Row]
}

@[markused]
pub fn (mut l VList) render(mut cx core.ViewCx[VList]) core.Element {
	rows := l.rows
	return ui.div().size_full().child(ui.uniform_list('vl', rows.len, 24, fn [rows] (i int) core.Element {
		return core.view_element(rows[i].any())
	}))
}

fn test_views_in_virtual_list_rows_render_in_prepaint() {
	mut h := testing.new_harness()!
	mut cx := core.Cx{
		app: h.app
	}
	mut rows := []core.View[Row]{}
	for i in 0 .. 1000 {
		rows << cx.new_view(Row{ id: 'v${i}', label: 'Item ${i}' })
	}
	h.open(VList{ rows: rows })!
	assert h.has_text('Item 2')
	update(mut h, rows[2], 'Changed', set_label)
	assert h.has_text('Changed') && !h.has_text('Item 2')
	same_as_full(h)
}

fn test_a_resize_lays_out_everything() {
	mut h, _, rows := open_list(20)!
	update(mut h, rows[3], red, set_color)
	w := h.window()
	n := w.stats.relayouts
	h.resize(500, 400)
	assert w.stats.relayouts == n // a full layout
	assert (h.find('list') or { panic('') }).width == 500
	same_as_full(h)
}

fn test_equal_writes_build_nothing() {
	mut h, _, rows := open_list(10)!
	mut cx := core.Cx{
		app: h.app
	}
	b := h.builds()
	cx.update_if(rows[1].model(), blue, set_color_if)
	h.frame()
	assert h.builds() == b
	cx.update_if(rows[1].model(), red, set_color_if)
	h.frame()
	assert h.builds() == b + 1
}

fn add_row(mut l List, v core.View[Row], mut _ core.ViewCx[List]) {
	l.rows.insert(1, v)
}

fn test_a_parent_and_a_child_notified_together() {
	mut h, list, rows := open_list(5)!
	mut cx := core.Cx{
		app: h.app
	}
	fresh := cx.new_view(Row{ id: 'fresh', label: long_label })
	cx.update_with(rows[0].model(), red, set_color)
	cx.update_with(list.model(), fresh, add_row)
	h.frame()
	f := h.find('fresh') or { panic('') }
	r0 := h.find('row0') or { panic('') }
	r1 := h.find('row1') or { panic('') }
	assert f.y == r0.y + r0.height && r1.y == f.y + f.height
	same_as_full(h)
	update(mut h, fresh, 'short', set_label) // the new view is placed: incremental too
	same_as_full(h)
}

struct KRow {
mut:
	label string
}

@[markused]
pub fn (mut k KRow) render(mut cx core.ViewCx[KRow]) core.Element {
	return ui.div().flex_row().gap(4).child(kit.button('kb').label(k.label)).child(ui.text('after'))
}

fn set_klabel(mut k KRow, s string, mut _ core.ViewCx[KRow]) {
	k.label = s
}

fn test_kit_components_built_in_layout() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(KRow{ label: 'Go' })!
	mut cx := core.Cx{
		app: h.app
	}
	for s in ['A much longer label', 'Go', 'Mid'] {
		cx.update_with(v.model(), s, set_klabel)
		h.frame()
		assert h.has_text(s)
		same_as_full(h)
	}
}

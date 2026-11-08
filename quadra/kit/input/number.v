// NumberInput: a text input that accepts numbers, with stepper buttons and arrow keys.
module input

import math
import strconv
import quadra.core
import quadra.kit
import quadra.ui

@[heap]
pub struct NumberInput {
pub mut:
	field    core.View[TextInput]
	value    f64
	min      f64 = -1.0e15
	max      f64 = 1.0e15
	step     f64 = 1
	decimals int
	change   ?core.AnyListener // f64
}

@[params]
pub struct NumberOptions {
pub:
	id       string
	value    f64
	min      f64 = -1.0e15
	max      f64 = 1.0e15
	step     f64 = 1
	decimals int
}

fn format_number(v f64, decimals int) string {
	scale := math.pow(10, decimals)
	r := math.round(v * scale) / scale
	if decimals <= 0 {
		return '${i64(r)}'
	}
	whole := i64(math.abs(r))
	frac := i64(math.round((math.abs(r) - f64(whole)) * scale))
	sign := if r < 0 { '-' } else { '' }
	digits := frac.str()
	return '${sign}${whole}.' + '0'.repeat(decimals - digits.len) + digits
}

pub fn new_number_input(mut cx core.Cx, opts NumberOptions) core.View[NumberInput] {
	field := new_input(mut cx, id: opts.id, value: format_number(opts.value, opts.decimals))
	cx.update_silent(field.model(), fn (mut v TextInput, mut _ core.ViewCx[TextInput]) {
		v.filter = fn (s string) string {
			return s.bytes().filter(it.is_digit() || it == `.` || it == `-` || it == `,`).bytestr().replace(',', '.')
		}
	})
	view := cx.new_view(NumberInput{
		field:    field
		value:    opts.value
		min:      opts.min
		max:      opts.max
		step:     opts.step
		decimals: opts.decimals
	})
	on_change(mut cx, field, core.Listener[string]{core.AnyListener{
		view:   view.id
		holder: unsafe { nil }
		call:   number_typed
	}})
	return view
}

pub fn on_number_change(mut cx core.Cx, n core.View[NumberInput], l core.Listener[f64]) {
	mut w := cx.peek(n.model())
	w.change = l.any
}

fn number_typed(_ voidptr, view voidptr, event voidptr, mut cx core.Cx) {
	mut n := unsafe { &NumberInput(view) }
	s := unsafe { *(&string(event)) }
	if v := strconv.atof64(s) {
		n.value = v
		n.emit(mut cx)
	}
}

fn (n &NumberInput) emit(mut cx core.Cx) {
	if l := n.change {
		v := n.value
		cx.app.invoke(l, &v, cx.win, true)
	}
}

fn (mut n NumberInput) bump(dir f64, mut cx core.ViewCx[NumberInput]) {
	n.value = math.min(n.max, math.max(n.min, n.value + dir * n.step))
	set_value(mut cx.Cx, n.field, format_number(n.value, n.decimals))
	n.emit(mut cx.Cx)
}

@[markused]
pub fn (mut n NumberInput) render(mut cx core.ViewCx[NumberInput]) core.Element {
	t := kit.theme(cx)
	return ui.div().flex_row().items_center().gap(4).w(160).on_key_down(cx.listener(fn (mut n NumberInput, e &core.KeyDownEvent, mut cx core.ViewCx[NumberInput]) {
		match e.key {
			.up { n.bump(1, mut cx) }
			.down { n.bump(-1, mut cx) }
			else { return }
		}

		cx.stop_propagation()
	})).child(ui.div().grow().child(cx.child_view(n.field))).child(ui.div().flex_col().child(stepper(cx.entity,
		t, 1, kit.icon_chevron_up, 'num-up')).child(stepper(cx.entity, t, -1,
		kit.icon_chevron_down, 'num-down')))
}

// stepper is one of the up and down buttons.
fn stepper(entity u32, t &kit.Theme, dir f64, glyph string, id string) core.Element {
	return ui.div().test_id(id).size(16).center().rounded(t.radius_sm).cursor(.pointer).text_color(t.p.text_2).hover(ui.Patch{
		background: t.p.panel_hover
	}).on_click(core.listener_with[NumberInput, f64, core.ClickEvent](entity, dir, fn (mut n NumberInput, dir f64, _ &core.ClickEvent, mut cx core.ViewCx[NumberInput]) {
		n.bump(dir, mut cx)
	})).child(kit.icon(glyph).size(12))
}

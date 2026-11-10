// NumberInput: a text input that accepts numbers, with stepper buttons and arrow keys
// (specs/15): up and down chevrons beside the field, or a framed `− value +` stepper.
module input

import math
import strconv
import quadra.core
import quadra.kit
import quadra.prim
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
	id       string
	stepper  bool
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
	stepper  bool // a framed `− value +` control, 128 wide
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
	field := new_input(mut cx,
		id:     opts.id
		value:  format_number(opts.value, opts.decimals)
		bare:   opts.stepper
		center: opts.stepper
	)
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
		id:       opts.id
		stepper:  opts.stepper
	})
	on_change(mut cx, field, core.Listener[string]{core.AnyListener{
		view: view.id
		call: number_typed
	}})
	return view
}

pub fn on_number_change(mut cx core.Cx, n core.View[NumberInput], l core.Listener[f64]) {
	mut w := cx.peek(n.model())
	w.change = l.any
}

fn number_typed(_ prim.Holder, view prim.Holder, event prim.Holder, mut cx core.Cx) {
	mut n := prim.held[NumberInput](view)
	s := *prim.held[string](event)
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
	mut d := ui.div().flex_row().items_center().on_key_down(cx.listener(fn (mut n NumberInput, e &core.KeyDownEvent, mut cx core.ViewCx[NumberInput]) {
		match e.key {
			.up { n.bump(1, mut cx) }
			.down { n.bump(-1, mut cx) }
			else { return }
		}

		cx.stop_propagation()
	}))
	field := ui.div().grow().min_w(0).child(cx.child_view(n.field))
	if n.stepper {
		r := t.radius_lg - 1 // inside the 1 px frame
		return d.w(128).h(t.height(.lg)).rounded(t.radius_lg).bg(t.p.input_bg).border(1).border_color(t.p.line).child(step_button(cx.entity,
			t, -1, '${n.id}-dec', prim.Corners{ top_left: r, bottom_left: r })).child(field).child(step_button(cx.entity,
			t, 1, '${n.id}-inc', prim.Corners{ top_right: r, bottom_right: r }))
	}
	return d.gap(4).w(160).child(field).child(ui.div().flex_col().child(stepper(cx.entity, t, 1,
		kit.icon_chevron_up, 'num-up')).child(stepper(cx.entity, t, -1, kit.icon_chevron_down,
		'num-down')))
}

// step_button is the stepper's `−` or `+`: ghost, 32 wide, full height, only its outer
// corners rounded.
fn step_button(entity u32, t &kit.Theme, dir f64, id string, radii prim.Corners) core.Element {
	mut b := ui.div().test_id(id).w(32).h_full().center().shrink_0().cursor(.pointer).text_color(t.p.text).hover(ui.Patch{
		background: t.p.line.alpha(0.4)
	}).active(ui.Patch{
		background: t.p.line.alpha(0.6)
	}).role(.button).label(if dir < 0 { 'Decrement' } else { 'Increment' }).on_click(core.listener_with[NumberInput, f64, core.ClickEvent](entity,
		dir, fn (mut n NumberInput, dir f64, _ &core.ClickEvent, mut cx core.ViewCx[NumberInput]) {
		n.bump(dir, mut cx)
	})).child(kit.icon(if dir < 0 { kit.icon_minus } else { kit.icon_plus }).size(16))
	b.visual.radii = radii
	return b
}

// stepper is one of the up and down buttons.
fn stepper(entity u32, t &kit.Theme, dir f64, glyph string, id string) core.Element {
	return ui.div().test_id(id).size(16).center().rounded(t.radius_sm).cursor(.pointer).text_color(t.p.text_2).hover(ui.Patch{
		background: t.p.panel_hover
	}).on_click(core.listener_with[NumberInput, f64, core.ClickEvent](entity, dir, fn (mut n NumberInput, dir f64, _ &core.ClickEvent, mut cx core.ViewCx[NumberInput]) {
		n.bump(dir, mut cx)
	})).child(kit.icon(glyph).size(12))
}

// The Color field: eight swatches and a "Custom..." popover with the full grid (specs/26).
module sessionui

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.prim
import quadra.ui
import store

// The eight featured colors; the first is the session default (specs/26 decision 6).
const featured_colors = [store.default_color_hex, '#E06C75', '#E5C07B', '#98C379', '#61AFEF',
	'#C678DD', '#E8848F', '#83C2F4']

// palette_hex is the 13 x 10 grid behind the swatches: thirteen hues, ten lightness steps.
fn palette_hex() []string {
	mut out := []string{cap: 130}
	for h in 0 .. 13 {
		for l in 0 .. 10 {
			c := prim.hsla(f32(h) / 13 * 360, 0.7, 0.2 + f32(l) * 0.07, 1)
			out << c.hex().to_upper()
		}
	}
	return out
}

// ColorField is the row, the popover and the hex field of one dialog.
@[heap]
pub struct ColorField {
pub mut:
	value   string
	open    bool
	hex     core.View[input.TextInput]
	changed ?core.AnyListener // string: the new value
}

fn new_color_field(mut cx core.Cx, value string) core.View[ColorField] {
	v := cx.new_view(ColorField{
		value: value
	})
	cx.update_view(v, fn (mut f ColorField, mut cx core.ViewCx[ColorField]) {
		f.hex = input.new_input(mut cx.Cx, id: 'color-hex', value: f.value, placeholder: '#RRGGBB')
		input.on_submit(mut cx.Cx, f.hex, cx.listener(fn (mut f ColorField, v &string, mut cx core.ViewCx[ColorField]) {
			f.apply_hex(*v, mut cx)
		}))
	})
	return v
}

fn (mut f ColorField) pick(hex string, mut cx core.ViewCx[ColorField]) {
	f.value = hex
	f.open = false
	input.set_value(mut cx.Cx, f.hex, hex)
	cx.notify()
	kit.fire(f.changed, hex, mut cx.Cx)
}

// apply_hex takes a typed `#RRGGBB`; anything else stays in the field and changes nothing.
fn (mut f ColorField) apply_hex(s string, mut cx core.ViewCx[ColorField]) {
	v := s.trim_space()
	if prim.parse_hex(v) == none {
		return
	}
	f.pick(if v.starts_with('#') { v } else { '#${v}' }.to_upper(), mut cx)
}

fn swatch(t &kit.Theme, hex string, selected bool, size f32, ent u32) core.Element {
	color := prim.parse_hex(hex) or { t.p.accent }
	return ui.div().test_id('session-color-${hex}').size(size).shrink_0().rounded(t.radius_sm).bg(color).border(2).border_color(if selected {
		t.p.accent_text
	} else {
		color.alpha(0)
	}).cursor(.pointer).role(.button).label(hex).on_click(core.listener_with[ColorField, string, core.ClickEvent](ent,
		hex, fn (mut f ColorField, h string, _ &core.ClickEvent, mut cx core.ViewCx[ColorField]) {
		f.pick(h, mut cx)
	}))
}

@[markused]
pub fn (mut f ColorField) render(mut cx core.ViewCx[ColorField]) core.Element {
	t := kit.theme(cx)
	mut row := ui.div().flex_row().items_center().gap(8).wrap()
	for hex in featured_colors {
		row.child(swatch(t, hex, hex.to_lower() == f.value.to_lower(), 20, cx.entity))
	}
	row.child(kit.button('session-color-custom').label('Custom...').icon(kit.icon_palette).size(.sm).on_click(cx.listener(fn (mut f ColorField, _ &core.ClickEvent, mut cx core.ViewCx[ColorField]) {
		f.open = !f.open
		cx.notify()
	})))
	if !f.open {
		return row
	}
	mut grid := ui.div().flex_col().gap(2)
	hexes := palette_hex()
	for h in 0 .. 13 {
		mut line := ui.div().flex_row().gap(2)
		for l in 0 .. 10 {
			line.child(swatch(t, hexes[h * 10 + l], false, 16, cx.entity))
		}
		grid.child(line)
	}
	mut featured := ui.div().flex_row().gap(2).pb(4)
	for hex in featured_colors {
		featured.child(swatch(t, hex, false, 16, cx.entity))
	}
	box := ui.div().test_id('color-popover').flex_col().gap(6).p(8).bg(t.p.overlay_bg).rounded(t.radius_xl).border(1).border_color(t.p.line).shadow_level(1).on_action(cx.listener(fn (mut f ColorField, _ &kit.Dismiss, mut cx core.ViewCx[ColorField]) {
		f.open = false
		cx.notify()
	})).child(featured).child(grid).child(cx.child_view(f.hex))
	return ui.div().flex_col().child(row).child(ui.anchored(ui.div().size_full().on_mouse_down(cx.listener(fn (mut f ColorField, _ &core.MouseDownEvent, mut cx core.ViewCx[ColorField]) {
		f.open = false
		cx.notify()
	}))).fill_window().layer(5998)).child(ui.anchored(box).placement(.bottom_start).layer(6000))
}

// The counter example: one view with state, a styled button and a click listener.
module main

import quadra.core
import quadra.prim
import quadra.ui

pub struct Counter {
pub mut:
	count int
}

pub fn (mut c Counter) render(mut cx core.ViewCx[Counter]) core.Element {
	blue := prim.rgb(0x2563EB)
	return ui.div().size_full().center().bg(prim.rgb(0xF3F4F6)).child(ui.div().flex_col().gap(16).items_center().p(32).bg(prim.white).rounded(12).border(1).border_color(prim.rgb(0xE5E7EB)).shadow_level(2).child(ui.text('Count: ${c.count}').size(28).weight(600).color(prim.rgb(0x111827))).child(ui.div().test_id('inc').px(20).py(10).rounded(8).bg(blue).text_color(prim.white).font_weight(600).cursor(.pointer).hover(ui.Patch{
		background: prim.rgb(0x1D4ED8)
	}).active(ui.Patch{ background: prim.rgb(0x1E40AF) }).on_click(cx.listener(fn (mut c Counter, _ &core.ClickEvent, mut _ core.ViewCx[Counter]) {
		c.count++
	})).child(ui.text('Tăng +1'))))
}

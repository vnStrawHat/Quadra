// Chart pages: Sparkline, Line chart and Bar chart (kit/viz, drawn with quads only).
module pages

import math
import quadra.core
import quadra.kit
import quadra.kit.viz
import quadra.ui

fn wave(n int, phase f64, base f64, amp f64) []f64 {
	return []f64{len: n, init: base + amp * math.sin(f64(index) / 7 + phase) +
		amp * 0.3 * math.sin(f64(index) / 2.3 + phase * 2)}
}

fn sparkline_page(t &kit.Theme) core.Element {
	mut rows := []core.Element{}
	for i, name in ['api', 'worker', 'web', 'postgres'] {
		vals := wave(60, f64(i), 120, 60)
		rows << ui.div().flex_row().items_center().gap(16).child(ui.div().w(80).child(ui.text(name))).child(viz.sparkline(vals).size(160, 24).color(viz.series_color(t, i))).child(ui.text('${int(vals.last())}m').family(t.mono_family).color(t.p.text_2))
	}
	return page('Sparkline',
		'An inline trend: a 1.5 px line over an area at 12 %. Lines are drawn as one quad per pixel column, so live data never fills the atlas.', [
		demo('CPU per app', col(rows), 'viz.sparkline(values).size(160, 24)'),
	])
}

fn line_chart_page(t &kit.Theme) core.Element {
	start := i64(1793966400000)
	one := viz.line_chart('demo-line', [
		viz.Series{'cpu', wave(120, 0, 140, 50), none},
	]).time(start, 1000).unit('m').height(180)
	many := viz.line_chart('demo-lines', []viz.Series{len: 5, init: viz.Series{'pod-${index}', wave(120,
		f64(index) * 1.3, 60 + f64(index) * 40, 25), none}}).time(start, 1000).unit('m').height(220)
	return page('Line chart',
		'Up to 8 series on a time axis, a y axis rounded up to 1, 2, 2.5 or 5 times a power of ten, and a crosshair with the values under the mouse.', [
		demo('One series (with area)', framed(t, ui.div().p(12).child(one)),
			"viz.line_chart(id, [viz.Series{'cpu', values, none}]).time(start_ms, 1000)"),
		demo('Five series', framed(t, ui.div().p(12).child(many)),
			'series colors come from theme.series'),
	])
}

fn bar_chart_page(t &kit.Theme) core.Element {
	bars := ['api', 'worker', 'web', 'redis', 'postgres', 'prometheus'].map(viz.Bar{it, 40 +
		f64(it.len * 37 % 160), none})
	return page('Bar chart',
		'Labeled horizontal bars with their values, scaled to a rounded maximum.', [
		demo('CPU by app', framed(t,
			ui.div().p(12).child(viz.bar_chart('demo-bars', bars).unit('m'))),
			"viz.bar_chart(id, [viz.Bar{'api', 120, none}]).unit('m')"),
	])
}

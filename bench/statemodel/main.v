// State wiring at scale: change one row's colour among 3,000 live (not virtualized) rows and
// time the update plus the frame that settles it, p50 and p95. Coarse: one view owns every
// row. Fine: a view per row, only the changed row's view is notified. Equal write: the
// value written is the one the row has, and cx.update_if skips the notification.
// Run: tools/v.sh -cc clang -prod run bench/statemodel
module main

import os
import time
import quadra.core
import quadra.prim
import quadra.testing
import quadra.ui

const palette = [prim.rgb(0x2563EB), prim.rgb(0xDC2626), prim.rgb(0x16A34A)]

struct Coarse {
mut:
	labels []string
	colors []prim.Color
}

@[markused]
pub fn (mut c Coarse) render(mut cx core.ViewCx[Coarse]) core.Element {
	mut list := ui.div().flex_col().size_full().overflow_y_scroll()
	for i in 0 .. c.labels.len {
		list.child(ui.div().h(24).px(8).bg(c.colors[i]).child(ui.text(c.labels[i])))
	}
	return list
}

struct RowView {
mut:
	label string
	color prim.Color
}

@[markused]
pub fn (mut r RowView) render(mut cx core.ViewCx[RowView]) core.Element {
	return ui.div().h(24).px(8).bg(r.color).child(ui.text(r.label))
}

struct Fine {
mut:
	rows []core.View[RowView]
}

@[markused]
pub fn (mut f Fine) render(mut cx core.ViewCx[Fine]) core.Element {
	mut list := ui.div().flex_col().size_full().overflow_y_scroll()
	for v in f.rows {
		list.child(cx.child_view(v))
	}
	return list
}

fn set_coarse(mut c Coarse, k int, mut cx core.ViewCx[Coarse]) {
	c.colors[k % c.colors.len] = palette[k % palette.len]
	cx.notify()
}

fn set_row(mut r RowView, k int, mut cx core.ViewCx[RowView]) {
	r.color = palette[k % palette.len]
	cx.notify()
}

fn report(name string, mut us []i64) {
	us.sort()
	println('${name:-34s} p50 ${f64(us[us.len / 2]) / 1000:7.2f} ms   p95 ${f64(us[us.len * 95 / 100]) / 1000:7.2f} ms')
}

fn labels(rows int) []string {
	mut out := []string{cap: rows}
	for i in 0 .. rows {
		out << 'Row ${i}'
	}
	return out
}

// equal_row writes the value the row already has; update_if skips the notification.
fn equal_row(mut r RowView, _ int, mut _ core.ViewCx[RowView]) bool {
	return core.assign(mut r.color, r.color)
}

fn run(rows int) {
	n := 200
	mut h := testing.new_harness(width: 1200, height: 800) or { panic(err) }
	v := h.open(Coarse{
		labels: labels(rows)
		colors: []prim.Color{len: rows, init: palette[0]}
	},
		width:  1200
		height: 800
	) or { panic(err) }
	mut cx := core.Cx{
		app: h.app
	}
	mut us := []i64{}
	for k in 0 .. n + 10 {
		sw := time.new_stopwatch()
		cx.update_with(v.model(), k * 7, set_coarse)
		h.frame()
		if k >= 10 {
			us << sw.elapsed().microseconds()
		}
	}
	w1 := h.window()
	report('${rows} rows, coarse (one view)', mut us)
	println('    phases us: render ${w1.stats.phase_us[0]}, layout ${w1.stats.phase_us[1]}, prepaint ${w1.stats.phase_us[2]}, paint ${w1.stats.phase_us[3]}; incremental builds ${w1.stats.relayouts}, nodes relaid ${w1.stats.relaid}, layout evaluations ${w1.tree.measure_calls}')
	mut h2 := testing.new_harness(width: 1200, height: 800) or { panic(err) }
	mut cx2 := core.Cx{
		app: h2.app
	}
	mut views := []core.View[RowView]{cap: rows}
	for l in labels(rows) {
		views << cx2.new_view(RowView{ label: l, color: palette[0] })
	}
	h2.open(Fine{ rows: views },
		width:  1200
		height: 800
	) or { panic(err) }
	for pass in 0 .. 2 {
		us = []i64{}
		for k in 0 .. n + 10 {
			sw := time.new_stopwatch()
			if pass == 0 {
				cx2.update_with(views[(k * 7) % rows].model(), k, set_row)
			} else {
				cx2.update_if(views[(k * 7) % rows].model(), k, equal_row)
			}
			h2.frame()
			if k >= 10 {
				us << sw.elapsed().microseconds()
			}
		}
		w := h2.window()
		report(if pass == 0 {
			'${rows} rows, fine (view per row)'
		} else {
			'${rows} rows, fine, equal write'
		}, mut us)
		println('    phases us: render ${w.stats.phase_us[0]}, layout ${w.stats.phase_us[1]}, prepaint ${w.stats.phase_us[2]}, paint ${w.stats.phase_us[3]}; incremental builds ${w.stats.relayouts}, nodes relaid ${w.stats.relaid}, layout evaluations ${w.tree.measure_calls}')
	}
}

fn main() {
	sizes := if os.args.len > 1 { os.args[1..].map(it.int()) } else { [1000, 3000, 10000] }
	for rows in sizes {
		run(rows)
	}
}

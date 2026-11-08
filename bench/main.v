// Headless benchmarks (run with -prod): layout, text, full frames, hover and scrolling.
// Prints one line per benchmark: name, time per op, ops.
module main

import time
import quadra.core
import quadra.prim
import quadra.layout
import quadra.testing
import quadra.text
import quadra.text.fake
import quadra.ui

fn bench(name string, ops int, f fn ()) {
	f() // warm up
	sw := time.new_stopwatch()
	for _ in 0 .. ops {
		f()
	}
	per := f64(sw.elapsed().microseconds()) / f64(ops)
	println('${name:-44s} ${per:10.1f} us/op  (${ops} ops)')
}

fn layout_flex_10k() {
	mut t := layout.Tree{}
	mut rows := []layout.NodeId{}
	for _ in 0 .. 100 {
		mut cells := []layout.NodeId{}
		for _ in 0 .. 99 {
			cells << t.node(layout.Style{
				grow:    1
				height:  layout.px(20)
				padding: prim.edges(2)
			}, [])
		}
		rows << t.node(layout.Style{
			gap_col: 4
		}, cells)
	}
	root := t.node(layout.Style{
		direction: .column
		gap_row:   2
	}, rows)
	t.compute(root, prim.size(1920, 1080))
}

fn layout_grid_10k() {
	mut t := layout.Tree{}
	mut cells := []layout.NodeId{}
	for _ in 0 .. 10000 {
		cells << t.node(layout.Style{
			height: layout.px(20)
		}, [])
	}
	root := t.node(layout.Style{
		display: .grid
		gap_col: 4
		grid:    &layout.GridStyle{
			columns: layout.repeat(50, layout.fr(1))
		}
	}, cells)
	t.compute(root, prim.size(1920, 1080))
}

struct Grid {}

@[markused]
pub fn (mut g Grid) render(mut cx core.ViewCx[Grid]) core.Element {
	mut d := ui.div().grid_fill(90).gap(4).p(8)
	for i in 0 .. 1000 {
		d.child(ui.div().test_id('b${i}').px(8).py(4).rounded(4).bg(prim.rgb(0x2563EB)).cursor(.pointer).hover(ui.Patch{
			background: prim.rgb(0x1D4ED8)
		}).text_color(prim.white).child(ui.text('Button ${i}')))
	}
	return d
}

struct Rows {}

@[markused]
pub fn (mut r Rows) render(mut cx core.ViewCx[Rows]) core.Element {
	mut list := ui.div().flex_col().size_full().overflow_y_scroll()
	for i in 0 .. 2000 {
		list.child(ui.div().h(24).px(8).child(ui.text('Row ${i}')))
	}
	return list
}

fn main() {
	bench('layout: flex, 10k nodes', 50, layout_flex_10k)
	bench('layout: grid, 10k cells (50 fr columns)', 50, layout_grid_10k)
	mut ts := text.new_system(fake.new())
	para :=
		'Tiếng Việt có dấu được ngắt dòng theo từ, không cắt giữa chữ. '.repeat(1600) // ~100 KB
	st := text.TextStyle{
		size: 14
	}
	bench('text: wrap 100 KB, cold caches', 5, fn [mut ts, para, st] () {
		ts.end_frame()
		ts.end_frame()
		ts.wrap(para, st, 600)
	})
	bench('text: wrap 100 KB, warm word cache', 20, fn [mut ts, para, st] () {
		ts.wrap(para + ' ', st, 601)
	})
	mut h := testing.new_harness(width: 1920, height: 1080) or { panic(err) }
	h.open(Grid{}, width: 1920, height: 1080) or { panic(err) }
	mut w := h.window()
	bench('frame: full build, 1000-button grid', 20, fn [mut w] () {
		w.invalidate()
		w.draw_frame()
	})
	ph := w.stats.phase_us
	println('  full build phases (us): render ${ph[0]}, layout ${ph[1]}, prepaint ${ph[2]}, paint ${ph[3]}, layout evaluations ${w.tree.measure_calls}')
	mut i := 0
	bench('frame: hover change on 1000-button grid', 200, fn [mut h, mut i] () {
		r := h.find('b${i % 1000}') or { panic('') }
		h.move_to(r.x + 2, r.y + 2)
		i += 7
	})
	mut h2 := testing.new_harness(width: 800, height: 600) or { panic(err) }
	h2.open(Rows{}, width: 800, height: 600) or { panic(err) }
	bench('frame: scroll a 2000-row list', 200, fn [mut h2] () {
		h2.scroll_at(100, 100, -24)
	})
	gallery_benches()
	println('hover benchmark: builds ${h.window().stats.builds}, full paints ${h.window().stats.paints}, patched ${h.window().stats.patches}')
}

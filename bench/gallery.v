// Gallery scenarios of the spec 12 budgets: first render, resize, typing, big table scroll.
module main

import apps.gallery.pages
import quadra.kit
import quadra.testing

@[heap]
struct Sum {
mut:
	v i64
}

fn gallery_harness() &testing.Harness {
	mut h := testing.new_harness(width: 1200, height: 800) or { panic(err) }
	kit.install(mut h.app) or { panic(err) }
	h.open(pages.Gallery{}, width: 1200, height: 800) or { panic(err) }
	return h
}

fn open_page(mut h testing.Harness, id string) {
	for k in 0 .. 16 { // down first, then back up
		if h.find('nav-${id}') != none {
			break
		}
		h.scroll_at(100, 400, if k < 8 { -400 } else { 400 })
	}
	h.click('nav-${id}') or { panic(err) }
}

fn gallery_benches() {
	bench('gallery: first render (new app + window)', 10, fn () {
		gallery_harness()
	})
	mut h := gallery_harness()
	mut k := 0
	bench('gallery: window resize, full relayout', 50, fn [mut h, mut k] () {
		k++
		h.resize(1200 - f32(k % 2) * 40, 800)
	})
	open_page(mut h, 'text-input')
	h.click('in-name') or { panic(err) }
	mut acc := &Sum{}
	bench('gallery: typing a character in an input', 200, fn [mut h, mut acc] () {
		h.type_text('a')
		acc.v += h.window().stats.last_frame_us
	})
	println('  of which the frame (build + draw): ${f64(acc.v) / 201:.1f} us')
	ph := h.window().stats.phase_us
	println('  typing build phases (us): render ${ph[0]}, layout ${ph[1]}, prepaint ${ph[2]}, paint ${ph[3]}; layout nodes ${h.window().stats.layout_nodes}, layout evaluations ${h.window().tree.measure_calls}')
	open_page(mut h, 'table')
	r := h.find('dt') or { panic('no table') }
	bench('gallery: scroll the 1,000,000-row table', 200, fn [mut h, r] () {
		h.scroll_at(r.x + 100, r.y + 200, -32)
	})
	w := h.window()
	println('gallery data page: ${w.stats.draw_calls} draw calls, ${w.stats.instances} instances, ${w.stats.layout_nodes} layout nodes')
}

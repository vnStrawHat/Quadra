// Variable-height virtual list (ui/vlist.v): rows take their content's height, scrolling
// up into unmeasured rows keeps the screen still, reveal reaches unmeasured rows.
module main

import quadra.core
import quadra.prim
import quadra.testing
import quadra.ui

struct Feed {
mut:
	reveal int = -1
	set_y  f32 = -1
}

fn row_h(i int) f32 {
	return 20 + f32(i % 7) * 13 // never the estimate (10)
}

@[markused]
pub fn (mut f Feed) render(mut _ core.ViewCx[Feed]) core.Element {
	return ui.list('feed', 10_000, fn (i int) core.Element {
		return ui.div().test_id('row-${i}').w_full().h(row_h(i)).bg(prim.rgb(0xE5E7EB))
	}).estimate(10).reveal(f.reveal).scroll_y(f.set_y)
}

fn top(h &testing.Harness, i int) f32 {
	r := h.find('row-${i}') or { panic('row-${i} not on screen') }
	return r.y
}

fn set(mut h testing.Harness, v core.View[Feed], reveal int, y f32) {
	mut cx := core.Cx{
		app: h.app
	}
	cx.update(v.model(), fn [reveal, y] (mut f Feed, mut _ core.ViewCx[Feed]) {
		f.reveal, f.set_y = reveal, y
	})
	h.frame()
}

fn test_rows_take_their_content_height() {
	mut h := testing.new_harness(width: 400, height: 300)!
	h.open(Feed{}, width: 400, height: 300)!
	assert top(h, 0) == 0 // taller than the estimate, still shown from its top
	for i in 0 .. 5 {
		r := h.find('row-${i}') or { panic('missing') }
		assert r.height == row_h(i)
		assert r.y == top(h, 0) + f32(0) + sum_h(0, i)
	}
	assert h.find('row-40') == none
}

fn sum_h(a int, b int) f32 {
	mut s := f32(0)
	for i in a .. b {
		s += row_h(i)
	}
	return s
}

fn test_scrolling_up_into_unmeasured_rows_keeps_the_screen_still() {
	mut h := testing.new_harness(width: 400, height: 300)!
	v := h.open(Feed{}, width: 400, height: 300)!
	set(mut h, v, -1, 100_000) // far down: everything above is an estimate
	set(mut h, v, -1, -1)
	mut on := -1
	for i in 0 .. 10_000 {
		if r := h.find('row-${i}') {
			if r.y > 40 {
				on = i
				break
			}
		}
	}
	assert on > 0
	for _ in 0 .. 20 { // each step reveals rows measured for the first time
		y0 := top(h, on)
		h.scroll_at(200, 150, 9)
		assert top(h, on) == y0 + 9
	}
}

fn test_reveal_reaches_an_unmeasured_row() {
	mut h := testing.new_harness(width: 400, height: 300)!
	v := h.open(Feed{}, width: 400, height: 300)!
	set(mut h, v, 7000, -1)
	r := h.find('row-7000') or { panic('reveal did not reach row 7000') }
	assert r.y >= 0 && r.y + r.height <= 300
}

module main

import quadra.testing

fn test_counter_click_increments_and_rerenders_once() {
	mut h := testing.new_harness()!
	v := h.open(Counter{})!
	assert h.has_text('Count: 0')
	renders := h.renders_of(v)
	h.click('inc')!
	assert h.has_text('Count: 1')
	assert h.renders_of(v) == renders + 1
	h.click('inc')!
	h.click('inc')!
	assert h.has_text('Count: 3')
}

fn test_idle_produces_no_frames() {
	mut h := testing.new_harness()!
	h.open(Counter{})!
	builds := h.builds()
	h.advance(5000) // five idle seconds of virtual time
	assert h.builds() == builds
	assert h.plat.timer_count() == 0
}

fn test_hover_sets_cursor_and_restyles() {
	mut h := testing.new_harness()!
	h.open(Counter{})!
	r := h.find('inc') or { panic('button missing') }
	before := h.snapshot()
	h.move_to(r.x + 5, r.y + 5)
	assert h.cursor() == .pointer
	assert h.snapshot() != before // hover background differs
	h.move_to(1, 1)
	assert h.cursor() == .arrow
	assert h.snapshot() == before
}

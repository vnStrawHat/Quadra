// Popovers (click, outside click, escape) and hover cards (delay, grace, staying open).
module main

import quadra.core
import quadra.kit
import quadra.testing
import quadra.ui

struct Pops {}

@[markused]
pub fn (mut p Pops) render(mut cx core.ViewCx[Pops]) core.Element {
	return ui.div().size_full().flex_col().gap(40).p(20).child(kit.popover_button('pop', 'Columns', fn () core.Element {
		return ui.text('Visible columns')
	})).child(kit.hover_card(ui.div().test_id('trigger').w(120).h(20).child(ui.text('api-7d9f')), fn () core.Element {
		return ui.div().test_id('card').w(200).h(60).child(ui.text('Running on node-2'))
	}))
}

fn harness() !&testing.Harness {
	mut h := testing.new_harness(width: 800, height: 600)!
	kit.install(mut h.app)!
	h.open(Pops{}, width: 800, height: 600)!
	return h
}

fn test_popover_opens_on_click_and_closes_on_escape_or_outside_click() {
	mut h := harness()!
	assert !h.has_text('Visible columns')
	h.click('pop')!
	assert h.has_text('Visible columns')
	h.press('escape')!
	assert !h.has_text('Visible columns')
	h.click('pop')!
	assert h.has_text('Visible columns')
	h.click_at(700, 500)
	assert !h.has_text('Visible columns')
}

fn center(mut h testing.Harness, id string) (f32, f32) {
	r := h.find(id) or { panic('${id} missing') }
	return r.x + r.width / 2, r.y + r.height / 2
}

fn test_hover_card_opens_after_the_delay_and_stays_while_hovered() {
	mut h := harness()!
	x, y := center(mut h, 'trigger')
	h.move_to(x, y)
	h.advance(200)
	assert !h.has_text('Running on node-2')
	h.advance(300)
	assert h.has_text('Running on node-2')
	// Moving onto the card keeps it open, across the gap.
	cx, cy := center(mut h, 'card')
	h.move_to(cx, cy)
	h.advance(500)
	assert h.has_text('Running on node-2')
	// Leaving both closes it after the grace period.
	h.move_to(700, 500)
	h.advance(50)
	assert h.has_text('Running on node-2')
	h.advance(200)
	assert !h.has_text('Running on node-2')
}

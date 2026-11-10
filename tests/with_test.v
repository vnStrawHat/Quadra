// Closure-free callback APIs (core/with.v) and the closure leak detector (core/leakcheck.v).
module main

import quadra.core
import quadra.core.life
import quadra.testing
import quadra.ui

struct Ticker {
mut:
	log   []string
	other core.Model[int]
}

struct Step {
	word string
	n    int
}

@[markused]
pub fn (mut t Ticker) render(mut cx core.ViewCx[Ticker]) core.Element {
	return ui.div().size(40).test_id('tick').on_click(cx.listener(fn (mut t Ticker, _ &core.ClickEvent, mut cx core.ViewCx[Ticker]) {
		core.set_timeout_with(mut cx, 10, Step{'timeout', 1}, add_step)
		core.debounce_with(mut cx, 'k', 10, Step{'first', 1}, add_step)
		core.debounce_with(mut cx, 'k', 10, Step{'debounced', 2}, add_step)
		cx.app.after_with(10, Step{'after', 3}, fn (s Step, mut app core.App) {
			app.quit() // headless: a no-op; reaching here is the point
		})
		cx.update_view_with(cx.handle(), Step{'update', 4}, fn (mut t Ticker, s Step, mut _ core.ViewCx[Ticker]) {
			t.log << '${s.word}:${s.n}'
		})
	}))
}

fn add_step(mut t Ticker, s Step, mut _ core.ViewCx[Ticker]) {
	t.log << '${s.word}:${s.n}'
}

fn test_with_variants_carry_their_data_and_make_no_closure() {
	mut h := testing.new_harness(width: 100, height: 100)!
	v := h.open(Ticker{}, width: 100, height: 100)!
	h.click('tick')! // warm up
	h.advance(20)
	c0 := life.live_closures()
	h.click('tick')!
	h.advance(20)
	mut cx := core.Cx{
		app: h.app
	}
	t := cx.peek(v.model())
	assert t.log == ['update:4', 'timeout:1', 'debounced:2', 'update:4', 'timeout:1', 'debounced:2']
	assert life.live_closures() == c0
	assert h.app.closure_leaks() == []
}

struct Leaky {
mut:
	n int
}

@[markused]
pub fn (mut l Leaky) render(mut cx core.ViewCx[Leaky]) core.Element {
	return ui.div().size(40).test_id('leak').on_click(cx.listener(fn (mut l Leaky, _ &core.ClickEvent, mut cx core.ViewCx[Leaky]) {
		k := l.n
		cx.set_timeout(1000, fn [k] (mut l Leaky, mut _ core.ViewCx[Leaky]) {
			l.n = k
		})
	}))
}

fn test_the_detector_names_a_dispatch_that_makes_a_closure() {
	mut h := testing.new_harness(width: 100, height: 100)!
	h.open(Leaky{}, width: 100, height: 100)!
	h.click('leak')!
	$if prod {
		assert h.app.closure_leaks() == []
	} $else {
		assert h.app.closure_leaks() == ['handler of Leaky']
	}
}

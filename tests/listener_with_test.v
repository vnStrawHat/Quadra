// Closure-free listeners (core.handler_with, core.listener_with): data rides in the holder.
module main

import quadra.core
import quadra.testing
import quadra.ui

@[heap]
struct Acc {
mut:
	v int
}

struct Spec {
	acc  &Acc
	step int
}

struct Counter {
mut:
	total int
	acc   &Acc = &Acc{}
}

@[markused]
pub fn (mut c Counter) render(mut cx core.ViewCx[Counter]) core.Element {
	mut row := ui.div().flex_row()
	for i in 0 .. 3 {
		row.child(ui.div().test_id('add-${i}').size(20).on_click(core.listener_with(cx.entity,
			i + 1, fn (mut c Counter, n int, _ &core.ClickEvent, mut _ core.ViewCx[Counter]) {
			c.total += n
		})))
	}
	row.child(ui.div().test_id('named').size(20).on_click(core.handler_with(Spec{c.acc, 5}, fn (s Spec, _ &core.ClickEvent, mut _ core.Cx) {
		mut a := s.acc
		a.v += s.step
	})))
	return row
}

fn test_listeners_carry_their_data() {
	mut h := testing.new_harness(width: 200, height: 100)!
	v := h.open(Counter{}, width: 200, height: 100)!
	h.click('add-0')!
	h.click('add-2')!
	h.click('add-2')!
	h.click('named')!
	mut cx := core.Cx{
		app: h.app
	}
	c := cx.peek(v.model())
	assert c.total == 7 && c.acc.v == 5
}

struct Greeting {
	word string
}

fn test_bound_runs_with_its_data() {
	b := core.bind_fn(Greeting{'hi'}, fn (g Greeting, n int) string {
		return '${g.word} ${n}'
	})
	assert b.is_set() && b.run(3) == 'hi 3'
	empty := core.Bound[int, string]{}
	assert !empty.is_set()
}

struct Box {
	n int
}

fn test_bound_ref_takes_a_pointer_argument() {
	b := core.bind_fn_ref('total', fn (label string, x &Box) string {
		return '${label} ${x.n}'
	})
	assert b.run(&Box{4}) == 'total 4'
}

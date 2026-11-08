// Memory: closures made while rendering, building and prepainting are freed once their
// elements are gone; closures that escape a render stay valid; prepaint layouts don't pile up.
module main

import quadra.core
import quadra.core.life
import quadra.kit
import quadra.platform
import quadra.testing
import quadra.ui

struct Counter {
mut:
	n     int
	label string = 'count'
	// set during a render on purpose: an escape the framework must keep alive
	keep   core.Model[Kept]
	pinned bool
}

struct Kept {
	f fn () string = unsafe { nil }
}

@[markused]
pub fn (mut c Counter) render(mut cx core.ViewCx[Counter]) core.Element {
	label, n := c.label, c.n
	if c.pinned && c.keep.id == 0 {
		// A closure made in render, stored in a model created in render.
		c.keep = cx.new_model(Kept{
			f: fn [label] () string {
				return 'kept ${label}'
			}
		})
	}
	// Per-render closures (captured values) and kit components (closures per build).
	return ui.div().flex_col().child(kit.heading('${label} ${n}', 2)).child(kit.button('inc').label('+1').on_click(core.handler(fn [n] (_ &core.ClickEvent, mut _ core.Cx) {
		_ = n
	}))).child(ui.div().test_id('rows').h(200).child(ui.uniform_list('rows', 1000, 20, fn [label] (i int) core.Element {
		return ui.div().h(20).child(ui.text('${label} ${i}')).on_click(core.handler(fn [i] (_ &core.ClickEvent, mut _ core.Cx) {
			_ = i
		}))
	})))
}

fn bump(mut h testing.Harness, v core.View[Counter]) {
	mut cx := core.Cx{
		app: h.app
	}
	cx.update(v.model(), fn (mut c Counter, mut _ core.ViewCx[Counter]) {
		c.n++
	})
	h.frame()
}

fn test_rerenders_do_not_accumulate_closures() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(Counter{})!
	for _ in 0 .. 5 {
		bump(mut h, v)
	}
	base := life.live_closures()
	for _ in 0 .. 50 {
		bump(mut h, v)
	}
	assert life.live_closures() <= base + 2, 'closures grew from ${base} to ${life.live_closures()}'
	assert h.has_text('count 55')
}

fn test_scrolling_frees_rows_and_scratch_layout() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	h.open(Counter{})!
	r := h.find('rows') or { panic('no list') }
	h.scroll_at(r.x + 10, r.y + 10, -100)
	base := life.live_closures()
	nodes := h.window().tree.len()
	for _ in 0 .. 40 {
		h.scroll_at(r.x + 10, r.y + 10, -100)
	}
	assert life.live_closures() <= base + 2
	assert h.window().tree.len() <= nodes + 2, 'layout tree grew while scrolling'
}

fn test_closure_escaping_a_render_survives_rerenders() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(Counter{
		pinned: true
	})!
	for _ in 0 .. 10 {
		bump(mut h, v)
	}
	gc_collect()
	cx := core.Cx{
		app: h.app
	}
	c := cx.peek(v.model())
	assert cx.peek(c.keep).f() == 'kept count'
}

fn test_closed_window_frees_its_views() {
	mut h := testing.new_harness()!
	kit.install(mut h.app)!
	v := h.open(Counter{})!
	first := h.win
	models := h.app.model_count()
	h.open(Counter{})!
	assert h.app.model_count() == models + 2 // the window's Host and its root view
	h.win = first
	bump(mut h, v)
	bump(mut h, v) // opening a window pinned the first one's render once; settle it
	with_two := life.live_closures()
	h.win = h.app.windows.last().id
	h.inject(platform.WindowClosed{})
	h.win = first
	bump(mut h, v) // the first window builds: the closed window's views and builds are freed
	// The closed window held 13 closures (its Counter render: the kit has almost none, specs/14).
	assert life.live_closures() < with_two - 10, 'closed window kept its closures (${life.live_closures()} vs ${with_two})'
	assert h.app.model_count() == models, 'closing the window must release its views'
}

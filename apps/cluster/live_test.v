// Live updates in the harness: the interval ticks the fake cluster, the table keeps the
// selection by key across reorders and replacements, and no closures pile up.
module main

import quadra.core
import quadra.core.life
import quadra.kit
import quadra.testing

fn open_live() !(&testing.Harness, &Cluster) {
	mut h := testing.new_harness(width: 1280, height: 800)!
	kit.install(mut h.app)!
	v := h.open(Cluster{}, width: 1280, height: 800)!
	mut cx := core.Cx{
		app: h.app
	}
	return h, cx.peek(v.model())
}

fn test_ticks_keep_the_selection_by_key() {
	mut h, c := open_live()!
	h.click('resources-h4')! // by restarts: rows reorder as pods restart
	h.click('resources-row5')!
	key := c.selected
	assert key != ''
	t0 := c.data.t
	h.advance(1000)
	assert c.data.t == t0 + 1 // the interval drives the tick
	for _ in 0 .. 40 {
		h.advance(1000)
	}
	mut cx := core.Cx{
		app: h.app
	}
	tb := cx.peek(c.table.model())
	if key in c.rows.map(it.key()) {
		assert c.selected == key
		sel := tb.selection()
		assert sel.len == 1 && c.rows[sel[0]].key() == key
	} else {
		assert c.selected == '' && tb.selection().len == 0
	}
	assert h.has_text('Pods · 124') // replacements keep the count
	assert c.data.of('Pod').all(it.name.starts_with(it.app + '-7d9f8c6b5-')) // and the app
}

fn test_changed_cells_flash_and_closures_stay_flat() {
	mut h, c := open_live()!
	for _ in 0 .. 11 {
		h.advance(1000) // the 11th tick changes a pod's status
	}
	mut cx := core.Cx{
		app: h.app
	}
	tb := cx.peek(c.table.model())
	assert tb.flash.len > 0
	h.click('resources-row0')!
	h.advance(1000)
	before := life.live_closures()
	for _ in 0 .. 30 {
		h.advance(1000)
	}
	assert life.live_closures() == before
}

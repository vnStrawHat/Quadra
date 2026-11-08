// Drag and drop between elements (core/drag.v): threshold, preview, accepted kinds,
// drag-over style, escape, drop outside.
module main

import quadra.core
import quadra.prim
import quadra.platform
import quadra.testing
import quadra.ui

struct Board {
mut:
	todo   []string = ['write spec', 'ship']
	done   []string
	drops  int
	clicks int
}

fn card(id string, text string) &ui.Div {
	return ui.div().test_id(id).w(160).h(28).bg(prim.rgb(0xE5E7EB)).child(ui.text(text))
}

@[markused]
pub fn (mut b Board) render(mut cx core.ViewCx[Board]) core.Element {
	mut todo := ui.div().test_id('todo').flex_col().gap(4).w(200).h(300).p(8)
	for t in b.todo {
		todo.child(card('card-${t}', t).draggable('task', t, fn [t] () core.Element {
			return ui.div().test_id('preview').w(160).h(28).child(ui.text('moving ${t}'))
		}).on_click(cx.listener(fn (mut b Board, _ &core.ClickEvent, mut _ core.ViewCx[Board]) {
			b.clicks++
		})))
	}
	mut done := ui.div().test_id('done').flex_col().gap(4).w(200).h(300).p(8).drag_over(ui.Patch{
		background: prim.rgb(0xBFDBFE)
	}).on_drop('task', cx.listener(fn (mut b Board, e &core.DropEvent, mut _ core.ViewCx[Board]) {
		b.todo = b.todo.filter(it != e.payload.value)
		b.done << e.payload.value
		b.drops++
	}))
	for t in b.done {
		done.child(card('done-${t}', t))
	}
	// A target for another kind: never accepts tasks.
	other := ui.div().test_id('other').w(200).h(300).on_drop('file', cx.listener(fn (mut b Board, _ &core.DropEvent, mut _ core.ViewCx[Board]) {
		b.drops += 100
	}))
	return ui.div().size_full().flex_row().gap(20).child(todo).child(done).child(other)
}

fn mid(h &testing.Harness, id string) (f32, f32) {
	r := h.find(id) or { panic('${id} missing') }
	return r.x + r.width / 2, r.y + r.height / 2
}

fn board(h &testing.Harness, v core.View[Board]) &Board {
	mut cx := core.Cx{
		app: h.app
	}
	return cx.peek(v.model())
}

fn test_dropping_a_card_on_the_accepting_target_moves_it() {
	mut h := testing.new_harness(width: 800, height: 400)!
	v := h.open(Board{}, width: 800, height: 400)!
	x0, y0 := mid(h, 'card-ship')
	x1, y1 := mid(h, 'done')
	h.drag(x0, y0, x1, y1, true)
	b := board(h, v)
	assert b.drops == 1 && b.done == ['ship'] && b.todo == ['write spec']
	assert b.clicks == 0, 'a drag is not a click'
	assert h.find('done-ship') != none
}

fn test_preview_follows_without_hitboxes_and_target_restyles() {
	mut h := testing.new_harness(width: 800, height: 400)!
	h.open(Board{}, width: 800, height: 400)!
	before := h.snapshot()
	x0, y0 := mid(h, 'card-ship')
	x1, y1 := mid(h, 'done')
	h.drag(x0, y0, x1, y1, false)
	assert h.has_text('moving ship') == false, 'the preview is painted, not prepainted'
	assert h.find('preview') == none, 'the preview has no hitboxes'
	assert h.snapshot() != before // drag-over style and the preview
	h.press('escape')! // cancels: the release drops nothing
	h.inject(platform.MouseEvent{
		action: .up
		button: .left
		x:      x1
		y:      y1
	})
	assert h.snapshot() == before
}

fn test_small_moves_click_and_other_kinds_do_not_accept() {
	mut h := testing.new_harness(width: 800, height: 400)!
	v := h.open(Board{}, width: 800, height: 400)!
	x0, y0 := mid(h, 'card-ship')
	h.drag(x0, y0, x0 + 2, y0 + 1, true) // under the 4 px threshold: a click
	assert board(h, v).clicks == 1
	x1, y1 := mid(h, 'other')
	h.drag(x0, y0, x1, y1, true)
	assert board(h, v).drops == 0
	h.drag(x0, y0, 790, 390, true) // outside any target
	assert board(h, v).drops == 0 && board(h, v).todo.len == 2
}

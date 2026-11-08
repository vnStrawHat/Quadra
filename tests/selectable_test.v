// Selectable text: drag, double-click, select all and copy (headless clipboard only).
module main

import quadra.core
import quadra.platform
import quadra.testing
import quadra.ui

struct Doc {}

@[markused]
pub fn (mut d Doc) render(mut cx core.ViewCx[Doc]) core.Element {
	return ui.div().flex_col().size_full().child(ui.div().flex_row().child(ui.div().test_id('t').child(ui.text('alpha beta gamma').selectable()))).child(ui.div().test_id('away').h(40).w_full())
}

fn press_at(mut h testing.Harness, x f32, y f32, action platform.MouseAction) {
	h.inject(platform.MouseEvent{
		action: action
		button: .left
		x:      x
		y:      y
	})
}

fn copied(mut h testing.Harness) string {
	h.press('ctrl-c') or { panic(err) }
	return h.plat.clipboard_read() or { '' }
}

fn test_drag_double_click_and_select_all() {
	mut h := testing.new_harness()!
	h.open(Doc{}, width: 400, height: 200)!
	r := h.find('t') or { panic('no text') }
	ch := r.width / 16 // the fake backend gives every grapheme the same advance
	y := r.y + r.height / 2
	// Drag over "alpha".
	h.move_to(r.x + 1, y)
	press_at(mut h, r.x + 1, y, .down)
	h.move_to(r.x + ch * 5 + 1, y)
	press_at(mut h, r.x + ch * 5 + 1, y, .up)
	assert copied(mut h) == 'alpha'
	// Double-click selects the word under the mouse.
	h.advance(1000)
	h.click_at(r.x + ch * 7.5, y)
	h.click_at(r.x + ch * 7.5, y)
	assert copied(mut h) == 'beta'
	h.press('ctrl-a')!
	assert copied(mut h) == 'alpha beta gamma'
	// Clicking elsewhere drops the focus, so ctrl-c no longer copies.
	h.plat.clipboard_write('')
	h.click('away')!
	assert copied(mut h) == ''
}

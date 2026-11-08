// Text input behaviour on the headless harness.
module main

import quadra.core
import quadra.kit
import quadra.kit.input
import quadra.platform
import quadra.testing
import quadra.ui

struct Signup {
mut:
	name      core.View[input.TextInput]
	secret    core.View[input.TextInput]
	notes     core.View[input.TextInput]
	last      string
	submitted string
}

@[markused]
pub fn (mut s Signup) render(mut cx core.ViewCx[Signup]) core.Element {
	if s.name.id == 0 {
		s.name = input.new_input(mut cx.Cx, id: 'name', placeholder: 'Họ tên')
		s.secret = input.new_input(mut cx.Cx, id: 'secret', password: true)
		s.notes = input.textarea(mut cx.Cx, id: 'notes')
		input.on_change(mut cx.Cx, s.name, cx.listener(fn (mut s Signup, v &string, mut _ core.ViewCx[Signup]) {
			s.last = *v
		}))
		input.on_submit(mut cx.Cx, s.name, cx.listener(fn (mut s Signup, v &string, mut _ core.ViewCx[Signup]) {
			s.submitted = *v
		}))
	}
	return ui.div().flex_col().gap(8).p(8).w(400).child(cx.child_view(s.name)).child(cx.child_view(s.secret)).child(cx.child_view(s.notes))
}

fn setup() (&testing.Harness, &Signup) {
	mut h := testing.new_harness() or { panic(err) }
	kit.install(mut h.app) or { panic(err) }
	v := h.open(Signup{}) or { panic(err) }
	cx := core.Cx{
		app: h.app
	}
	return h, cx.peek(v.model())
}

fn value(h &testing.Harness, v core.View[input.TextInput]) string {
	cx := core.Cx{
		app: h.app
	}
	return cx.peek(v.model()).value()
}

fn test_typing_ime_selection_and_clipboard() {
	mut h, s := setup()
	h.click('name')!
	h.type_text('Xin ')
	h.ime_compose(['c', 'ch', 'chà'], 'chào') // Telex-style composition, committed
	assert value(h, s.name) == 'Xin chào'
	assert s.last == 'Xin chào' // the owner saw every change
	h.press('ctrl-shift-left')!
	h.press('ctrl-c')!
	assert h.plat.clipboard == 'chào'
	h.press('ctrl-x')!
	assert value(h, s.name) == 'Xin '
	h.press('ctrl-v')!
	h.press('ctrl-v')!
	assert value(h, s.name) == 'Xin chàochào'
	h.press('ctrl-z')! // undo the second paste
	assert value(h, s.name) == 'Xin chào'
	h.press('ctrl-y')!
	assert value(h, s.name) == 'Xin chàochào'
	h.press('ctrl-a')!
	h.press('backspace')!
	assert value(h, s.name) == ''
	h.type_text('An')
	h.press('enter')!
	assert s.submitted == 'An'
}

fn test_grapheme_backspace_and_word_delete() {
	mut h, s := setup()
	h.click('name')!
	h.type_text('Tiếng Việt')
	h.press('backspace')!
	assert value(h, s.name) == 'Tiếng Việ'
	h.press('backspace')! // "ệ" is one grapheme
	assert value(h, s.name) == 'Tiếng Vi'
	h.press('ctrl-backspace')!
	assert value(h, s.name) == 'Tiếng '
	h.press('home')!
	h.press('delete')!
	assert value(h, s.name) == 'iếng '
}

fn test_password_is_masked() {
	mut h, s := setup()
	h.click('secret')!
	h.type_text('mật khẩu')
	assert value(h, s.secret) == 'mật khẩu'
	assert !h.has_text('mật khẩu')
	assert h.has_text('••••••••')
	h.press('ctrl-a')!
	h.press('ctrl-c')!
	assert h.plat.clipboard == '' // never copied
}

fn test_textarea_lines_and_vertical_movement() {
	mut h, s := setup()
	h.click('notes')!
	h.type_text('dòng một')
	h.press('enter')!
	h.type_text('hai')
	assert value(h, s.notes) == 'dòng một\nhai'
	h.press('up')!
	h.type_text('!')
	assert value(h, s.notes) == 'dòn!g một\nhai' // caret kept its x (about 3 columns)
}

fn test_mouse_places_caret_and_double_click_selects_word() {
	mut h, s := setup()
	h.click('name')!
	h.type_text('alpha beta')
	r := h.find('name') or { panic('') }
	// 14 px text in the fake backend: 8.4 px per grapheme; padding 10, border 1.
	x := r.x + 11 + 8.4 * 2.2
	h.click_at(x, r.y + r.height / 2)
	h.type_text('X')
	assert value(h, s.name) == 'alXpha beta'
	bx := r.x + 11 + 8.4 * 8
	for _ in 0 .. 2 {
		h.inject(platform.MouseEvent{
			action: .down
			button: .left
			x:      bx
			y:      r.y + 10
		})
		h.inject(platform.MouseEvent{
			action: .up
			button: .left
			x:      bx
			y:      r.y + 10
		})
	}
	h.type_text('B')
	assert value(h, s.name) == 'alXpha B'
}

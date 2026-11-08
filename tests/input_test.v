// Focus, keymap/actions, text input, scrolling, overlays and animation.
module main

import quadra.core
import quadra.prim
import quadra.testing
import quadra.ui

struct Save {}

struct Undo {}

struct Form {
mut:
	a      core.FocusHandle
	b      core.FocusHandle
	typed  string
	saved  int
	undone int
	keys   []string
}

@[markused]
pub fn (mut f Form) render(mut cx core.ViewCx[Form]) core.Element {
	if f.a.id == 0 {
		f.a = cx.focus_handle()
		f.b = cx.focus_handle()
	}
	field := fn [mut cx] (h core.FocusHandle, id string) &ui.Div {
		return ui.div().test_id(id).track_focus(h).role(.textbox).label(id).w(100).h(24).key_context('Field').on_text_input(cx.listener(fn (mut f Form, e &core.TextInputEvent, mut _ core.ViewCx[Form]) {
			f.typed += e.text
		}))
	}
	return ui.div().flex_col().gap(8).on_action(cx.listener(fn (mut f Form, _ &Save, mut _ core.ViewCx[Form]) {
		f.saved++
	})).child(field(f.a, 'a').on_action(cx.listener(fn (mut f Form, _ &Undo, mut _ core.ViewCx[Form]) {
		f.undone++
	}))).child(field(f.b, 'b').on_key_down(cx.listener(fn (mut f Form, e &core.KeyDownEvent, mut _ core.ViewCx[Form]) {
		f.keys << e.key.str()
	})))
}

fn form(h &testing.Harness, v core.View[Form]) &Form {
	cx := core.Cx{
		app: h.app
	}
	return cx.peek(v.model())
}

fn test_tab_order_click_focus_and_text() {
	mut h := testing.new_harness()!
	v := h.open(Form{})!
	h.press('tab')!
	assert h.focused() == form(h, v).a.id
	assert (h.find_role(.textbox, 'a') or { panic('missing') }).focused
	h.press('tab')!
	assert h.focused() == form(h, v).b.id
	h.press('shift-tab')!
	assert h.focused() == form(h, v).a.id
	h.click('b')!
	assert h.focused() == form(h, v).b.id
	h.type_text('Việt')
	assert form(h, v).typed == 'Việt'
	h.press('x')!
	assert form(h, v).keys.last() == 'x' // b also saw the shift-tab that left it
}

fn test_actions_resolve_by_context_and_chords() {
	mut h := testing.new_harness()!
	h.app.bind('ctrl-s', Save{})!
	h.app.bind('ctrl-z', Undo{}, context: 'Field')!
	h.app.bind('ctrl-k ctrl-s', Save{})!
	v := h.open(Form{})!
	h.click('a')!
	h.press('ctrl-s')!
	assert form(h, v).saved == 1 // bubbles from the field to the form
	h.press('ctrl-z')!
	assert form(h, v).undone == 1
	h.press('ctrl-k ctrl-s')!
	assert form(h, v).saved == 2
	h.click('b')!
	h.press('ctrl-z')!
	assert form(h, v).undone == 1 // b has no Undo handler and Undo does not reach the form
	assert h.app.bindings_for[Save]() == ['ctrl-s', 'ctrl-k ctrl-s']
}

struct LongList {}

@[markused]
pub fn (mut l LongList) render(mut cx core.ViewCx[LongList]) core.Element {
	mut list := ui.div().test_id('list').flex_col().h(100).w(200).overflow_y_scroll()
	for i in 0 .. 20 {
		list.child(ui.div().test_id('row${i}').h(20).child(ui.text('row ${i}')))
	}
	return list
}

fn test_wheel_scrolls_and_clamps() {
	mut h := testing.new_harness()!
	v := h.open(LongList{})!
	renders := h.renders_of(v)
	assert (h.find('row4') or { panic('') }).y == 80
	assert h.find('row5') == none // below the visible area: culled, costs nothing
	h.scroll_at(50, 50, -60) // wheel down 60 px
	assert (h.find('row4') or { panic('') }).y == 20
	h.scroll_at(50, 50, -10000)
	assert (h.find('row19') or { panic('') }).y == 80 // last row ends at the bottom
	assert h.renders_of(v) == renders // scrolling never re-renders
	assert h.window().stats.prepaints >= 2
}

struct Popup {
mut:
	clicks_under int
	clicks_over  int
}

@[markused]
pub fn (mut p Popup) render(mut cx core.ViewCx[Popup]) core.Element {
	return ui.div().size_full().test_id('base').on_click(cx.listener(fn (mut p Popup, _ &core.ClickEvent, mut _ core.ViewCx[Popup]) {
		p.clicks_under++
	})).child(ui.anchored(ui.div().test_id('menu').w(120).h(80).bg(prim.white).on_click(cx.listener(fn (mut p Popup, _ &core.ClickEvent, mut _ core.ViewCx[Popup]) {
		p.clicks_over++
	}))).at(prim.rect(700, 560, 40, 20)))
}

fn test_overlay_is_on_top_and_stays_inside_the_window() {
	mut h := testing.new_harness()!
	v := h.open(Popup{})!
	r := h.find('menu') or { panic('menu missing') }
	assert r.right() <= 800 && r.bottom() <= 600 // flipped above / shifted left
	assert r.y < 560
	h.click('menu')!
	cx := core.Cx{
		app: h.app
	}
	p := cx.peek(v.model())
	assert p.clicks_over == 1
	assert p.clicks_under == 0 // overlays sit above: the content under them is not clicked
}

struct Fader {
mut:
	open bool
}

@[markused]
pub fn (mut f Fader) render(mut cx core.ViewCx[Fader]) core.Element {
	w := cx.animate('w', if f.open { f32(200) } else { 0 }, 200, .linear)
	return ui.div().test_id('panel').h(10).w(w).on_click(cx.listener(fn (mut f Fader, _ &core.ClickEvent, mut _ core.ViewCx[Fader]) {
		f.open = !f.open
	}))
}

fn test_animation_follows_the_frame_clock() {
	mut h := testing.new_harness()!
	h.open(Fader{
		open: true
	})!
	assert (h.find('panel') or { panic('') }).width == 200
	h.click('panel')! // close: animate 200 -> 0 over 200 ms
	h.advance(100)
	assert (h.find('panel') or { panic('') }).width == 100
	h.advance(150)
	assert (h.find('panel') or { panic('') }).width == 0
	builds := h.builds()
	h.advance(1000)
	assert h.builds() == builds // settled: no more frames
}

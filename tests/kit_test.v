// Kit components on the headless harness.
module main

import quadra.core
import quadra.kit
import quadra.platform
import quadra.testing
import quadra.ui

struct Panel {
mut:
	clicks  int
	checked bool
	radio   int
	value   f32 = 50
	tab     int
	open    []bool = [false, false]
	picked  string
}

@[markused]
pub fn (mut p Panel) render(mut cx core.ViewCx[Panel]) core.Element {
	items := [
		kit.MenuItem{
			label:     'Rename'
			on_select: fn (mut cx core.Cx) {
				kit.push_toast(mut cx, .success, 'Renamed')
			}
		},
		kit.separator(),
		kit.MenuItem{
			label:  'Delete'
			danger: true
		},
	]
	return ui.div().flex_col().gap(8).p(8).child(kit.button('go').label('Go').primary().on_click(cx.listener(fn (mut p Panel, _ &core.ClickEvent, mut _ core.ViewCx[Panel]) {
		p.clicks++
	}))).child(kit.checkbox('cb', p.checked).label('Agree').on_change(cx.listener(fn (mut p Panel, v &bool, mut _ core.ViewCx[Panel]) {
		p.checked = *v
	}))).child(kit.radio_group('rg', ['A', 'B', 'C'], p.radio).on_change(cx.listener(fn (mut p Panel, v &int, mut _ core.ViewCx[Panel]) {
		p.radio = *v
	}))).child(kit.slider('sl', p.value).width(208).on_change(cx.listener(fn (mut p Panel, v &f32, mut _ core.ViewCx[Panel]) {
		p.value = *v
	}))).child(kit.tabs('tb', ['One', 'Two'], p.tab).on_change(cx.listener(fn (mut p Panel, v &int, mut _ core.ViewCx[Panel]) {
		p.tab = *v
	}))).child(kit.accordion('ac', [kit.AccordionItem{'First', ui.text('first body')},
		kit.AccordionItem{'Second', ui.text('second body')}], p.open).on_toggle(cx.listener(fn (mut p Panel, i &int, mut _ core.ViewCx[Panel]) {
		p.open[*i] = !p.open[*i]
	}))).child(kit.menu_button('mb', 'File', items)).child(kit.button('tip').label('Hover me').tooltip('Helpful'))
}

fn panel(h &testing.Harness, v core.View[Panel]) &Panel {
	cx := core.Cx{
		app: h.app
	}
	return cx.peek(v.model())
}

fn setup() (&testing.Harness, core.View[Panel]) {
	mut h := testing.new_harness() or { panic(err) }
	kit.install(mut h.app) or { panic(err) }
	v := h.open(Panel{}) or { panic(err) }
	return h, v
}

fn test_buttons_toggles_and_choices() {
	mut h, v := setup()
	h.click('go')!
	assert panel(h, v).clicks == 1
	h.press('enter')! // the clicked button has focus
	assert panel(h, v).clicks == 2
	h.click('cb')!
	assert panel(h, v).checked
	assert (h.find_role(.checkbox, 'Agree') or { panic('') }).checked
	h.click('rg-2')!
	assert panel(h, v).radio == 2
	h.press('right')! // wraps around
	assert panel(h, v).radio == 0
}

fn test_slider_drag_and_keys() {
	mut h, v := setup()
	r := h.find('sl') or { panic('') }
	h.move_to(r.x + 8, r.y + 10)
	h.inject(platform.MouseEvent{
		action: .down
		button: .left
		x:      r.x + 8
		y:      r.y + 10
	})
	assert panel(h, v).value == 0
	h.inject(platform.MouseEvent{
		action: .move
		x:      r.x + 500 // dragged far outside: captured, clamped
		y:      r.y + 300
	})
	assert panel(h, v).value == 100
	h.inject(platform.MouseEvent{
		action: .up
		button: .left
		x:      r.x + 500
		y:      r.y + 300
	})
	h.press('left')!
	assert panel(h, v).value == 99
	h.press('home')!
	assert panel(h, v).value == 0
}

fn test_tabs_accordion() {
	mut h, v := setup()
	h.click('tb-1')!
	assert panel(h, v).tab == 1
	assert !h.has_text('first body')
	h.click('ac-0')!
	assert h.has_text('first body')
}

fn test_menu_toast_and_escape() {
	mut h, _ := setup()
	h.click('mb')!
	assert h.find('menu-0') != none
	h.press('down')! // highlight Rename
	h.press('enter')!
	assert h.find('menu-0') == none // closed
	assert h.find_role(.alert, 'Renamed') != none
	h.advance(10100)
	assert h.find_role(.alert, 'Renamed') == none // toasts expire after 10 s
	h.click('mb')!
	h.press('escape')!
	assert h.find('menu-0') == none
}

fn test_modal_backdrop_and_tooltip() {
	mut h, _ := setup()
	mut cx := core.Cx{
		app: h.app
		win: h.window()
	}
	id := kit.open_modal(mut cx, kit.ModalOptions{
		title: 'Confirm'
		body:  fn () core.Element {
			return ui.text('Are you sure?')
		}
	})
	h.frame()
	assert h.find('modal-${id}') != none
	h.click_at(5, 5) // backdrop
	assert h.find('modal-${id}') == none
	r := h.find('tip') or { panic('') }
	h.move_to(r.x + 4, r.y + 4)
	assert h.find_role(.tooltip, 'Helpful') == none
	h.advance(520)
	assert h.find_role(.tooltip, 'Helpful') != none
}

fn test_theme_switch_relayouts_without_rerender() {
	mut h, v := setup()
	renders := h.renders_of(v)
	kit.set_theme(mut h.app, kit.dark())
	h.frame()
	assert h.renders_of(v) == renders
	assert h.window().clear_color == kit.dark().colors.background
	kit.follow_system(mut h.app, kit.light(), kit.dark())
	h.plat.set_appearance(platform.Appearance{
		high_contrast: true
	})
	h.frame()
	assert kit.theme_of(h.app).mode == .high_contrast
}

fn test_key_label() {
	assert kit.key_label('ctrl-shift-p') == 'Ctrl-Shift-P'
	assert kit.key_label('shift-ctrl-p') == 'Ctrl-Shift-P'
	assert kit.key_label('ctrl-minus') == 'Ctrl--'
	assert kit.key_label('ctrl-=') == 'Ctrl-='
	assert kit.key_label('alt-f4') == 'Alt-F4'
	assert kit.key_label('shift-pageup') == 'Shift-PageUp'
	assert kit.key_label('ctrl-k ctrl-o') == 'Ctrl-K Ctrl-O'
}

// Accessibility glue (core/a11y.v): the semantic tree sent to the platform and actions
// coming back from assistive technology.
module main

import quadra.core
import quadra.platform
import quadra.testing
import quadra.ui

fn test_roles_match_the_platform_roles() {
	$for r in core.Role.values {
		assert '${r.value}' == '${unsafe { platform.A11yRole(int(r.value)) }}'
	}
}

struct Form {
mut:
	saves int
	on    bool
}

@[markused]
pub fn (mut f Form) render(mut cx core.ViewCx[Form]) core.Element {
	save := ui.div().w(80).h(30).role(.button).label('Save').on_click(cx.listener(fn (mut f Form, _ &core.ClickEvent, mut _ core.ViewCx[Form]) {
		f.saves++
	}))
	check := ui.div().w(80).h(30).role(.checkbox).label('Wrap').checked(f.on).on_click(cx.listener(fn (mut f Form, _ &core.ClickEvent, mut _ core.ViewCx[Form]) {
		f.on = !f.on
	}))
	return ui.div().size_full().p(20).child(ui.div().flex_col().gap(8).role(.group).label('Editor').child(save).child(check)).child(ui.div().role(.heading).label('Hidden'))
}

fn test_tree_is_sent_only_while_listening() {
	mut h := testing.new_harness(width: 300, height: 200)!
	h.open(Form{}, width: 300, height: 200)!
	assert h.plat.windows[0].a11y.len == 0
	h.plat.windows[0].a11y_on = true
	mut w := h.window()
	w.invalidate_prepaint()
	h.frame()
	nodes := h.plat.windows[0].a11y
	assert nodes.map(it.name) == ['Editor', 'Save', 'Wrap', 'Hidden']
	assert nodes.map(it.parent) == [-1, 0, 0, -1]
	assert nodes[2].role == .checkbox && !nodes[2].checked
	assert nodes[1].id != 0 && nodes[1].id != nodes[2].id
}

fn test_actions_click_the_node() {
	mut h := testing.new_harness(width: 300, height: 200)!
	v := h.open(Form{}, width: 300, height: 200)!
	h.plat.windows[0].a11y_on = true
	mut w := h.window()
	w.invalidate_prepaint()
	h.frame()
	nodes := h.plat.windows[0].a11y
	h.inject(platform.A11yEvent{nodes[1].id, .invoke})
	h.inject(platform.A11yEvent{nodes[2].id, .toggle})
	h.frame()
	mut cx := core.Cx{
		app: h.app
	}
	f := cx.peek(v.model())
	assert f.saves == 1
	assert f.on
	assert h.plat.windows[0].a11y[2].checked // the next tree shows the new state
}

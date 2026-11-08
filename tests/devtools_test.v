// Devtools overlay: profiler, paint flashing and inspector toggle with ctrl-shift keys.
module main

import quadra.core
import quadra.testing
import quadra.ui

struct Plain {}

@[markused]
pub fn (mut p Plain) render(mut cx core.ViewCx[Plain]) core.Element {
	return ui.div().size_full().child(ui.div().test_id('box').w(100).h(40).cursor(.pointer))
}

fn test_devtools_toggle_without_rerender() {
	mut h := testing.new_harness()!
	v := h.open(Plain{})!
	base := h.scene().instances.len
	renders := h.renders_of(v)
	h.press('ctrl-shift-p')!
	assert h.window().debug.profiler
	assert h.scene().instances.len > base // profiler panel painted
	h.press('ctrl-shift-p')!
	h.press('ctrl-shift-i')!
	h.move_to(10, 10)
	assert h.scene().instances.len > base // inspector highlight under the mouse
	assert h.renders_of(v) == renders // devtools never re-render app views
}

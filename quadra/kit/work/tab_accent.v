// Tabs with Controls.accent_chrome (specs/15): padded 12, the active tab with a 2 px
// accent line on top and seams on both sides, the close button always shown.
module work

import quadra.core
import quadra.prim
import quadra.layout
import quadra.kit
import quadra.ui

fn (s &TabStrip) tab_accent(i int, tab Tab, t &kit.Theme) core.Element {
	p := t.p
	on := tab.id == s.active
	fill := if on { p.canvas } else { p.tab_bg }
	mut content := ui.div().flex_row().items_center().gap(t.sp(4)).px(t.sp(12)).grow()
	if tab.dirty {
		content.child(ui.div().size(6).rounded(3).bg(p.accent))
	}
	if tab.glyph != '' {
		content.child(kit.icon(tab.glyph).size(14).color(p.text_2))
	}
	content.children(s.leads[tab.id])
	content.child(ui.text(tab.title).truncate())
	if s.close != none {
		content.child(ui.div().test_id('${s.id}-${tab.id}-close').size(t.sp(16)).shrink_0().center().rounded(3).hover(ui.Patch{
			background: p.text_2.alpha(0.15)
		}).child(kit.icon(kit.icon_x).size(12).color(p.text_2)).on_mouse_down(core.handler(fn (_ &core.MouseDownEvent, mut cx core.Cx) {
			cx.stop_propagation() // pressing close must not select or start a drag
		})).on_click(core.handler_with(StripTab{unsafe { s }, tab.id}, fn (st StripTab, _ &core.ClickEvent, mut cx core.Cx) {
			cx.stop_propagation()
			kit.fire(st.strip.close, st.id, mut cx)
		})))
	}
	if s.min_w > 0 {
		content.min_w(s.min_w).max_w(s.max_w)
	}
	mut d := ui.div().id('tab:${tab.id}').test_id('${s.id}-${tab.id}').flex_row().h_full().shrink_0().cursor(.pointer).border_color(p.line_strong).bg(fill).text_color(if on {
		p.tab_active_fg
	} else {
		p.tab_fg
	}).hover(ui.Patch{
		text_color: p.tab_active_fg
	}).role(.tab).label(tab.title).selected(on).child(content)
	if on {
		d.child(ui.div().absolute().top(0).left(0).w_full().h(2).bg(p.accent))
		if i > 0 {
			d.border_l(1)
		}
		d.border_r(1)
	} else {
		d.border_b(1)
	}
	return s.tab_events(d, tab)
}

// Probe records an element's bounds during prepaint (tab hit areas for dragging).
pub struct Probe {
mut:
	el  core.Element
	out &prim.Rect = unsafe { nil }
}

pub fn (mut p Probe) request_layout(mut cx core.LayoutCx) layout.NodeId {
	return p.el.request_layout(mut cx)
}

pub fn (mut p Probe) prepaint(bounds prim.Rect, mut cx core.PrepaintCx) {
	unsafe {
		*p.out = bounds
	}
	p.el.prepaint(bounds, mut cx)
}

pub fn (mut p Probe) paint(bounds prim.Rect, mut cx core.PaintCx) {
	p.el.paint(bounds, mut cx)
}
